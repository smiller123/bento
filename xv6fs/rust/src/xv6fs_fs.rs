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

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;

use core::cmp::min;
use core::mem;
use core::str;
use core::sync::atomic::{AtomicUsize, Ordering};

use datablock::DataBlock;

use fuse::{FileAttr, FileType};

use crate::xv6fs_file::*;
use crate::xv6fs_ll::*;
use crate::xv6fs_log::*;
use crate::xv6fs_utils::*;

use std::ffi::OsStr;
use std::os::unix::io::AsRawFd;
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
        sb.extract_from(&b_slice[0..mem::size_of::<Xv6fsSB>()])
            .map_err(|_| libc::EIO)?;
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
        let inode_slice =
            &mut data_slice[inode_offset..inode_offset + mem::size_of::<Xv6fsInode>()];

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
                    let mut new_inode_int = inode.internals.write().map_err(|_| libc::EIO)?;
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

    pub fn ilock<'a>(
        &self,
        inode_idx: usize,
        icache: &'a Vec<RwLock<Inode>>,
        inum: u32,
    ) -> Result<RwLockReadGuard<'a, Inode>, libc::c_int> {
        let inode_outer_lock = icache.get(inode_idx).ok_or(libc::EIO)?;
        let inode_outer = inode_outer_lock.read().map_err(|_| libc::EIO)?;
        {
            let mut internals = inode_outer.internals.write().map_err(|_| libc::EIO)?;

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
            let mut internals = inode_guard.internals.write().map_err(|_| libc::EIO)?;
            if internals.valid != 0 && internals.nlink == 0 {
                let r;
                {
                    let dinode_lock = icache.get(inode.idx).ok_or(libc::EIO)?;
                    let dinode = dinode_lock.read().map_err(|_| libc::EIO)?;
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
        let mut dinode = dinode_lock.write().map_err(|_| libc::EIO)?;
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
            let cell_segment = &mut b_data[idx * 4..(idx + 1) * 4];
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
            let dind_blk_id = inode.addrs.get_mut(NDIRECT as usize + 1).ok_or(libc::EIO)?;
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
            let cell_segment = &mut b_data[dind_idx * 4..(dind_idx + 1) * 4];
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
            let dcell_segment = &mut db_data[dblock_idx * 4..(dblock_idx + 1) * 4];
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

    pub fn itrunc(
        &self,
        inode: &mut CachedInode,
        internals: &mut InodeInternal,
    ) -> Result<(), libc::c_int> {
        for i in 0..NDIRECT as usize {
            let addr = internals.addrs.get_mut(i).ok_or(libc::EIO)?;
            if *addr != 0 {
                self.bfree(*addr as usize)?;
                *addr = 0;
            }
        }

        let disk = self.disk.as_ref().unwrap();
        let ind_blk_id = internals.addrs.get_mut(NDIRECT as usize).ok_or(libc::EIO)?;
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

    pub fn writei(
        &self,
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

    // entry lookup
    pub fn dirlookup<'a>(
        &'a self,
        internals: &mut InodeInternal,
        name: &OsStr,
        poff: &mut u64,
    ) -> Result<CachedInode<'a>, libc::c_int> {
        println!("dirlookup");
        // Check if inode is directory
        if internals.inode_type != T_DIR {
            return Err(libc::ENOTDIR);
        }
        let hroot_len = mem::size_of::<Htree_root>();
        let hindex_len = mem::size_of::<Htree_index>();
        let hentry_len = mem::size_of::<Htree_entry>();
        let de_len = mem::size_of::<Xv6fsDirent>();

        let mut hroot_arr_vec: Vec<u8> = vec![0; BSIZE];

        let search_name = match name.to_str() {
            Some(s) => s,
            None => {
                println!("name.to_str failed");
                return Err(libc::ENOENT);
            }
        };
        let mut de = Xv6fsDirent::new();
        let mut de_vec: Vec<u8> = vec![0; de_len];
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

        // get hash of target entry
        let tmp_name = match str::from_utf8(&de.name) {
            Ok(x) => x,
            Err(_) => return Err(libc::EIO),
        };
        let osstr_name = OsStr::new(tmp_name);
        let target_hash = osstr_name.calculate_hash();
        // get hash of target entry
        println!("calculate hash failed");
        // let target_hash = name.calculate_hash();

        // Nodes should be in different logical blocks within the same file

        // read in entire root block
        let root_arr_slice = hroot_arr_vec.as_mut_slice();
        self.readi(root_arr_slice, 0, BSIZE, internals)?;

        // extract root of dir
        let mut root = Htree_root::new();
        let root_slice = &mut root_arr_slice[0..hroot_len];
        root.extract_from(root_slice).map_err(|_| libc::EIO)?;

        // '.' and '..' are always the first two entries in a directory
        if search_name == "." {
            let mut de = Xv6fsDirent::new();
            let de_slice = &mut root_slice[0..de_len];
            de.extract_from(de_slice).map_err(|_| libc::EIO)?;
            *poff = 0;
            return self.iget(de.inum as u64);
        } else if search_name == ".." {
            let mut de = Xv6fsDirent::new();
            let de_slice = &mut root_slice[de_len..2 * de_len];
            de.extract_from(de_slice).map_err(|_| libc::EIO)?;
            *poff = de_len as u64;
            return self.iget(de.inum as u64);
        }

        // all other entries are stored in leaf nodes

        // get all the index entries in a list and do binary search
        let num_indeces = root.ind_entries;
        let mut index_vec: Vec<Htree_entry> = Vec::with_capacity(num_indeces as usize);
        for rie_idx in 0..num_indeces {
            if hroot_len + rie_idx as usize * hentry_len >= BSIZE {
                break;
            }
            let mut ie = Htree_entry::new();
            let ie_slice = &mut root_arr_slice[hroot_len + (hentry_len * rie_idx as usize)
                ..hroot_len + (hentry_len * (rie_idx as usize + 1))];
            ie.extract_from(ie_slice).map_err(|_| libc::EIO)?;
            if ie.block == 0 {
                break;
            }
            index_vec.push(ie);
        }

        // look for correct index node
        let ind_slice = index_vec.as_slice();
        let target_entry = match find_lowerbound(ind_slice, index_vec.len(), target_hash) {
            Some(index) => index,
            None => {
                println!("find rie lower bound failed");
                return Err(libc::ENOENT);
            }
        };

        // get index block
        let target_lblock: u32 = index_vec[target_entry].block;
        let mut hindex_arr_vec: Vec<u8> = vec![0; BSIZE];
        let hindex_arr_slice = hindex_arr_vec.as_mut_slice();
        self.readi(
            hindex_arr_slice,
            target_lblock as usize * BSIZE,
            BSIZE,
            internals,
        )?;

        // get index header
        let mut index = Htree_index::new();
        let hindex_slice = &mut hindex_arr_slice[0..hindex_len];
        index.extract_from(hindex_slice).map_err(|_| libc::EIO)?;

        // create vec for binary search
        let num_entries = index.entries;
        let mut leaf_vec: Vec<Htree_entry> = Vec::with_capacity(num_entries as usize);
        for off in (hindex_len..hindex_len + num_entries as usize * hentry_len).step_by(hentry_len)
        {
            if off >= BSIZE {
                break;
            }
            let mut hentry = Htree_entry::new();
            let hentry_slice = &mut hindex_arr_slice[off..off + hentry_len];
            hentry.extract_from(hentry_slice).map_err(|_| libc::EIO)?;
            if hentry.block == 0 {
                break;
            }
            leaf_vec.push(hentry);
        }

        // get correct leaf node
        let leaf_slice = leaf_vec.as_slice();
        let target_leaf = match find_lowerbound(leaf_slice, leaf_vec.len(), target_hash) {
            Some(index) => index,
            None => {
                println!("find ine lowerbound failed");
                return Err(libc::ENOENT);
            }
        };

        let leaf_idx = leaf_vec[target_leaf].block;
        let mut leaf_arr_vec: Vec<u8> = vec![0; BSIZE];
        let leaf_arr_slice = leaf_arr_vec.as_mut_slice();
        self.readi(leaf_arr_slice, BSIZE * leaf_idx as usize, BSIZE, internals)?;

        // look through the entries in the leafnode
        for de_idx in 0..BSIZE / de_len {
            let mut de = Xv6fsDirent::new();
            let de_slice = &mut leaf_arr_slice[de_idx * de_len..(de_idx + 1) * de_len];
            de.extract_from(de_slice).map_err(|_| libc::EIO)?;

            // if (leaf_idx as usize * BSIZE + de_idx * de_len) as u64 >= internals.size {
            //     break;
            // }
            if de.inum == 0 {
                continue;
            }
            let de_name = match str::from_utf8(&de.name) {
                Ok(x) => x,
                Err(_) => break,
            };
            let de_name_trimmed = de_name.trim_end_matches('\0');
            if de_name_trimmed == search_name {
                *poff = (leaf_idx as usize * BSIZE + de_idx * de_len) as u64;
                return self.iget(de.inum as u64);
            }
        }

        // nothing found in leaf node block
        // there is still possibility of a hash collision
        // check collision bit in hash
        // let next_leaf = target_leaf + 1;
        // let leaf_hash = leaf_vec[next_leaf].name_hash;

        // // TODO: get lowest bit
        // let collision = leaf_hash;
        // if collision == 1 {
        //     let leaf2_idx = leaf_vec[next_leaf].block;

        //     let mut leaf2_arr_vec: Vec<u8> = vec![0; BSIZE];
        //     let leaf2_arr_slice = leaf2_arr_vec.as_mut_slice();

        //     self.readi(
        //         leaf2_arr_slice,
        //         BSIZE * leaf2_idx as usize,
        //         BSIZE,
        //         internals,
        //     )?;

        //     for de_idx in 0..BSIZE / de_len {
        //         let mut de = Xv6fsDirent::new();
        //         let de_slice = &mut leaf_arr_slice[de_idx * de_len..(de_idx + 1) * de_len];
        //         de.extract_from(de_slice).map_err(|_| libc::EIO)?;

        //         if (leaf2_idx as usize * BSIZE + de_idx * de_len) as u64 >= internals.size {
        //             break;
        //         }
        //         if de.inum == 0 {
        //             continue;
        //         }
        //         let de_name = match str::from_utf8(&de.name) {
        //             Ok(x) => x,
        //             Err(_) => break,
        //         };
        //         let de_name_trimmed = de_name.trim_end_matches('\0');
        //         if de_name_trimmed == search_name {
        //             *poff = (leaf2_idx as usize * BSIZE + de_idx * de_len) as u64;
        //             return self.iget(de.inum as u64);
        //         }
        //     }
        // }

        // no hash collision, no entry found
        return Err(libc::ENOENT);
    }

    // create subdirectory with 'name' under the directory pointed to by 'internals'
    pub fn dirlink(
        &self,
        internals: &mut InodeInternal,
        name: &OsStr,
        child_inum: u32,
        parent_inum: u32,
    ) -> Result<usize, libc::c_int> {
        // check if inode is directory
        if internals.inode_type != T_DIR {
            return Err(libc::ENOTDIR);
        }
        println!("\ndirlink..");
        let hroot_len = mem::size_of::<Htree_root>();
        let hindex_len = mem::size_of::<Htree_index>();
        let hentry_len = mem::size_of::<Htree_entry>();
        let de_len = mem::size_of::<Xv6fsDirent>();

        let mut hroot_arr_vec: Vec<u8> = vec![0; BSIZE];

        let search_name = match name.to_str() {
            Some(s) => s,
            None => {
                return Err(libc::ENOENT);
            }
        };

        // Nodes should be in different logical blocks within the same file

        // read in entire root block
        let root_arr_slice = hroot_arr_vec.as_mut_slice();
        self.readi(root_arr_slice, 0, BSIZE, internals)?;

        // extract root of dir
        let mut root = Htree_root::new();
        let root_slice = &mut root_arr_slice[0..hroot_len];
        root.extract_from(root_slice).map_err(|_| libc::EIO)?;
        let num_indeces = root.ind_entries;
        let mut num_blocks = root.blocks as usize;
        println!(
            "extracted root node - depth: {}, num_indeces: {}, num_blocks: {}",
            root.depth, num_indeces, num_blocks
        );

        // dirent to be written to leaf node
        let mut de = Xv6fsDirent::new();
        let mut de_vec: Vec<u8> = vec![0; de_len];
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

        // get hash of target entry
        let tmp_name = match str::from_utf8(&de.name) {
            Ok(x) => x,
            Err(_) => return Err(libc::EIO),
        };
        let osstr_name = OsStr::new(tmp_name);
        let target_hash = osstr_name.calculate_hash();
        println!("search_name: {}, hash: {}", search_name, target_hash);
        // new directory, create root node
        if search_name == "." {
            println!("..creating '.'");
            root.ind_entries = 0;
            root.blocks = 1;
            root.dump_into(root_slice).map_err(|_| libc::EIO)?;
            let root_de_slice = &mut root_slice[0..de_len];
            de.dump_into(root_de_slice).map_err(|_| libc::EIO)?;

            if self.writei(root_slice, 0, hroot_len, internals, parent_inum)? != hroot_len {
                return Err(libc::EIO);
            }

            return Ok(0);
        } else if search_name == ".." {
            println!("..creating '..'");
            let root_de_slice = &mut root_slice[de_len..2 * de_len];
            de.dump_into(root_de_slice).map_err(|_| libc::EIO)?;
            println!("..writing at off: {}", de_len);
            if self.writei(root_de_slice, de_len, de_len, internals, parent_inum)? != de_len {
                return Err(libc::EIO);
            }

            return Ok(0);
        }

        println!("..regular dirent");
        // regular dirent
        de.dump_into(de_slice).map_err(|_| libc::EIO)?;
        // directory is empty
        if num_indeces == 0 {
            println!("..empty directory");
            num_blocks = 1;
            let rie_offset = hroot_len;
            let index_offset = num_blocks * BSIZE;
            let ine_offset = index_offset + hindex_len;
            let de_offset = (num_blocks + 1) * BSIZE;

            // create new index entry in root node
            let mut rie = Htree_entry::new();
            let mut rie_vec: Vec<u8> = vec![0; hentry_len];
            let rie_slice = rie_vec.as_mut_slice();
            rie.name_hash = target_hash;
            rie.block = (index_offset / BSIZE) as u32;
            println!(
                "..writing rie with hash: {}, block: {} at off: {}",
                rie.name_hash, rie.block, rie_offset
            );
            rie.dump_into(rie_slice).map_err(|_| libc::EIO)?;
            if self.writei(rie_slice, rie_offset, hentry_len, internals, parent_inum)? != hentry_len
            {
                return Err(libc::EIO);
            }

            // create index node
            let mut index = Htree_index::new();
            let mut index_vec: Vec<u8> = vec![0; hindex_len];
            let index_slice = index_vec.as_mut_slice();
            index.entries = 1 as u32;
            println!(
                "..writing index with entries #: {} at off: {}",
                index.entries, index_offset
            );
            index.dump_into(index_slice).map_err(|_| libc::EIO)?;

            if self.writei(
                index_slice,
                index_offset,
                hindex_len,
                internals,
                parent_inum,
            )? != hindex_len
            {
                return Err(libc::EIO);
            }

            // create entry in index node
            let mut ine = Htree_entry::new();
            let mut ine_vec: Vec<u8> = vec![0; hentry_len];
            let ine_slice = ine_vec.as_mut_slice();
            ine.name_hash = target_hash;
            ine.block = (de_offset / BSIZE) as u32;

            println!(
                "..writing ine with hash: {}, block: {} at off: {}",
                ine.name_hash, ine.block, ine_offset
            );
            ine.dump_into(ine_slice).map_err(|_| libc::EIO)?;

            if self.writei(ine_slice, ine_offset, hentry_len, internals, parent_inum)? != hentry_len
            {
                return Err(libc::EIO);
            }

            if self.writei(de_slice, de_offset, de_len, internals, parent_inum)? != de_len {
                return Err(libc::EIO);
            }

            // update root info
            root.depth = 2;
            root.ind_entries = 1;
            root.blocks = 3;
            println!(
                "..updating root - depth: {}, ind_entries: {}",
                root.depth, root.ind_entries
            );
            root.dump_into(root_slice).map_err(|_| libc::EIO)?;
            if self.writei(root_slice, 0, hroot_len, internals, parent_inum)? != hroot_len {
                return Err(libc::EIO);
            }
            println!("..OK\n");
            return Ok(0);
        }

        println!("..directory is not empty");
        // directory is not empty
        let mut index_vec: Vec<Htree_entry> = Vec::with_capacity((num_indeces + 1) as usize);
        println!("..adding root indences to vec");
        for rie_idx in 0..num_indeces {
            println!("rie_idx: {}", rie_idx);
            if hroot_len + rie_idx as usize * hentry_len >= BSIZE {
                println!(
                    "..breaking cuz outside block. {} >= {}",
                    hroot_len + rie_idx as usize * hentry_len,
                    BSIZE
                );
                break;
            }
            let mut ie = Htree_entry::new();
            let ie_slice = &mut root_arr_slice[hroot_len + (hentry_len * rie_idx as usize)
                ..hroot_len + (hentry_len * (rie_idx as usize + 1))];
            println!("..extracting ie from slice");
            ie.extract_from(ie_slice).map_err(|_| libc::EIO)?;
            println!(
                "extracted - rie hash: {}, block: {}",
                ie.name_hash, ie.block
            );
            if ie.block == 0 {
                break;
            }
            println!("..adding rie hash: {}, block: {}", ie.name_hash, ie.block);
            index_vec.push(ie);
        }

        // case: new hash < lowest hash value in root entries
        if target_hash < index_vec[0].name_hash {
            let rie_offset = hroot_len;
            let index_offset = num_blocks * BSIZE;
            let ine_offset = index_offset + hindex_len;
            let de_offset = (num_blocks + 1) * BSIZE;

            // create index node
            let mut index = Htree_index::new();
            let mut index_bvec: Vec<u8> = vec![0; hindex_len];
            let index_slice = index_bvec.as_mut_slice();
            index.entries = 1 as u32;
            println!(
                "..writing index with entries #: {} at off: {}",
                index.entries, index_offset
            );
            index.dump_into(index_slice).map_err(|_| libc::EIO)?;

            if self.writei(
                index_slice,
                index_offset,
                hindex_len,
                internals,
                parent_inum,
            )? != hindex_len
            {
                return Err(libc::EIO);
            }

            // create entry in index node
            let mut ine = Htree_entry::new();
            let mut ine_vec: Vec<u8> = vec![0; hentry_len];
            let ine_slice = ine_vec.as_mut_slice();
            ine.name_hash = target_hash;
            ine.block = (de_offset / BSIZE) as u32;

            println!(
                "..writing ine with hash: {}, block: {} at off: {}",
                ine.name_hash, ine.block, ine_offset
            );
            ine.dump_into(ine_slice).map_err(|_| libc::EIO)?;

            if self.writei(ine_slice, ine_offset, hentry_len, internals, parent_inum)? != hentry_len
            {
                return Err(libc::EIO);
            }

            if self.writei(de_slice, de_offset, de_len, internals, parent_inum)? != de_len {
                return Err(libc::EIO);
            }

            let mut rie = Htree_entry::new();
            rie.name_hash = target_hash;
            rie.block = (index_offset / BSIZE) as u32;

            let mut index_vec_rev: Vec<Htree_entry> = Vec::with_capacity(index_vec.len());
            while let Some(hen) = index_vec.pop() {
                index_vec_rev.push(hen);
            }
            index_vec_rev.push(rie);
            println!(
                "..writing rie with hash: {}, block: {} at off: {}",
                rie.name_hash, rie.block, rie_offset
            );

            let mut rie_idx = 0;
            while let Some(rie) = index_vec_rev.pop() {
                let mut rie_vec: Vec<u8> = vec![0; hentry_len];
                let rie_slice = rie_vec.as_mut_slice();
                rie.dump_into(rie_slice).map_err(|_| libc::EIO)?;
                let offset = hroot_len + rie_idx * hentry_len;
                if self.writei(rie_slice, offset, hentry_len, internals, parent_inum)? != hentry_len
                {
                    return Err(libc::EIO);
                }

                rie_idx += 1;
            }
            println!("rie_idx = {}", rie_idx);
            // update root info and add new entry
            let root2_slice = &mut root_arr_slice[0..hroot_len];
            root.ind_entries += 1;
            root.blocks += 2;
            root.dump_into(root2_slice).map_err(|_| libc::EIO)?;
            if self.writei(root2_slice, 0, hroot_len, internals, parent_inum)? != hroot_len {
                return Err(libc::EIO);
            }
            return Ok(0);
        }

        // look for correct index node block
        let ind_slice = index_vec.as_slice();
        let target_entry = match find_lowerbound(ind_slice, index_vec.len(), target_hash) {
            Some(index) => index,
            None => {
                println!("find lowerbound for rie failed");
                return Err(libc::ENOENT);
            }
        };
        println!("..found lowerbound for rie: {}", target_entry);

        let target_lblock: u32 = index_vec[target_entry].block;
        let mut hindex_arr_vec: Vec<u8> = vec![0; BSIZE];
        let hindex_arr_slice = hindex_arr_vec.as_mut_slice();
        self.readi(
            hindex_arr_slice,
            target_lblock as usize * BSIZE,
            BSIZE,
            internals,
        )?;
        println!("..target_index_block: {}", target_lblock);
        // get index header
        let mut index = Htree_index::new();
        let hindex_slice = &mut hindex_arr_slice[0..hindex_len];
        index.extract_from(hindex_slice).map_err(|_| libc::EIO)?;

        // create vec for binary search
        let num_entries = index.entries;
        let mut leaf_vec: Vec<Htree_entry> = Vec::with_capacity((num_entries + 1) as usize);
        println!(
            "..num_entries in index: {}, finding lower bound",
            num_entries
        );
        for off in (hindex_len..hindex_len + hentry_len * num_entries as usize).step_by(hentry_len)
        {
            if off >= BSIZE {
                break;
            }
            let mut hentry = Htree_entry::new();
            let hentry_slice = &mut hindex_arr_slice[off..off + hentry_len];
            hentry.extract_from(hentry_slice).map_err(|_| libc::EIO)?;
            if hentry.block == 0 {
                break;
            }
            println!(
                "..adding entry hash: {}, block: {} ",
                hentry.name_hash, hentry.block
            );
            leaf_vec.push(hentry);
        }

        // get correct leaf node
        let leaf_slice = leaf_vec.as_slice();
        let target_leaf = match find_lowerbound(leaf_slice, leaf_vec.len(), target_hash) {
            Some(index) => index,
            None => return Err(libc::ENOENT),
        };

        let leaf_idx = leaf_vec[target_leaf].block;
        println!(
            "lowerbound for leafnode: {}, block: {}",
            target_leaf, leaf_idx,
        );
        let mut leaf_arr_vec: Vec<u8> = vec![0; BSIZE];
        let leaf_arr_slice = leaf_arr_vec.as_mut_slice();
        self.readi(leaf_arr_slice, BSIZE * leaf_idx as usize, BSIZE, internals)?;

        // TODO need to check for collision, if so, add to next leaf node and set collision bit
        let mut final_off = None;
        for de_idx in 0..BSIZE / de_len {
            println!("de_idx: {}", de_idx);
            let mut de_temp = Xv6fsDirent::new();
            let de_slice_temp = &mut leaf_arr_slice[de_idx * de_len..(de_idx + 1) * de_len];
            de_temp.extract_from(de_slice_temp).map_err(|_| libc::EIO)?;
            if de_temp.inum == 0 {
                println!("inum == 0");
                final_off = Some((leaf_idx as usize * BSIZE + de_idx * de_len) as u64);
            }

            // there is enough space in the leaf node
            if final_off.is_some() {
                println!("..found space in leafnode, inserting..");
                let final_off = final_off.unwrap();
                if self.writei(de_slice, final_off as usize, de_len, internals, parent_inum)?
                    != de_len
                {
                    return Err(libc::EIO);
                }
                println!("..OK, inserted at off: {}\n", final_off);
                return Ok(0);
            }
        }
        println!("no space in current leaf node");

        // not enough space in the current leaf node
        // need to split leaf nodes

        // final off should be internals.size
        let final_off = num_blocks * BSIZE;
        let mut de_map: BTreeMap<u32, Vec<Xv6fsDirent>> = BTreeMap::new();

        println!(
            "..current leafnode is full. Splitting.. new leaf node at off: {}",
            final_off
        );
        for de_idx in 0..BSIZE / de_len {
            println!("de_idx: {}", de_idx);
            let mut de_temp = Xv6fsDirent::new();
            let de_slice_temp = &mut leaf_arr_slice[de_idx * de_len..(de_idx + 1) * de_len];
            de_temp.extract_from(de_slice_temp).map_err(|_| libc::EIO)?;
            if de_temp.inum == 0 {
                continue;
            }
            let de_name = match str::from_utf8(&de_temp.name) {
                Ok(x) => x,
                Err(_) => return Err(libc::EIO),
            };
            println!("de_name: {}", de_name);
            let de_name = OsStr::new(de_name);
            let de_hash = de_name.calculate_hash();
            println!("de_hash: {}", de_hash);

            if !de_map.contains_key(&de_hash) {
                de_map.insert(de_hash, Vec::with_capacity(3));
            }
            if let Some(x) = de_map.get_mut(&de_hash) {
                x.push(de_temp);
            }
        }
        println!("..total number of dirents: {}", de_map.len());
        // get the new lower bounds for the leaf nodes
        let mut keys: Vec<_> = de_map.keys().cloned().collect();
        let keys2 = keys.split_off(keys.len() / 2);
        let mut de_map2 = de_map.split_off(&keys2[0]);
        println!(
            "de_map len: {}, de_map2 len: {}",
            de_map.len(),
            de_map2.len()
        );
        // let leaf1_lower = 0;
        let leaf2_lower = keys2[0];
        println!("leaf2_lower bound: {}", leaf2_lower);

        // get vecs from the dirents maps
        let mut leaf1_dir_vec: Vec<Xv6fsDirent> = Vec::with_capacity(66);
        let mut leaf2_dir_vec: Vec<Xv6fsDirent> = Vec::with_capacity(66);

        {
            println!("map1 keys");
            for key in keys {
                println!("{}", key);
                if let Some(mut val) = de_map.remove(&key) {
                    while let Some(dirent) = val.pop() {
                        leaf1_dir_vec.push(dirent);
                    }
                }
            }
        }
        {
            println!("map2 keys");
            for key in keys2 {
                println!("{}", key);
                if let Some(mut val) = de_map2.remove(&key) {
                    while let Some(dirent) = val.pop() {
                        leaf2_dir_vec.push(dirent);
                    }
                }
            }
        }
        println!("leaf1_dir_vec len: {}", leaf1_dir_vec.len());
        println!("leaf2_dir_vec len: {}", leaf2_dir_vec.len());

        // keep half dirents into current leaf node
        {
            let mut leaf_vec: Vec<u8> = vec![0; BSIZE];
            let leaf_slice = leaf_vec.as_mut_slice();
            let mut idx = 0;
            println!("..orig leafnode # dirents: {}", leaf1_dir_vec.len());
            while let Some(de) = leaf1_dir_vec.pop() {
                let leaf_idx_slice = &mut leaf_slice[idx * de_len..(idx + 1) * de_len];
                de.dump_into(leaf_idx_slice).map_err(|_| libc::EIO)?;
                idx += 1;
            }
            // let write_size = idx * de_len as usize;
            // need overwrite prev dirents with 0's.
            let write_size = BSIZE;
            println!(
                "..written # dirents: {}, total write size: {}",
                idx, write_size
            );
            if self.writei(
                leaf_slice,
                leaf_idx as usize * BSIZE,
                write_size,
                internals,
                parent_inum,
            )? != write_size
            {
                println!("orig leafnode write failed");
                return Err(libc::EIO);
            }
        }

        // write other half into a new leafnode
        {
            let mut leaf_vec: Vec<u8> = vec![0; BSIZE];
            let leaf_slice = leaf_vec.as_mut_slice();
            let mut idx = 0;
            println!("..new leafnode # dirents: {}", leaf2_dir_vec.len());
            while let Some(de) = leaf2_dir_vec.pop() {
                let leaf_idx_slice = &mut leaf_slice[idx * de_len..(idx + 1) * de_len];
                de.dump_into(leaf_idx_slice).map_err(|_| libc::EIO)?;
                idx += 1;
            }
            let write_size = idx * de_len as usize;
            println!(
                "..written # dirents: {}, total write size: {}",
                idx, write_size
            );
            if self.writei(
                leaf_slice,
                (num_blocks + 1) * BSIZE,
                write_size,
                internals,
                parent_inum,
            )? != write_size
            {
                println!("new leafnode write failed");
                return Err(libc::EIO);
            }
        }

        println!("..creating new hentry in index node");
        // create a new hentry in the index node
        let num_entries = index.entries as usize;

        // sort old entries in index node
        let mut ie_map: BTreeMap<u32, Htree_entry> = BTreeMap::new();
        println!("leaf_vec len: {}", leaf_vec.len());
        while let Some(ie) = leaf_vec.pop() {
            ie_map.insert(ie.name_hash, ie);
        }
        println!("ie_map len: {}", ie_map.len());
        let mut new_ie = Htree_entry::new();
        new_ie.name_hash = leaf2_lower;
        new_ie.block = num_blocks as u32 + 1;
        ie_map.insert(new_ie.name_hash, new_ie);
        println!("ie_map len: {}", ie_map.len());
        println!(
            "..new_ie hash: {}, block: {}",
            new_ie.name_hash, new_ie.block
        );
        {
            let mut keys: Vec<_> = ie_map.keys().cloned().collect();
            while let Some(key) = keys.pop() {
                println!("key: {}", key);
                if let Some(val) = ie_map.remove(&key) {
                    leaf_vec.push(val);
                }
            }
        }
        println!("leaf_vec sorted len: {}", leaf_vec.len());
        println!("leaf_vec is sorted");
        // enough space in current index node
        if num_entries < ((BSIZE - hindex_len) / hentry_len) {
            // // insert in sorted order
            println!(".. inserting new_ie in current index node");
            let mut index_vec: Vec<u8> = vec![0; BSIZE];
            let index_slice = index_vec.as_mut_slice();
            index.entries += 1;
            let index_header_slice = &mut index_slice[0..hindex_len];
            index.dump_into(index_header_slice).map_err(|_| libc::EIO)?;
            println!("new index entries number: {}", index.entries);
            let mut ie_idx = 0;
            while let Some(ie) = leaf_vec.pop() {
                println!("dumping ine with hash: {}", ie.name_hash);
                let ie_slice = &mut index_slice[hindex_len + ie_idx as usize * hentry_len
                    ..hindex_len + (ie_idx + 1) as usize * hentry_len];
                ie.dump_into(ie_slice).map_err(|_| libc::EIO)?;
                ie_idx += 1;
            }
            let write_size = ie_idx * hentry_len as usize;
            println!("writing ie_idx elements: {}", ie_idx);
            if self.writei(
                index_slice,
                target_lblock as usize * BSIZE,
                write_size,
                internals,
                parent_inum,
            )? != write_size
            {
                println!("write index_slice failed");
                return Err(libc::EIO);
            }
        } // else {
          //     // not enough space, need to split root index nodes

        //     println!("..not enough space, create new hentry in root node");
        //     if (root.ind_entries as usize) < (BSIZE - hroot_len) / hentry_len {
        //         // new entries to add to root node
        //         let mut new_rie = Htree_entry::new();
        //         let mut new_index = Htree_index::new();

        //         // split original index node and update previous index node
        //         let mut ie2_vec = leaf_vec.split_off(leaf_vec.len() / 2);
        //         {
        //             index.entries = leaf_vec.len() as u32;
        //             let mut index_vec: Vec<u8> = vec![0; BSIZE];
        //             let index_slice = index_vec.as_mut_slice();
        //             let index_header_slice = &mut index_slice[de_len..hindex_len];
        //             index.dump_into(index_header_slice).map_err(|_| libc::EIO)?;
        //             let mut sorted_vec: Vec<Htree_entry> = Vec::with_capacity(leaf_vec.len());
        //             while let Some(hen) = leaf_vec.pop() {
        //                 sorted_vec.push(hen);
        //             }
        //             let mut ie_idx = 0;
        //             while let Some(ie) = sorted_vec.pop() {
        //                 let ie_slice = &mut index_slice[hindex_len + ie_idx as usize * hentry_len
        //                     ..hindex_len + (ie_idx + 1) as usize * hentry_len];
        //                 ie.dump_into(ie_slice).map_err(|_| libc::EIO)?;
        //                 ie_idx += 1;
        //             }
        //             let write_size = ie_idx * hentry_len as usize;
        //             if self.writei(
        //                 index_slice,
        //                 target_lblock as usize * BSIZE,
        //                 write_size,
        //                 internals,
        //                 parent_inum,
        //             )? != write_size
        //             {
        //                 return Err(libc::EIO);
        //             }
        //         }

        //         // create new index node with the remaining entries for root indeces

        //         new_index.entries = ie2_vec.len() as u32;
        //         // should be the same as leaf2_lower
        //         let lower_bound = ie2_vec[0].name_hash;
        //         new_rie.name_hash = lower_bound;
        //         new_rie.block = num_blocks as u32 + 2;

        //         {
        //             let mut index_vec: Vec<u8> = vec![0; BSIZE];
        //             let index_slice = index_vec.as_mut_slice();
        //             let index_header_slice = &mut index_slice[0..hindex_len];
        //             new_index
        //                 .dump_into(index_header_slice)
        //                 .map_err(|_| libc::EIO)?;
        //             let mut sorted_vec: Vec<Htree_entry> = Vec::with_capacity(ie2_vec.len());
        //             while let Some(hen) = ie2_vec.pop() {
        //                 sorted_vec.push(hen);
        //             }
        //             let mut ie_idx = 0;
        //             while let Some(ie) = sorted_vec.pop() {
        //                 let ie_slice = &mut index_slice[hindex_len + ie_idx as usize * hentry_len
        //                     ..hindex_len + (ie_idx + 1) as usize * hentry_len];
        //                 ie.dump_into(ie_slice).map_err(|_| libc::EIO)?;
        //                 ie_idx += 1;
        //             }
        //             let write_size = ie_idx * hentry_len as usize;
        //             if self.writei(
        //                 index_slice,
        //                 num_blocks + 2 as usize * BSIZE,
        //                 write_size,
        //                 internals,
        //                 parent_inum,
        //             )? != write_size
        //             {
        //                 return Err(libc::EIO);
        //             }
        //         }
        //         // index_vec = index entries in the root node
        //         // leaf_vec = hentries in index node

        //         // udpate root
        //         let mut rie_map: BTreeMap<u32, Htree_entry> = BTreeMap::new();
        //         while let Some(ie) = index_vec.pop() {
        //             rie_map.insert(ie.name_hash, ie);
        //         }
        //         rie_map.insert(new_rie.name_hash, new_rie);
        //         {
        //             let keys: Vec<_> = rie_map.keys().cloned().collect();
        //             for key in keys {
        //                 if let Some(val) = rie_map.remove(&key) {
        //                     index_vec.push(val);
        //                 }
        //             }
        //         }

        //         {
        //             root.ind_entries += 1;
        //             let root_header_slice = &mut root_arr_slice[0..hroot_len];
        //             root.dump_into(root_header_slice).map_err(|_| libc::EIO)?;
        //             let mut sorted_vec: Vec<Htree_entry> = Vec::with_capacity(index_vec.len());
        //             while let Some(hen) = index_vec.pop() {
        //                 sorted_vec.push(hen);
        //             }
        //             let mut rie_idx = 0;
        //             while let Some(rie) = sorted_vec.pop() {
        //                 let rie_slice = &mut root_arr_slice[hroot_len
        //                     + rie_idx as usize * hentry_len
        //                     ..hroot_len + (rie_idx + 1) as usize * hentry_len];
        //                 rie.dump_into(rie_slice).map_err(|_| libc::EIO)?;
        //                 rie_idx += 1;
        //             }
        //             let write_size = hroot_len + hentry_len * rie_idx as usize;
        //             if self.writei(root_arr_slice, 0, write_size, internals, parent_inum)?
        //                 != write_size
        //             {
        //                 return Err(libc::EIO);
        //             }
        //         }
        //     } else {
        //         // root is cannot contain more index entries
        //         println!("root cannot contain more index entries");
        //         return Err(libc::EIO);
        //     }
        // }

        // END //
        println!("finish dirlink");
        return Ok(0);
    }
}
