/*
 * SPDX-License-Identifier: GPL-2.0 OR MIT
 *
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 * Copyright (C) 2006-2018 Frans Kaashoek, Robert Morris, Russ Cox,
 *                      Massachusetts Institute of Technology
 */

#[cfg(not(feature = "user"))]
use crate::fuse;
#[cfg(not(feature = "user"))]
use crate::libc;
#[cfg(not(feature = "user"))]
use crate::println;
#[cfg(not(feature = "user"))]
use crate::std;
#[cfg(not(feature = "user"))]
use crate::time;

use alloc::sync::Arc;
use alloc::vec::Vec;

use core::cmp::min;
use core::mem;
use core::str;
use core::sync::atomic::{AtomicUsize, Ordering};

use datablock::DataBlock;

use fuse::{FileAttr,FileType};

use crate::xv6fs_file::*;
use crate::xv6fs_ll::*;
use crate::xv6fs_utils::*;
use crate::xv6fs_log::*;

use std::os::unix::io::AsRawFd;
use std::ffi::OsStr;
use std::sync::*;

use time::Timespec;

static LAST_BLOCK: AtomicUsize = AtomicUsize::new(0);

impl Xv6FileSystem {
    // Read xv6 superblock from disk
    fn readsb(&mut self) -> Result<(), libc::c_int> {
        let sb = self.sb.as_mut().unwrap();
        let disk = self.disk.as_ref().unwrap();
        let bh = disk.bread(1)?;
        let b_slice = bh.data();
        sb.extract_from(&b_slice[0..mem::size_of::<Xv6fsSB>()]).map_err(|_| libc::EIO)?;
        return Ok(());
    }

    fn bzero(&self, bno: usize) -> Result<(), libc::c_int> {
        let log = self.log.as_ref().unwrap();
        let disk = self.disk.as_ref().unwrap();
        let mut bh = disk.bread(bno as u64)?;

        let b_slice = bh.data_mut();
        for byte in b_slice {
            *byte = 0;
        }

        bh.mark_buffer_dirty();
        log.log_write(bno as u32);

        return Ok(());
    }

    // Allocate a block on disk, using a slightly different alloc strategy from xv6.
    // xv6 scans from 0th block and allocates the first available block, we scan from the latest used block since last boot.
    fn balloc(&self) -> Result<u32, libc::c_int> {
        let log = self.log.as_ref().unwrap();
        let sb = self.sb.as_ref().unwrap();
        let fs_size = sb.size;
        let mut allocated_block = None;

        // Bitmap operations on bitmap blocks
        let most_recent = LAST_BLOCK.load(Ordering::SeqCst);
        let mut first = true;
        // last_segment is the bitmap block ID and block_offset is the offset for 'most_recent'
        let last_segment = most_recent - most_recent % BPB;
        let mut block_offset = most_recent % BPB;

        let mut b = last_segment;

        while first || b < last_segment {
            let disk = self.disk.as_ref().unwrap();
            // Read bitmap block that contains bitmap for b/last_segment, bitmap_slice contains the data
            let mut bh = disk.bread(bblock(b as usize, &sb) as u64)?;
            let bitmap_slice = bh.data_mut();

            let mut changed = false;

            // last allocated was block_offset, scan from it until end of block.
            for bi in block_offset..BPB {
                let _guard = self.balloc_lock.as_ref().unwrap().write();
                let curr_data_block = b as u32 + bi as u32; // 'b' is block id and 'bi' is offset
                if curr_data_block >= fs_size {
                    break;
                }

                let m = 1 << (bi % 8);
                let byte_data = bitmap_slice.get_mut(bi / 8).ok_or(libc::EIO)?;

                if *byte_data & m == 0 {
                    // found unallocated, bitmap bit is zero.
                    *byte_data |= m;
                    changed = true;
                    allocated_block = Some(curr_data_block);
                    break;
                }
            }

            // Write buffer
            if changed {
                bh.mark_buffer_dirty();
                log.log_write(bblock(b as usize, &sb) as u32);
            }
            // extract new block ID x
            if let Some(x) = allocated_block {
                LAST_BLOCK.store(x as usize, Ordering::SeqCst);
                self.bzero(x as usize)?;
                return Ok(x);
            }

            // did not find usable block in this bitmap block, go to the next bitmap block
            block_offset = 0;
            b += BPB;
            if b >= fs_size as usize {
                b = 0;
                first = false;
            }
        }
        return Err(libc::EIO);
    }

    fn bfree(&self, block_id: usize) -> Result<(), libc::c_int> {
        // Get block number
        let sb = self.sb.as_ref().unwrap();
        let block_num = bblock(block_id, &sb);
        let log = self.log.as_ref().unwrap();

        // Read block
        let disk = self.disk.as_ref().unwrap();
        let mut bh = disk.bread(block_num as u64)?;
        let b_slice = bh.data_mut();

        // Get bit id
        let bit_id = block_id % BPB;
        let byte_id = bit_id / 8;
        let bit_in_byte = bit_id % 8;

        // Clear the bit
        let maybe_mut_byte = b_slice.get_mut(byte_id);
        let mut_byte = maybe_mut_byte.ok_or(libc::EIO)?;

        *mut_byte &= !(1 << bit_in_byte);

        // Write buffer
        bh.mark_buffer_dirty();
        log.log_write(block_num as u32);

        return Ok(());
    }

    pub fn iinit(&mut self) {
        if self.readsb().is_err() {
            println!("Unable to read super block from disk.");
        }

        let log = Xv6Log::new(Arc::clone(self.disk.as_ref().unwrap()));
        self.log = Some(log);

        let mut inode_vec: Vec<RwLock<Inode>> = Vec::with_capacity(NINODE);
        for _ in 0..NINODE {
            inode_vec.push(RwLock::new(Inode::new()));
        }
        self.ilock_cache = Some(inode_vec);

        self.ialloc_lock = Some(RwLock::new(0));
        self.balloc_lock = Some(RwLock::new(0));

        let sb = self.sb.as_mut().unwrap();
        let log = self.log.as_ref().unwrap();
        let _ = log.initlog(sb);
        println!(
            "sb: size {}, nblocks {}, ninodes {}, nlog {}, logstart {} inodestart {}, bmap start {}",
            sb.size,
            sb.nblocks,
            sb.ninodes,
            sb.nlog,
            sb.logstart,
            sb.inodestart,
            sb.bmapstart
            );
    }
    
    pub fn ialloc<'a>(&'a self, i_type: u16) -> Result<CachedInode<'a>, libc::c_int> {
        let sb = self.sb.as_ref().unwrap();
        let num_inodes = sb.ninodes;
        let log = self.log.as_ref().unwrap();
        for block_inum in (0..num_inodes as usize).step_by(IPB) {
            let _guard = self.ialloc_lock.as_ref().unwrap().write();
            let disk = self.disk.as_ref().unwrap();
            let mut bh = disk.bread(iblock(block_inum, &sb) as u64)?;
            let data_slice = bh.data_mut();
            for inum_idx in (block_inum % IPB)..IPB {
                let inum = block_inum + inum_idx;
                if inum == 0 {
                    continue;
                }
                // Get the specific inode offset
                let inode_offset = (inum as usize % IPB) * mem::size_of::<Xv6fsInode>();
    
                let inode_slice =
                    &mut data_slice[inode_offset..inode_offset + mem::size_of::<Xv6fsInode>()];
    
                let mut dinode = Xv6fsInode::new();
                dinode.extract_from(inode_slice).map_err(|_| libc::EIO)?;
                // Check if inode is free
                if dinode.inode_type == 0 {
                    dinode.major = 0;
                    dinode.minor = 0;
                    dinode.size = 0;
                    for addr_mut in dinode.addrs.iter_mut() {
                        *addr_mut = 0;
                    }
                    dinode.inode_type = i_type;
                    dinode.nlink = 1;
                    dinode.dump_into(inode_slice).map_err(|_| libc::EIO)?;
                    bh.mark_buffer_dirty();
                    log.log_write(iblock(inum, &sb) as u32);
                    return self.iget(inum as u64);
                }
            }
        }
        return Err(libc::EIO);
    }
    
    pub fn iupdate(&self, internals: &InodeInternal, inum: u32) -> Result<(), libc::c_int> {
        let log = self.log.as_ref().unwrap();
        let disk = self.disk.as_ref().unwrap();
        let sb = self.sb.as_ref().unwrap();
        let iblock = iblock(inum as usize, &sb);
        let mut bh = disk.bread(iblock as u64)?;
        let data_slice = bh.data_mut();
    
        // Get the specific inode offset
        let inode_offset = (inum as usize % IPB) * mem::size_of::<Xv6fsInode>();
        let inode_slice = &mut data_slice[inode_offset..inode_offset + mem::size_of::<Xv6fsInode>()];
    
        let mut disk_inode = Xv6fsInode::new();
        disk_inode
            .extract_from(inode_slice)
            .map_err(|_| libc::EIO)?;
        disk_inode.inode_type = internals.inode_type;
        disk_inode.major = internals.major;
        disk_inode.minor = internals.minor;
        disk_inode.nlink = internals.nlink;
        disk_inode.size = internals.size;
        disk_inode.addrs.copy_from_slice(&internals.addrs);
        disk_inode.dump_into(inode_slice).map_err(|_| libc::EIO)?;
    
        bh.mark_buffer_dirty();
        log.log_write(iblock as u32);
        return Ok(());
    }
    
    pub fn iget<'a>(&'a self, inum: u64) -> Result<CachedInode<'a>, libc::c_int> {
        let mut final_idx = None;
    
        let icache = self.ilock_cache.as_ref().unwrap();
        for (idx, inode_lock) in icache.iter().enumerate() {
            let mut inode = match inode_lock.try_write() {
                Ok(x) => x,
                Err(_) => continue,
            };
            let disk = self.disk.as_ref().unwrap();
            let dev_id = disk.as_raw_fd();
            if inode.nref > 0 && inode.dev == dev_id as u32 && inode.inum == inum as u32 {
                inode.nref += 1;
    
                return Ok(CachedInode {
                    idx: idx,
                    inum: inum as u32,
                    fs: self,
                });
            }
            if final_idx.is_none() && inode.nref == 0 {
                {
                    let mut new_inode_int = inode.internals.write().map_err(|_| {libc::EIO})?;
                    new_inode_int.valid = 0;
                }
                inode.dev = dev_id as u32;
                inode.inum = inum as u32;
                inode.nref = 1;
                final_idx = Some(idx);
            }
        }
    
        let new_inode_idx = final_idx.ok_or(libc::EIO)?;
    
        let ret = Ok(CachedInode {
            idx: new_inode_idx,
            inum: inum as u32,
            fs: self,
        });
        return ret;
    }
    
    pub fn ilock<'a>(&self, 
        inode_idx: usize,
        icache: &'a Vec<RwLock<Inode>>,
        inum: u32,
    ) -> Result<RwLockReadGuard<'a, Inode>, libc::c_int> {
        let inode_outer_lock = icache.get(inode_idx).ok_or(libc::EIO)?;
        let inode_outer = inode_outer_lock.read().map_err(|_| { libc::EIO})?;
        {
            let mut internals = inode_outer.internals.write().map_err(|_| {libc::EIO})?;
    
            if internals.valid == 0 {
                let disk = self.disk.as_ref().unwrap();
                let sb = self.sb.as_ref().unwrap();
                let bh = disk.bread(iblock(inum as usize, &sb) as u64)?;
                let data_slice = bh.data();
    
                // Get the specific inode offset
                let inode_offset = (inum as usize % IPB) * mem::size_of::<Xv6fsInode>();
    
                let inode_slice =
                    &data_slice[inode_offset..inode_offset + mem::size_of::<Xv6fsInode>()];
                let mut disk_inode = Xv6fsInode::new();
                disk_inode
                    .extract_from(inode_slice)
                    .map_err(|_| libc::EIO)?;
    
                internals.valid = 0;
                internals.inode_type = disk_inode.inode_type;
                internals.major = disk_inode.major;
                internals.minor = disk_inode.minor;
                internals.nlink = disk_inode.nlink;
                internals.size = disk_inode.size;
                internals.addrs.copy_from_slice(&disk_inode.addrs);
                internals.valid = 1;
                if internals.inode_type == 0 {
                    return Err(libc::EIO);
                }
                
            }
        }
        return Ok(inode_outer);
    }
    
    pub fn iput(&self, inode: &mut CachedInode) -> Result<(), libc::c_int> {
        let icache = self.ilock_cache.as_ref().unwrap();
        {
            let inode_guard = self.ilock(inode.idx, &icache, inode.inum)?;
            let mut internals = inode_guard.internals.write().map_err(|_| {libc::EIO})?;
            if internals.valid != 0 && internals.nlink == 0 {
                let r;
                {
                    let dinode_lock = icache.get(inode.idx).ok_or(libc::EIO)?;
                    let dinode = dinode_lock.read().map_err(|_| { libc::EIO })?;
                    r = dinode.nref;
                }
                if r == 1 {
                    self.itrunc(inode, &mut internals)?;
                    internals.inode_type = 0;
                    self.iupdate(&internals, inode.inum)?;
                    internals.valid = 0;
                }
            }
        }
    
        let dinode_lock = icache.get(inode.idx).ok_or(libc::EIO)?;
        let mut dinode = dinode_lock.write().map_err(|_| {libc::EIO})?;
        dinode.nref -= 1;
        return Ok(());
    }
    
    fn bmap(&self, inode: &mut InodeInternal, blk_idx: usize) -> Result<u32, libc::c_int> {
        let log = self.log.as_ref().unwrap();
        let mut idx = blk_idx;
        if idx < NDIRECT as usize {
            let addr = inode.addrs.get_mut(idx).ok_or(libc::EIO)?;
            if *addr == 0 {
                return self.balloc().map(|blk_id| {
                    *addr = blk_id;
                    blk_id
                });
            }
            return Ok(*addr);
        }
    
        idx -= NDIRECT as usize;
        if idx < NINDIRECT as usize {
            // indirect block
            let ind_blk_id = inode.addrs.get_mut(NDIRECT as usize).ok_or(libc::EIO)?;
            if *ind_blk_id == 0 {
                self.balloc().map(|blk_id| {
                    *ind_blk_id = blk_id;
                })?;
            }
    
            let result_blk_id: u32;
            let disk = self.disk.as_ref().unwrap();
            let mut bh = disk.bread(*ind_blk_id as u64)?;
            let b_data = bh.data_mut();
    
            let mut cell_data = [0; 4];
            let cell_segment = &mut b_data[idx * 4 .. (idx + 1) * 4];
            cell_data.copy_from_slice(cell_segment);
            let cell = u32::from_ne_bytes(cell_data);
            if cell == 0 {
                // need to allocate blk
                result_blk_id = self.balloc()?;
                let blk_data = result_blk_id.to_ne_bytes();
                cell_segment.copy_from_slice(&blk_data);
                bh.mark_buffer_dirty();
                log.log_write(*ind_blk_id);
            } else {
                // just return the blk
                result_blk_id = cell;
            }
    
            return Ok(result_blk_id);
        }
    
        if idx < (MAXFILE - NDIRECT) as usize {
            idx -= NINDIRECT as usize;
            // double indirect block
            let dind_blk_id = inode
                .addrs
                .get_mut(NDIRECT as usize + 1)
                .ok_or(libc::EIO)?;
            if *dind_blk_id == 0 {
                self.balloc().map(|blk_id| {
                    *dind_blk_id = blk_id;
                })?;
            }
    
            let disk = self.disk.as_ref().unwrap();
            let mut bh = disk.bread(*dind_blk_id as u64)?;
            let b_data = bh.data_mut();
            let dind_idx = idx / NINDIRECT as usize;
    
            let mut cell_data = [0; 4];
            let cell_segment = &mut b_data[dind_idx * 4 .. (dind_idx + 1) * 4];
            cell_data.copy_from_slice(cell_segment);
            let cell = u32::from_ne_bytes(cell_data);
    
            if cell == 0 {
                let result_blk_id = self.balloc()?;
                let result_blk_data = result_blk_id.to_ne_bytes();
                cell_segment.copy_from_slice(&result_blk_data);
                bh.mark_buffer_dirty();
                log.log_write(*dind_blk_id);
            }
    
            let mut dbh = disk.bread(cell as u64)?;
            let db_data = dbh.data_mut();
            let dblock_idx = idx % NINDIRECT as usize;
    
            let result_blk_id: u32;
            let mut dcell_data = [0; 4];
            let dcell_segment = &mut db_data[dblock_idx * 4 .. (dblock_idx + 1) * 4];
            dcell_data.copy_from_slice(dcell_segment);
            let dcell = u32::from_ne_bytes(dcell_data);
            if dcell == 0 {
                result_blk_id = self.balloc()?;
                let result_blk_data = result_blk_id.to_ne_bytes();
                dcell_segment.copy_from_slice(&result_blk_data);
                dbh.mark_buffer_dirty();
                log.log_write(cell);
            } else {
                result_blk_id = dcell;
            }
            return Ok(result_blk_id);
        }
    
        return Err(libc::EIO);
    }
    
    pub fn itrunc(&self, inode: &mut CachedInode, internals: &mut InodeInternal) -> Result<(), libc::c_int> {
        for i in 0..NDIRECT as usize {
            let addr = internals.addrs.get_mut(i).ok_or(libc::EIO)?;
            if *addr != 0 {
                self.bfree(*addr as usize)?;
                *addr = 0;
            }
        }
    
        let disk = self.disk.as_ref().unwrap();
        let ind_blk_id = internals
            .addrs
            .get_mut(NDIRECT as usize)
            .ok_or(libc::EIO)?;
        if *ind_blk_id != 0 {
            let bh = disk.bread(*ind_blk_id as u64)?;
            let b_data = bh.data();
    
            let mut addr_slice = [0; 4];
            for i in 0..NINDIRECT as usize {
                addr_slice.copy_from_slice(&b_data[i * 4..(i + 1) * 4]);
                let addr = u32::from_ne_bytes(addr_slice);
                if addr != 0 {
                    self.bfree(addr as usize)?;
                }
            }
            self.bfree(*ind_blk_id as usize)?;
            *ind_blk_id = 0;
        }
        let dind_blk_id = internals
            .addrs
            .get_mut(NDIRECT as usize + 1)
            .ok_or(libc::EIO)?;
        if *dind_blk_id != 0 {
            let mut bh = disk.bread(*dind_blk_id as u64)?;
            let b_data = bh.data_mut();
    
            let mut ind_addr_slice = [0; 4];
            for i in 0..NINDIRECT as usize {
                let ind_region = &mut b_data[i * 4..(i + 1) * 4];
                ind_addr_slice.copy_from_slice(ind_region);
                let ind_blk_id = u32::from_ne_bytes(ind_addr_slice);
                if ind_blk_id != 0 {
                    let dbh = disk.bread(ind_blk_id as u64)?;
                    let db_data = dbh.data();
                    let mut daddr_slice = [0; 4];
    
                    for j in 0..NINDIRECT as usize {
                        let daddr_region = &db_data[j * 4..(j + 1) * 4];
                        daddr_slice.copy_from_slice(&daddr_region);
                        let daddr = u32::from_ne_bytes(daddr_slice);
                        if daddr != 0 {
                            self.bfree(daddr as usize)?;
                        }
                    }
                    self.bfree(ind_blk_id as usize)?;
                    ind_region.copy_from_slice(&[0; 4]);
                }
            }
            self.bfree(*dind_blk_id as usize)?;
            *dind_blk_id = 0;
        }
    
        internals.size = 0;
        return self.iupdate(&internals, inode.inum);
    }
    
    pub fn stati(&self, ino: u64, internals: &InodeInternal) -> Result<FileAttr, libc::c_int> {
        if internals.inode_type == 0 {
            return Err(libc::ENOENT);
        }
        let file_kind = match internals.inode_type {
            T_DIR => FileType::Directory,
            T_LNK => FileType::Symlink,
            _ => FileType::RegularFile,
        };
        let attr = FileAttr {
            ino: ino,
            size: internals.size,
            blocks: 0,
            atime: Timespec::new(0, 0),
            mtime: Timespec::new(0, 0),
            ctime: Timespec::new(0, 0),
            crtime: Timespec::new(0, 0),
            kind: file_kind,
            perm: 0o077,
            nlink: internals.nlink as u32,
            uid: 0,
            gid: 0,
            rdev: 0,
            flags: 0,
        };
        return Ok(attr);
    }
    
    pub fn readi(
        &self, 
        buf: &mut [u8],
        _off: usize,
        _n: usize,
        internals: &mut InodeInternal,
    ) -> Result<usize, libc::c_int> {
        let mut n = _n;
        let mut off = _off;
        let i_size = internals.size as usize;
        if off > i_size || off + n < off {
            return Err(libc::EIO);
        }
        if off + n > i_size {
            n = i_size - off;
        }
        let mut m;
        let mut dst = 0;
        let mut tot = 0;
    
        while tot < n {
            let block_no = self.bmap(internals, off / BSIZE)?;
            m = min(n - tot, BSIZE - off % BSIZE);
            let disk = self.disk.as_ref().unwrap();
            let bh = disk.bread(block_no as u64)?;
            let data_slice = bh.data();
    
            let data_off = off % BSIZE;
            let data_region = &data_slice[data_off..data_off + m];
    
            let copy_region = &mut buf[dst..dst + m];
            copy_region.copy_from_slice(data_region);
    
            tot += m;
            off += m;
            dst += m;
        }
        return Ok(n);
    }
    
    pub fn writei(&self, 
        buf: &[u8],
        _off: usize,
        n: usize,
        internals: &mut InodeInternal,
        inum: u32,
    ) -> Result<usize, libc::c_int> {
        let log = self.log.as_ref().unwrap();
        let mut off = _off;
        let i_size = internals.size as usize;
        if off + n < off {
            return Err(libc::EIO);
        }
        if off + n > (MAXFILE as usize) * BSIZE {
            return Err(libc::EIO);
        }
    
        let max_blocks = (MAXOPBLOCKS - 1 - 1 - 2) / 2;
        let mut written_blocks = 0;
        let mut end_size = i_size;
    
        if off > i_size {
            let mut start_off = i_size;
            while start_off < off {
                if written_blocks >= max_blocks {
                    break;
                }
                let block_no = self.bmap(internals, start_off / BSIZE)?;
                let disk = self.disk.as_ref().unwrap();
                let mut bh = disk.bread(block_no as u64)?;
    
                let b_data = bh.data_mut();
    
                let m = min(off - start_off, BSIZE - start_off % BSIZE);
                for i in start_off..start_off + m {
                    let idx = b_data.get_mut(i % BSIZE).ok_or(libc::EIO)?;
                    *idx = 0;
                }
                bh.mark_buffer_dirty();
                written_blocks += 1;
                log.log_write(block_no);
    
                start_off += m;
                end_size = start_off;
            }
        }
    
        let mut src = 0;
        let mut tot = 0;
    
        while tot < n {
            if written_blocks >= max_blocks {
                break;
            }
            let block_no = self.bmap(internals, off / BSIZE)?;
            let m = min(n - tot, BSIZE - off % BSIZE);
    
            let disk = self.disk.as_ref().unwrap();
            let mut bh = disk.bread(block_no as u64)?;
    
            let data_slice = bh.data_mut();
            let data_off = off % BSIZE;
            let data_region = &mut data_slice[data_off..data_off + m];
    
            let copy_region = &buf[src..src + m];
            data_region.copy_from_slice(copy_region);
            bh.mark_buffer_dirty();
            log.log_write(block_no);
            written_blocks += 1;
    
            tot += m;
            off += m;
            src += m;
            end_size = off;
        }
    
        if n > 0 && end_size > i_size {
            internals.size = end_size as u64;
            self.iupdate(internals, inum)?;
        }
        return Ok(n);
    }
    
    pub fn dirlookup<'a>(&'a self, 
        internals: &mut InodeInternal,
        name: &OsStr,
        poff: &mut u64,
    ) -> Result<CachedInode<'a>, libc::c_int> {
        // Check if inode is directory
        if internals.inode_type != T_DIR {
            return Err(libc::ENOTDIR);
        }
        let de_size = mem::size_of::<Xv6fsDirent>();
        let mut de_arr_vec: Vec<u8> = vec![0; BSIZE];
    
        let num_blocks = match internals.size {
            0 => 0,
            _ => (internals.size as usize - 1) / BSIZE + 1,
        };
        let search_name = match name.to_str() {
            Some(s) => s,
            None => {
                return Err(libc::ENOENT);
            },
        };
    
        for block_idx in 0..num_blocks {
            let de_arr_slice = de_arr_vec.as_mut_slice();
            self.readi(de_arr_slice, BSIZE * block_idx, BSIZE, internals)?;
            // resolve all dirent entries in the current data block.
            for de_idx in 0..BSIZE / de_size {
                let mut de = Xv6fsDirent::new();
                let de_slice = &mut de_arr_slice[de_idx * de_size..(de_idx + 1) * de_size];
                de.extract_from(de_slice).map_err(|_| libc::EIO)?;
    
                if (block_idx * BSIZE + de_idx * de_size) as u64 >= internals.size {
                    break;
                }
                if de.inum == 0 {
                    continue;
                }
                let de_name = match str::from_utf8(&de.name) {
                    Ok(x) => x,
                    Err(_) => break,
                };
                let de_name_trimmed = de_name.trim_end_matches('\0');
                if de_name_trimmed == search_name {
                    *poff = (block_idx * BSIZE + de_idx * de_size) as u64;
                    return self.iget(de.inum as u64);
                }
            }
        }
    
        return Err(libc::ENOENT);
    }
    
    // create subdirectory with 'name' under the directory pointed to by 'internals'
    pub fn dirlink(&self, 
        internals: &mut InodeInternal,
        name: &OsStr,
        child_inum: u32,
        parent_inum: u32,
    ) -> Result<usize, libc::c_int> {
        // Check if inode is directory
        if internals.inode_type != T_DIR {
            return Err(libc::ENOTDIR);
        }
    
        let de_size = mem::size_of::<Xv6fsDirent>();
        let mut de_arr_vec: Vec<u8> = vec![0; BSIZE];
        let mut final_off = None;
    
        let num_blocks = match internals.size {
            0 => 0,
            _ => (internals.size as usize - 1) / BSIZE + 1,
        };
        for block_idx in 0..num_blocks {
            let de_arr_slice = de_arr_vec.as_mut_slice();
            self.readi(de_arr_slice, BSIZE * block_idx, BSIZE, internals)?;
    
            for de_idx in 0..BSIZE / de_size {
                if (block_idx * BSIZE + de_idx * de_size) >= internals.size as usize {
                    break;
                }
                let mut de = Xv6fsDirent::new();
                let de_slice = &mut de_arr_slice[de_idx * de_size..(de_idx + 1) * de_size];
                de.extract_from(de_slice).map_err(|_| libc::EIO)?;
                if de.inum == 0 {
                    final_off = Some((block_idx * BSIZE + de_idx * de_size) as u64);
                    break;
                }
            }
            if final_off.is_some() {
                break;
            }
        }
        let final_off = final_off.unwrap_or(internals.size);
        let mut de_vec: Vec<u8> = vec![0; de_size];
    
        let mut de = Xv6fsDirent::new();
        let de_slice = de_vec.as_mut_slice();
        de.extract_from(de_slice).map_err(|_| libc::EIO)?;
    
        let name_slice = name.to_str().unwrap().as_bytes();
        if name_slice.len() > DIRSIZ as usize {
            return Err(libc::EIO);
        }
        for (idx, ch) in de.name.iter_mut().enumerate() {
            *ch = match name_slice.get(idx) {
                Some(x) => *x,
                None => 0,
            };
        }
    
        de.inum = child_inum as u32;
        de.dump_into(de_slice).map_err(|_| libc::EIO)?;
    
        if self.writei(
            de_slice,
            final_off as usize,
            de_size,
            internals,
            parent_inum,
        )? != de_size
        {
            return Err(libc::EIO);
        }
    
        return Ok(0);
    }
}