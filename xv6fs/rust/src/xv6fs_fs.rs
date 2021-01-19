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
#[cfg(not(feature = "user"))]
use bento::kernel::kobj::BufferHead;

use alloc::collections::btree_map::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;

use core::cmp::min;
use core::mem;
use core::str;
use core::sync::atomic::{AtomicUsize, Ordering};

use datablock::DataBlock;

use fuse::{FileAttr, FileType};

use crate::xv6fs_file::*;
use crate::xv6fs_htree::*;
use crate::xv6fs_ll::*;
use crate::xv6fs_utils::*;
use crate::xv6fs_extents::*;

#[cfg(not(feature = "user"))]
use bento::kernel::journal::*;
#[cfg(feature = "user")]
use crate::xv6fs_log::*;

use std::ffi::OsStr;
use std::os::unix::io::AsRawFd;
use std::sync::*;

use time::Timespec;

static LAST_BLOCK: AtomicUsize = AtomicUsize::new(0);
static LAST_INODE: AtomicUsize = AtomicUsize::new(0);

// Total number of blocks file system wide
static XV6FS_NBLOCKS: AtomicUsize = AtomicUsize::new(0);


// TODO: method that might need change: bmap, iupdate, iget, ilock, iupdate, iput, itrunc, stati, readi, writei.
// probably done: ialloc, 
// TODO: bmap and itruncate
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

    fn bzero(&self, bno: usize, handle: &Handle) -> Result<(), libc::c_int> {
        let disk = self.disk.as_ref().unwrap();
        let mut bh = disk.getblk(bno as u64)?;
        bh.lock();
        handle.get_create_access(&bh);

        let b_slice = bh.data_mut();
        b_slice.fill(0);
        bh.set_buffer_uptodate();
        bh.unlock();

        handle.journal_write(&mut bh);

        return Ok(());
    }

    // non transactional bzero
    #[allow(dead_code)]
    fn bzero_data(&self, bno: usize) -> Result<(), libc::c_int> {
        let disk = self.disk.as_ref().unwrap();
        let mut bh = disk.bread(bno as u64)?;

        let b_slice = bh.data_mut();
        for byte in b_slice {
            *byte = 0;
        }

        bh.mark_buffer_dirty();
        bh.sync_dirty_buffer();

        return Ok(());
    }

    // TODO: add preallocation feature to allow for more contiguous blocks for a file
    // Allocate a block on disk, using a slightly different alloc strategy from xv6.
    // xv6 scans from 0th block and allocates the first available block, we scan from the latest used block since last boot.
    fn balloc(&self, handle: &Handle) -> Result<u32, libc::c_int> {
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
            handle.get_write_access(&bh);

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
                handle.journal_write(&mut bh);
            }
            // extract new block ID x
            if let Some(x) = allocated_block {
                LAST_BLOCK.store(x as usize, Ordering::SeqCst);
                self.bzero(x as usize, &handle)?;
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

    fn bfree(&self, block_id: usize, handle: &Handle) -> Result<(), libc::c_int> {
        // Get block number
        let sb = self.sb.as_ref().unwrap();
        let block_num = bblock(block_id, &sb);

        // Read block
        let disk = self.disk.as_ref().unwrap();
        let mut bh = disk.bread(block_num as u64)?;
        handle.get_write_access(&bh);
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
        handle.journal_write(&mut bh);

        return Ok(());
    }

    pub fn iinit(&mut self) {
        if self.readsb().is_err() {
            println!("Unable to read super block from disk.");
        }

        let mut inode_vec: Vec<RwLock<Inode>> = Vec::with_capacity(NINODE);
        for _ in 0..NINODE {
            inode_vec.push(RwLock::new(Inode::new()));
        }
        self.ilock_cache = Some(inode_vec);
        self.icache_map = Some(RwLock::new(BTreeMap::new()));

        self.ialloc_lock = Some(RwLock::new(0));
        self.balloc_lock = Some(RwLock::new(0));

        let sb = self.sb.as_mut().unwrap();

        if self.log.is_none() {
            let disk_ref = Arc::clone(self.disk.as_ref().unwrap());
            let disk_ref2 = Arc::clone(self.disk.as_ref().unwrap());
            let log = Journal::new_from_disk(disk_ref, disk_ref2, sb.logstart as u64, sb.nlog as i32, BSIZE as i32).unwrap();
            self.log = Some(log);
        }

        XV6FS_NBLOCKS.store(sb.nblocks as usize, Ordering::SeqCst);

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

    pub fn ialloc<'a>(&'a self, i_type: u16, handle: &Handle) -> Result<CachedInode<'a>, libc::c_int> {
        let sb = self.sb.as_ref().unwrap();
        let num_inodes = sb.ninodes;

        let most_recent = LAST_INODE.load(Ordering::SeqCst);
        let mut first = true;
        let last_segment = most_recent - most_recent % IPB;
        let mut block_inum = last_segment;

        while first || block_inum < last_segment {
            let _guard = self.ialloc_lock.as_ref().unwrap().write();
            let disk = self.disk.as_ref().unwrap();
            let mut bh = disk.bread(iblock(block_inum, &sb) as u64)?;
            handle.get_write_access(&bh);
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
                    // clear extent header and extents stored in inode
                    dinode.eh.clear();
                    for extent_mut in dinode.ee_arr.iter_mut() {
                        extent_mut.clear();
                    }
                    dinode.inode_type = i_type;
                    dinode.nlink = 1;
                    dinode.dump_into(inode_slice).map_err(|_| libc::EIO)?;
                    handle.journal_write(&mut bh);
                    LAST_INODE.store(inum as usize, Ordering::SeqCst);
                    return self.iget(inum as u64);
                }
            }
            block_inum += IPB;
            if block_inum >= num_inodes as usize {
                block_inum = 0;
                first = false;
            }
        }
        return Err(libc::EIO);
    }

    pub fn iupdate(&self, internals: &InodeInternal, inum: u32, handle: &Handle) -> Result<(), libc::c_int> {
        let disk = self.disk.as_ref().unwrap();
        let sb = self.sb.as_ref().unwrap();
        let iblock = iblock(inum as usize, &sb);
        let mut bh = disk.bread(iblock as u64)?;
        handle.get_write_access(&bh);
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
        disk_inode.eh = internals.eh;
        disk_inode.ee_arr.copy_from_slice(&internals.ee_arr);
        disk_inode.dump_into(inode_slice).map_err(|_| libc::EIO)?;

        handle.journal_write(&mut bh);
        return Ok(());
    }

    pub fn iget<'a>(&'a self, inum: u64) -> Result<CachedInode<'a>, libc::c_int> {
        let mut final_idx = None;

        let icache = self.ilock_cache.as_ref().unwrap();
        let mut map = self.icache_map.as_ref().unwrap().write().unwrap();
        if map.contains_key(&inum) {
            let idx = map.get(&inum).unwrap();
            let inode_lock = icache.get(*idx).unwrap();
            let inode = inode_lock.read().unwrap();
            let disk = self.disk.as_ref().unwrap();
            let dev_id = disk.as_raw_fd();
            let mut inode_nref = inode.nref.write().unwrap();
            if *inode_nref > 0 && inode.dev == dev_id as u32 && inode.inum == inum as u32 {
                *inode_nref += 1;

                return Ok(CachedInode {
                    idx: *idx,
                    inum: inum as u32,
                    fs: self,
                });
            }
        }
        for (idx, inode_lock) in icache.iter().enumerate() {
            let mut inode = match inode_lock.try_write() {
                Ok(x) => x,
                Err(_) => continue,
            };
            let disk = self.disk.as_ref().unwrap();
            let dev_id = disk.as_raw_fd();
            if final_idx.is_none() && *inode.nref.read().unwrap() == 0 {
                {
                    let mut new_inode_int = inode.internals.write().map_err(|_| libc::EIO)?;
                    new_inode_int.valid = 0;
                }
                inode.dev = dev_id as u32;
                inode.inum = inum as u32;
                *inode.nref.write().unwrap() = 1;
                final_idx = Some(idx);
                map.insert(inum, idx);
                break;
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
                internals.eh = disk_inode.eh;
                internals.ee_arr.copy_from_slice(&disk_inode.ee_arr);
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
                    let dinode = dinode_lock.read().map_err(|_| { libc::EIO })?;
                    r = *dinode.nref.read().unwrap();
                }
                if r == 1 {
                    let handle = self.log.as_ref().unwrap().begin_op(MAXOPBLOCKS as u32);
                    self.itrunc(inode, &mut internals, &handle)?;
                    internals.inode_type = 0;
                    self.iupdate(&internals, inode.inum, &handle)?;
                    internals.valid = 0;
                }
            }
        }

        let dinode_lock = icache.get(inode.idx).ok_or(libc::EIO)?;
        let mut map = self.icache_map.as_ref().unwrap().write().unwrap();
        let dinode = dinode_lock.read().map_err(|_| {libc::EIO})?;
        let mut dinode_nref = dinode.nref.write().unwrap();
        *dinode_nref -= 1;
        if *dinode_nref == 0 {
            map.remove(&(inode.inum as u64));
        }
        return Ok(());
    }

    // When root reaches max # of extents, need to grow in depth.
    fn grow_extent_root(
        &self,
        inode: &mut InodeInternal,
        new_ext: &Xv6fsExtent,
        handle: Option<&Handle>,
        new_tx: Option<Handle>
    ) -> Result<u32, libc::c_int> {
        let mid = INEXTENTS;
        if mid != inode.eh.eh_entries {
            // something is corrupted in root node
            println!("grow_extent_root eh_entries != INEXTENTS");
            return Err(libc::EIO);
        }

        // need to get 2 blocks for 2 new leaf nodes
        let new_blocks_idx = [0; 2];
        {
            let h = match handle {
                Some(_) => handle.unwrap(),
                None => new_tx.get_or_insert_with(|| self.log.as_ref().unwrap().begin_op(5)),
            };
            for i in 0..new_blocks_idx.len() {
                self.balloc(h).map(|blk_id| {
                    new_blocks_idx[i] = blk_id;
                });
            }
        }

        let re_map: BTreeMap<u32, Xv6fsExtent> = get_sorted_root_ext(inode, *new_ext)?;
        let blocks_idx = self.insert_ext_to_leaf_nodes(&re_map, handle, new_tx, new_blocks_idx[0], new_blocks_idx[1], None)?;
        for root_idx in 0..INEXTENTS {
            inode.ee_arr[root_idx as usize].clear();
        }
        inode.ee_arr[0].xe_block = blocks_idx.0;
        inode.ee_arr[0].xe_block_addr = new_blocks_idx[0];
        inode.ee_arr[1].xe_block = blocks_idx.1;
        inode.ee_arr[1].xe_block_addr = new_blocks_idx[1];
 
        return Ok(new_ext.xe_block_addr);
    }
    
    fn insert_ext_to_leaf_nodes (&self,
        ext_map: &BTreeMap<u32, Xv6fsExtent>,
        handle: Option<&Handle>,
        new_tx: Option<Handle>,
        first_block: u32,
        second_block: u32,
        first_bh: Option<BufferHead> 
    ) -> Result<(u32, u32), libc::c_int> {
        let keys: Vec<u32> = ext_map.keys().cloned().collect();
        // split extents into leaf nodes
        let first_key: u32;
        let second_key: u32;
        let mut ext_idx = 0;
        // TODO: make into method
        {       
            let h = match handle {
                Some(_) => handle.unwrap(),
                None => new_tx.get_or_insert_with(|| self.log.as_ref().unwrap().begin_op(5)),
            };

            let mut bh = match first_bh {
                Some(fbh) => fbh,
                None => self.disk.as_ref().unwrap().bread(first_block as u64)?,
            };
            let b_data = bh.data_mut();
            h.get_write_access(&bh);
            let mut ext_off = EH_LEN;
            while ext_idx < keys.len() / 2 {
                let ext_slice = &mut b_data[ext_off..ext_off + EXT_LEN];
                let ext = ext_map.get(&keys[ext_idx]).unwrap();
                if ext_idx == 0 {
                    first_key = ext.xe_block;
                }
                ext.dump_into(ext_slice).map_err(|_| libc::EIO)?;
                ext_idx += 1;
                ext_off += EXT_LEN;
            }
            let leaf_eh = Xv6fsExtentHeader::new();
            leaf_eh.eh_entries = (ext_idx) as u16;
            leaf_eh.eh_depth = 1;
            // TODO: migth need to updaet eh_max
            let eh_slice = &mut b_data[0..EH_LEN];
            leaf_eh.dump_into(eh_slice).map_err(|_| libc::EIO)?;
            h.journal_write(&mut bh);
        }

        {       
            let h = match handle {
                Some(_) => handle.unwrap(),
                None => new_tx.get_or_insert_with(|| self.log.as_ref().unwrap().begin_op(5)),
            };

            let mut bh = self.disk.as_ref().unwrap().bread(second_block as u64)?;
            let b_data = bh.data_mut();
            h.get_write_access(&bh);
            let mut ext_off = EH_LEN;
            while ext_idx < keys.len() {
                let ext_slice = &mut b_data[ext_off..ext_off + EXT_LEN];
                let ext = ext_map.get(&keys[ext_idx]).unwrap();
                if ext_idx == keys.len() / 2 {
                    second_key = ext.xe_block;
                }
                ext.dump_into(ext_slice).map_err(|_| libc::EIO)?;
                ext_idx += 1;
                ext_off += EXT_LEN;
            }
            let leaf_eh = Xv6fsExtentHeader::new();
            leaf_eh.eh_entries = (ext_idx - keys.len() / 2) as u16;
            leaf_eh.eh_depth = 1;
            // TODO: migth need to updaet eh_max
            let eh_slice = &mut b_data[0..EH_LEN];
            leaf_eh.dump_into(eh_slice).map_err(|_| libc::EIO)?;
            h.journal_write(&mut bh);

        }
        return Ok((first_key, second_key));
    }

    // handle should be Some(_) if this bmap is part of a transaction, None otherwise
    // bmap may have to write to disk during some read operation
    // Returns an avaiable block number.
    // If the blk_idx was outside of the total # of blocks of an inode, it will balloc a block and return that.
    // Otherwise, it will just return a previously balloc'd block.
    fn bmap(&self, inode: &mut InodeInternal, blk_idx: usize, handle: Option<&Handle>) -> Result<u32, libc::c_int> {
        let mut idx = blk_idx;

        let mut new_tx: Option<Handle> = None;

        let max_depth = inode.eh.eh_depth;
        let valid_entries = inode.eh.eh_entries;
        let eh_len = mem::size_of::<Xv6fsExtentHeader>();
        let ei_len = mem::size_of::<Xv6fsExtentIdx>();
        let ext_len = mem::size_of::<Xv6fsExtent>();
        let ext_path_vec = Arc::new(Vec::<Xv6fsExtentPath>::with_capacity(3));
        let mut curr_depth = 0;
        let mut ppos = 0;

        let disk = self.disk.as_ref().unwrap();
        let epv_mut = Arc::get_mut(&mut ext_path_vec).unwrap();
        epv_mut.push(Xv6fsExtentPath::new());
        // TODO: consider making helper function for traversal
        loop {
            if curr_depth < max_depth {
                let mut curr_path = epv_mut.get_mut(ppos).unwrap();
                if curr_depth == 0 { // we're at the root
                    curr_path.p_hdr = Some(Arc::new(inode.eh));
                    curr_path.p_maxdepth = inode.eh.eh_depth;
                    let mut ext_idx = match ext_binary_search(&inode.ee_arr[0..valid_entries as usize], valid_entries as u32, idx as u32) {
                        Some(e_idx) => e_idx,
                        None => { // curr inode has no data
                            let ext = inode.ee_arr.get_mut(0).ok_or(libc::EIO)?;
                            let h = match handle {
                                Some(_) => handle.unwrap(),
                                None => new_tx.get_or_insert_with(|| self.log.as_ref().unwrap().begin_op(2)),
                            };
                            return self.balloc(h).map(|blk_id| {
                                ext.xe_block = 0;
                                ext.xe_block_addr = blk_id;
                                blk_id
                            });
                        },
                    };
                    curr_path.p_ext = Some(Arc::new(*inode.ee_arr.get(ext_idx as usize).unwrap()));
            
                } else { // curr_depth > 0, we're in an index node
                    let bh_slice = match curr_path.p_bh {
                        Some(bh) => Arc::get_mut(&mut bh).unwrap().data_mut(),
                        None => {
                            println!("Error in getting data_mut from bh");
                            return Err(libc::EIO);
                        },
                    };
                    let ei_entries = match curr_path.p_hdr {
                        Some(hdr) => hdr.eh_entries,
                        None =>  {
                            println!("Error in header from curr_path");
                            return Err(libc::EIO);
                        },
                    };
                    let ei_vec: Vec<Xv6fsExtentIdx> = Vec::with_capacity(ei_entries as usize);
                    for ei_off in (eh_len..eh_len + (ei_len * ei_entries as usize)).step_by(ei_len) {
                        if ei_off >= BSIZE {
                            break;
                        }

                        let mut ei = Xv6fsExtentIdx::new();
                        let ei_slice = &mut bh_slice[ei_off..ei_off + ei_len];
                        ei.extract_from(ei_slice).map_err(|_| libc::EIO)?;
                        // check for invalid extent index
                        // TODO: might want to do something else than just breaking out of the loop
                        if ei.ei_block_addr == 0 {
                            break;
                        }
                        ei_vec.push(ei);
                    }

                    // do binary search on extent indeces
                    let ei_vec_slice = ei_vec.as_slice();
                    curr_path.p_idx = match ext_binary_search_idx(ei_vec_slice, ei_vec.len() as u32, idx as u32) {
                        Some(ei_idx) => Some(Arc::new(ei_vec[ei_idx as usize])),
                        None => {
                            println!("Error in index node binary search");
                            return Err(libc::EIO);
                        },
                    };


                }
                curr_path.p_depth = curr_depth;

                curr_depth += 1;

                // if all data is stored in root extent, we can stop early and avoid unnecessary operations
                if curr_depth >= max_depth {
                    break;
                }
                // update ppos and read in the next block
                ppos += 1;

                
                let mut next_path = Xv6fsExtentPath::new();
                // set buffer head and extent header of new node block.
                next_path.p_bh = Some(Arc::new(disk.bread(curr_path.p_ext.unwrap().xe_block_addr as u64)?));
                next_path.p_hdr = Some(Arc::new(bh_to_ext_header(&next_path)?)); 
                ext_path_vec.push(next_path);
            } else { // curr_depth >= max_depth
                break;
            }
        }
              
        // now curr_depth == max_depth
        // do binary search on leaf node, and return block num if found, otherwise need to allocate new block
        if curr_depth == 0 {
            // all data is stored in the extents in the root node

            // ppos == 0 and curr_depth == 0
            let curr_path = epv_mut.get_mut(ppos).unwrap();
            let curr_ext = curr_path.p_ext.unwrap();

            // case 1: block is alrady allocated
            if idx >= curr_ext.xe_block as usize && idx < (curr_ext.xe_block + curr_ext.xe_len as u32) as usize {
                return Ok(curr_ext.xe_block_addr + (idx as u32 - curr_ext.xe_block as u32));
            }

            // case 2: block is not allocated
            // allocate new block
            
            let h = match handle {
                Some(_) => handle.unwrap(),
                None => new_tx.get_or_insert_with(|| self.log.as_ref().unwrap().begin_op(2)),
            };
            let new_block_addr = self.balloc(h)?;
            let new_block = get_next_block_idx(inode.size);

            // case 2a: new block is contiguous
            if new_block - 1 == curr_ext.xe_block_addr + curr_ext.xe_len as u32 && curr_ext.xe_len < u16::MAX {
                // TODO: might also want to check for max file size?
                // Can extend current extent
                curr_ext.xe_len += 1;
                return Ok(new_block_addr);
           } else { // block is not contiguous or the previous extent reached its maximum length
                // create new extent
                let new_ext = Xv6fsExtent::new();
                new_ext.xe_block = new_block;
                new_ext.xe_block_addr = new_block_addr;
                new_ext.xe_len = 1;

                let root_entries = curr_path.p_hdr.unwrap().eh_entries;
                let root_max_entries = ext_max_entries(inode, 0);
                // check if there's enough space in the root extent array
                if root_entries < root_max_entries {
                    // need to insert in sorted order
                    //inode.ee_arr[root_entries as usize] = new_ext;
                    let re_map: BTreeMap<u32, Xv6fsExtent> = get_sorted_root_ext(inode, new_ext)?;
                    insert_to_root_ext_sorted(inode, &re_map)?;
                    
                    return Ok(new_ext.xe_block_addr);
                } else { // not enough space
                    // need to grow in depth
                    return self.grow_extent_root(inode, &new_ext, handle, new_tx);
                }
            }

        } else { // depth != 0
            // we are at a leaf node

            // curr_depth != 0 && ppos point to leaf extent node
            let curr_path = epv_mut.get_mut(ppos).unwrap();
            let bh = match curr_path.p_bh {
                Some(bh) => Arc::get_mut(&mut bh).unwrap(),
                None => { 
                    println!("Leaf extent node - cannot get p_bh");
                    return Err(libc::EIO);
                }
            };
            let bh_slice = bh.data_mut();

            let ext_entries = match curr_path.p_hdr {
                Some(hdr) => hdr.eh_entries,
                None => {
                    println!("Leaf extent node - cannot get eh_entries");
                    return Err(libc::EIO);
                }
            };

            let ext_vec: Vec<Xv6fsExtent> = Vec::with_capacity(ext_entries as usize);
            for ext_off in (eh_len..eh_len + (ext_len * ext_entries as usize)).step_by(ext_len) {
                if ext_off >= BSIZE {
                    break;
                }
                
                let mut ext = Xv6fsExtent::new();
                let ext_slice = &mut bh_slice[ext_off..ext_off + ext_len];
                ext.extract_from(ext_slice).map_err(|_| libc::EIO)?;

                // check for invalid extent
                // TODO: might want to do something else than just breaking out of the loop
                if ext.xe_block_addr == 0 {
                    break;
                }
                ext_vec.push(ext);
            }
            
            // do binary search in leaf node
            let ext_vec_slice = ext_vec.as_slice();
            let curr_ext = match ext_binary_search(ext_vec_slice, ext_vec.len() as u32, idx as u32) {
                Some(ext_idx) => ext_vec[ext_idx as usize],
                None => {
                    println!("Root extent node - ext_binary_search failed");
                    return Err(libc::EIO);
                }
            };
            // case 1: block is already allocated
            if idx >= curr_ext.xe_block as usize && idx < (curr_ext.xe_block + curr_ext.xe_len as u32) as usize {
                return Ok(curr_ext.xe_block_addr + (idx as u32 - curr_ext.xe_block as u32));
            }

            // case 2: block is not allocated
            // TODO: move to helper function
            let h = match handle {
                Some(_) => handle.unwrap(),
                None => new_tx.get_or_insert_with(|| self.log.as_ref().unwrap().begin_op(2)),
            };
            let new_block_addr = self.balloc(h)?;
            let new_block = get_next_block_idx(inode.size);

            if new_block - 1 == curr_ext.xe_block_addr + curr_ext.xe_len as u32 && curr_ext.xe_len < u16::MAX {
                // TODO: might also want to check for max file size?
                curr_ext.xe_len += 1;
                return Ok(new_block_addr);
            } else { // new block is not contiguous or curr_ext.len >= u16::MAX
                let new_ext = Xv6fsExtent::new();
                new_ext.xe_block = new_block;
                new_ext.xe_block_addr = new_block_addr;
                new_ext.xe_len = 1;
                // sort extents and insert new extent
                let le_map: BTreeMap<u32, Xv6fsExtent> = BTreeMap::new();
                while let Some(leaf_ext) = ext_vec.pop() {
                    le_map.insert(leaf_ext.xe_block, leaf_ext);
                }
                le_map.insert(new_ext.xe_block, new_ext);

                // try inserting in current leaf node if there's space
                let leaf_max_entries = ext_max_entries(inode, curr_depth);
                if ext_vec.len() < leaf_max_entries as usize { // enough space in current leaf node
                    h.get_write_access(&bh);


                    let h = match handle {
                        Some(_) => handle.unwrap(),
                        None => new_tx.get_or_insert_with(|| self.log.as_ref().unwrap().begin_op(5)),         
                    };
                    // write extents into leaf node

                    let le_map_keys: Vec<u32> = le_map.keys().cloned().collect();
                    let ext_off = EH_LEN;
                    let num_leaf_ext = le_map_keys.len();
                    for key_idx in 0..num_leaf_ext {
                        let ext_slice = &mut bh_slice[ext_off..ext_off + EXT_LEN];
                        le_map.get(&le_map_keys[key_idx]).unwrap().dump_into(ext_slice).map_err(|_| libc::EIO)?;
                        ext_off += EH_LEN;
                    }
                    // udpate lead extent header
                    let leaf_hdr = curr_path.p_hdr.unwrap();
                    leaf_hdr.eh_entries = num_leaf_ext as u16;
                    let hdr_slice = &mut bh_slice[0..EH_LEN];
                    leaf_hdr.dump_into(hdr_slice).map_err(|_| libc::EIO)?;
                    h.journal_write(&mut bh);
                    
                    return Ok(new_ext.xe_block_addr);
                } else { // not enough space in current leaf node
                    // split leaf node

                    // allocate new block for new leaf node.
                    let h = match handle {
                        Some(_) => handle.unwrap(),
                        None => new_tx.get_or_insert_with(|| self.log.as_ref().unwrap().begin_op(5)),
                    };
                    // allocate new leaf block and split extents in old and new leaf blocks & update extents headers of leaf blocks
                    let new_leaf_block_addr = self.balloc(h)?;
                    let lower_bounds = self.insert_ext_to_leaf_nodes(&le_map, handle, new_tx, 0, new_leaf_block_addr, Some(*curr_path.p_bh.unwrap()))?;

                    // insert new extent "index"
                    if curr_depth == 1 {    // root has extents used as extent indeces
                        let re_num = inode.eh.eh_entries;
                        //le re_map 
                        let mut new_root_ext = Xv6fsExtent::new();
                        // update values of new root extent
                        new_root_ext.xe_block = lower_bounds.1;
                        new_root_ext.xe_block_addr = new_leaf_block_addr;
                        // update value of original root extent
                        ext_path_vec[0].p_ext.unwrap().xe_block = lower_bounds.0;
                        let re_map: BTreeMap<u32, Xv6fsExtent> = get_sorted_root_ext(inode, new_root_ext)?;
                        // sort root extents and insert new "index" (root contains only extents) extent into root
                        if re_map.len() <= INEXTENTS as usize {
                            insert_to_root_ext_sorted(inode, &re_map)?;
                            return Ok(new_ext.xe_block_addr);
                        } else {    // root has no more space, need to grow in depth and add two extent index nodes

                        }
                    } else { // general case where above node has extent indeces

                    }
                   // get extent indeces from parent node
                    // insert new index node.
                        // case 1. sufficient space in parent node
                        // case 2. need to split parent node.
                            // case 2a. parent is root node
                            // case 2b. parent is any index node where depth != 0

                }

            }
            // case 2a: new block is contiguous, need to increase xe_len 
            // case 2a1: xe_len < max_xe_len
            // case 2a2: xe_len > max_xe_len, need to split a single extent
                // case: if # extent > max_ext_num, need to split extent node
                    // new index node
                    // case a: index node fits
                    // case b: need to add a new pointer (ext or ext index depending on depth) in previous node.
            // 2b: new block is not contiguous, new to store in new extent
            // 2ba: need to find new correct location again and add it in extent node
        }
        /*
        if idx < NDIRECT as usize {
            let addr = inode.addrs.get_mut(idx).ok_or(libc::EIO)?;
            if *addr == 0 {
                let h = match handle {
                    Some(_) => handle.unwrap(),
                    None => new_tx.get_or_insert_with(|| self.log.as_ref().unwrap().begin_op(2)),
                };
                return self.balloc(h).map(|blk_id| {
                    *addr = blk_id;
                    blk_id
                });
            }
            return Ok(*addr);
        }
        */
/*
        idx -= NDIRECT as usize;
        if idx < NINDIRECT as usize {
            // indirect block
            let ind_blk_id = inode.addrs.get_mut(NDIRECT as usize).ok_or(libc::EIO)?;
            if *ind_blk_id == 0 {
                let h = match handle {
                    Some(_) => handle.unwrap(),
                    None => new_tx.get_or_insert_with(|| self.log.as_ref().unwrap().begin_op(5)),
                };
                self.balloc(h).map(|blk_id| {
                    *ind_blk_id = blk_id;
                })?;
            }

            let result_blk_id: u32;
            let disk = self.disk.as_ref().unwrap();
            let mut bh = disk.bread(*ind_blk_id as u64)?;
            let b_data = bh.data();

            let mut cell_data = [0; 4];
            let cell_segment = &b_data[idx * 4 .. (idx + 1) * 4];
            cell_data.copy_from_slice(cell_segment);
            let cell = u32::from_ne_bytes(cell_data);
            if cell == 0 {
                // need to allocate blk
                let h = match handle {
                    Some(_) => handle.unwrap(),
                    None => new_tx.get_or_insert_with(|| self.log.as_ref().unwrap().begin_op(3)),
                };
                h.get_write_access(&bh);
                let b_data = bh.data_mut();
                let cell_segment = &mut b_data[idx * 4 .. (idx + 1) * 4];

                result_blk_id = self.balloc(h)?;
                let blk_data = result_blk_id.to_ne_bytes();
                cell_segment.copy_from_slice(&blk_data);
                h.journal_write(&mut bh);
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
                let h = match handle {
                    Some(_) => handle.unwrap(),
                    None => new_tx.get_or_insert_with(|| self.log.as_ref().unwrap().begin_op(2)),
                };
                self.balloc(h).map(|blk_id| {
                    *dind_blk_id = blk_id;
                })?;
            }

            let disk = self.disk.as_ref().unwrap();
            let mut bh = disk.bread(*dind_blk_id as u64)?;
            let b_data = bh.data();
            let dind_idx = idx / NINDIRECT as usize;

            let mut cell_data = [0; 4];
            let cell_segment = &b_data[dind_idx * 4 .. (dind_idx + 1) * 4];
            cell_data.copy_from_slice(cell_segment);
            let cell = u32::from_ne_bytes(cell_data);

            if cell == 0 {
                let h = match handle {
                    Some(_) => handle.unwrap(),
                    None => new_tx.get_or_insert_with(|| self.log.as_ref().unwrap().begin_op(6)),
                };
                h.get_write_access(&bh);
                let b_data = bh.data_mut();
                let cell_segment = &mut b_data[dind_idx * 4 .. (dind_idx + 1) * 4];

                let result_blk_id = self.balloc(h)?;
                let result_blk_data = result_blk_id.to_ne_bytes();
                cell_segment.copy_from_slice(&result_blk_data);
                h.journal_write(&mut bh);
            }

            let mut dbh = disk.bread(cell as u64)?;
            let db_data = dbh.data();
            let dblock_idx = idx % NINDIRECT as usize;

            let result_blk_id: u32;
            let mut dcell_data = [0; 4];
            let dcell_segment = &db_data[dblock_idx * 4 .. (dblock_idx + 1) * 4];
            dcell_data.copy_from_slice(dcell_segment);
            let dcell = u32::from_ne_bytes(dcell_data);
            if dcell == 0 {
                let h = match handle {
                    Some(_) => handle.unwrap(),
                    None => new_tx.get_or_insert_with(|| self.log.as_ref().unwrap().begin_op(3)),
                };
                h.get_write_access(&dbh);
                let db_data = dbh.data_mut();
                let dcell_segment = &mut db_data[dblock_idx * 4 .. (dblock_idx + 1) * 4];

                result_blk_id = self.balloc(h)?;
                let result_blk_data = result_blk_id.to_ne_bytes();
                dcell_segment.copy_from_slice(&result_blk_data);
                h.journal_write(&mut dbh);
            } else {
                result_blk_id = dcell;
            }
            return Ok(result_blk_id);
        }
*/
        return Err(libc::EIO);
    }

    // TODO: check presence of extent tree for large files.
    //      - if there is no tree, then iterate of extents in inode, and free corrresponding blocks.
    //      - if there is a tree, then we need to traverse the entire tree, and start freeing from data blocks, 
    //        and backtract to leaf nodes, index nodes, and finally root. 
    pub fn itrunc(&self, inode: &mut CachedInode, internals: &mut InodeInternal, handle: &Handle) -> Result<(), libc::c_int> {
        // check for presence of extent tree
        let ext_tree_depth = internals.eh.eh_depth; 
        let num_extents = internals.eh.eh_entries;

        if ext_tree_depth <= 0 {
            // TODO replace with a free_leaf function
            // iterate over extent entries and free blocks
            for i in 0..num_extents {
                let mut curr_ext = internals.ee_arr.get_mut(i).ok_or(libc::EIO)?;
                let block_no = curr_ext.xe_block;
                let num_blocks = curr_ext.xe_len;
                for curr_block in block_no..(block_no + num_blocks * BSIZE).step_by(BSIZE) {
                   self.bfree(*curr_block as usize, handle)?; 
                }
                curr_ext.clear();

            }
    
        } /*else { // tree has depth > 0
            // need to traverse the extent tree and free all index/leaf/data nodes
            let disk = self.disk.as_ref().unwrap(); 
            // stack that contains the addresses stored in each extent or extent index
            let addr_stack: Vec<u32> = Vec::with_capacity(64);
            // create stack iterate over the extents in the root node
            for i in 0..num_extents {
                // TODO can create macros to get sizes of extents types.
                let eh_len = mem::size_of::<Xv6fsExtentHeader>();
                let ei_len = mem::size_of::<Xv6fsExtentIdx>();
                let ext_len = mem::size_of::<Xv6fsExtent>();

                let mut curr_ext = internals.ee_arr.get_mut(i).ok_or(libc::EIO)?;

                let curr_root_addr = curr_e;
                addr_stack.push_back(curr_ext.xe_block);
                // need to add outer while loop
                let disk = self.disk.as_ref().unwrap();
                let blk_idx = curr_ext.xe_block;
                if (blk_idx != 0) {
                    let bh = disk.bread(blk_idx as u64)?;
                    let b_data = bh.data();

                    // extract extent header from block
                    let mut eh = Xv6fsExtentHeader::new();
                    let eh_slice = &mut b_data[0..eh_len];
                    eh.extract_from(eh_slice).map_err(|_| libc::EIO)?;

                    if (eh.depth == 0) {
                        // it is a leaf node, so free blocks pointed by extents
                        for ext_off in (eh_len..eh_len + (ext_len * eh.eh_entries)).step_by(ext_len) {
                            if ext_off >= BSIZE { // should never happen
                                break;
                            }
                            let mut ext = Xv6fsExtent::new();
                            let ext_slice = &mut b_data[ext_off..ext_off + ext_len];
                            ext.extract_from(ext_slice).map_err(|_| libc::EIO)?;
                            
                            let ext_block = ext_block;
                            if ext_block != 0 {
                                for bfree_idx in (ext_block..ext_block + ext.xe_len) {
                                    self.bfree(bfree_idx as usize, handle)?;
                                }
                            }
                        }
                    } else { // eh.depth > 0
                        addr_stack.push_back()
                    }
                }
                /* for each root extent, add block address to stack,
                    for each block, check header:
                        - if current block is leaf block, then iterate over extents and free all data blocks
                        - if current block is index block, then push block address to stack, and continue to next loop iteration
                    at the end, pop address from stack, and bfree the block
                let mut curr_ext = internals.ee_arr.get_mut(i)_ok_or(libs::EIO)?;
                */

                // remove block address from stack, free block and clear root extent
                let root_child_block = addr_stack.pop_back();
                self.bfree(root_child_block as usize, handle)?;
                curr_ext.clear();

            }
        }
        */
        /*
        for i in 0..NDIRECT as usize {
            let addr = internals.addrs.get_mut(i).ok_or(libc::EIO)?;
            if *addr != 0 {
                self.bfree(*addr as usize, handle)?;
                *addr = 0;
            }
        }
        */
        

        // old indirect blocks
        /*
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
                    self.bfree(addr as usize, handle)?;
                }
            }
            self.bfree(*ind_blk_id as usize, handle)?;
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
                            self.bfree(daddr as usize, handle)?;
                        }
                    }
                    self.bfree(ind_blk_id as usize, handle)?;
                    ind_region.copy_from_slice(&[0; 4]);
                }
            }
            self.bfree(*dind_blk_id as usize, handle)?;
            *dind_blk_id = 0;
        }
*/
        internals.size = 0;
        return self.iupdate(&internals, inode.inum, handle);
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
            m = min(n - tot, BSIZE - off % BSIZE);
            let block_no = self.bmap(internals, off / BSIZE, None)?;
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
        handle: &Handle
    ) -> Result<usize, libc::c_int> {
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
                let m = min(off - start_off, BSIZE - start_off % BSIZE);
                let block_no = self.bmap(internals, start_off / BSIZE, Some(handle))?;
                let disk = self.disk.as_ref().unwrap();
                let mut bh = disk.bread(block_no as u64)?;
                handle.get_write_access(&bh);
    
                let b_data = bh.data_mut();

                for i in start_off..start_off + m {
                    let idx = b_data.get_mut(i % BSIZE).ok_or(libc::EIO)?;
                    *idx = 0;
                }
                written_blocks += 1;
                handle.journal_write(&mut bh);
    
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
            let m = min(n - tot, BSIZE - off % BSIZE);
            let block_no = self.bmap(internals, off / BSIZE, Some(handle))?;

            let disk = self.disk.as_ref().unwrap();
            let mut bh = disk.bread(block_no as u64)?;

            handle.get_write_access(&bh);
    
            let data_slice = bh.data_mut();
            let data_off = off % BSIZE;
            let data_region = &mut data_slice[data_off..data_off + m];

            let copy_region = &buf[src..src + m];
            data_region.copy_from_slice(copy_region);
            handle.journal_write(&mut bh);
            written_blocks += 1;

            tot += m;
            off += m;
            src += m;
            end_size = off;
        }

        if n > 0 && end_size > i_size {
            internals.size = end_size as u64;
            self.iupdate(internals, inum, handle)?;
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
        // Check if inode is directory
        if internals.inode_type != T_DIR {
            return Err(libc::ENOTDIR);
        }
        let hroot_len = mem::size_of::<Htree_root>();
        let hindex_len = mem::size_of::<Htree_index>();
        let hentry_len = mem::size_of::<Htree_entry>();
        let de_len = mem::size_of::<Xv6fsDirent>();
        let disk = self.disk.as_ref().unwrap();

        let search_name = match name.to_str() {
            Some(s) => s,
            None => {
                return Err(libc::ENOENT);
            },
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
        let target_hash = calculate_hash(&osstr_name);

        // read in entire root block
        let root_block_no = self.bmap(internals, 0, None)?;
        let mut root_bh = disk.bread(root_block_no as u64)?;
        let root_arr_slice = root_bh.data_mut();

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
            let mut rie = Htree_entry::new();
            let rie_slice = &mut root_arr_slice[hroot_len + (hentry_len * rie_idx as usize)
                ..hroot_len + (hentry_len * (rie_idx as usize + 1))];
            rie.extract_from(rie_slice).map_err(|_| libc::EIO)?;
            if rie.block == 0 {
                break;
            }
            index_vec.push(rie);
        }

        // look for lowerbound of correct index node
        let ind_slice = index_vec.as_slice();
        let target_entry = match find_lowerbound(ind_slice, index_vec.len(), target_hash) {
            Some(index) => index,
            None => {
                return Err(libc::ENOENT);
            }
        };

        // get index block
        let target_lblock: u32 = index_vec[target_entry].block;
        let hindex_block_no = self.bmap(internals, target_lblock as usize, None)?;
        let mut hindex_bh = disk.bread(hindex_block_no as u64)?;
        let hindex_arr_slice = hindex_bh.data_mut();

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

        // get lowerbound of correct leaf node
        let leaf_slice = leaf_vec.as_slice();
        let target_leaf = match find_lowerbound(leaf_slice, leaf_vec.len(), target_hash) {
            Some(index) => index,
            None => {
                return Err(libc::ENOENT)
            },
        };

        // read leafnode
        let leaf_idx = leaf_vec[target_leaf].block;
        let leaf_block_no = self.bmap(internals, leaf_idx as usize, None)?;
        let mut leaf_bh = disk.bread(leaf_block_no as u64)?;
        let leaf_arr_slice = leaf_bh.data_mut();

        // look through the entries in the leafnode
        for de_idx in 0..BSIZE / de_len {
            let mut de = Xv6fsDirent::new();
            let de_slice = &mut leaf_arr_slice[de_idx * de_len..(de_idx + 1) * de_len];
            de.extract_from(de_slice).map_err(|_| libc::EIO)?;

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

        return Err(libc::ENOENT);
    }

    // create subdirectory with 'name' under the directory pointed to by 'internals'
    pub fn dirlink(
        &self,
        internals: &mut InodeInternal,
        name: &OsStr,
        child_inum: u32,
        parent_inum: u32,
        handle: &Handle,
    ) -> Result<usize, libc::c_int> {
        // check if inode is directory
        if internals.inode_type != T_DIR {
            return Err(libc::ENOTDIR);
        }

        let hroot_len = mem::size_of::<Htree_root>();
        let hindex_len = mem::size_of::<Htree_index>();
        let hentry_len = mem::size_of::<Htree_entry>();
        let de_len = mem::size_of::<Xv6fsDirent>();
        let disk = self.disk.as_ref().unwrap();

        //let mut hroot_arr_vec: Vec<u8> = vec![0; BSIZE];

        let search_name = match name.to_str() {
            Some(s) => s,
            None => {
                return Err(libc::ENOENT);
            }
        };

        let root_block_no = self.bmap(internals, 0, None)?;
        let mut root_bh = disk.bread(root_block_no as u64)?;
        let root_arr_slice = root_bh.data_mut();

        // extract root of dir
        let mut root = Htree_root::new();
        let root_slice = &mut root_arr_slice[0..hroot_len];
        root.extract_from(root_slice).map_err(|_| libc::EIO)?;
        let num_indeces = root.ind_entries;
        let mut num_blocks = root.blocks as usize;

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
        let target_hash = calculate_hash(&osstr_name);

        // new directory, create root node
        if search_name == "." {
            root.ind_entries = 0;
            root.blocks = 1;
            root.dump_into(root_slice).map_err(|_| libc::EIO)?;
            let root_de_slice = &mut root_slice[0..de_len];
            de.dump_into(root_de_slice).map_err(|_| libc::EIO)?;

            if self.writei(root_slice, 0, hroot_len, internals, parent_inum, handle)? != hroot_len {
                return Err(libc::EIO);
            }

            return Ok(0);
        } else if search_name == ".." {
            let root_de_slice = &mut root_slice[de_len..2 * de_len];
            de.dump_into(root_de_slice).map_err(|_| libc::EIO)?;
            if self.writei(root_de_slice, de_len, de_len, internals, parent_inum, handle)? != de_len {
                return Err(libc::EIO);
            }

            return Ok(0);
        }

        // regular dirent
        de.dump_into(de_slice).map_err(|_| libc::EIO)?;
        // directory is empty
        if num_indeces == 0 {
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

            rie.dump_into(rie_slice).map_err(|_| libc::EIO)?;
            if self.writei(rie_slice, rie_offset, hentry_len, internals, parent_inum, handle)? != hentry_len
            {
                return Err(libc::EIO);
            }

            // create index node
            let mut index = Htree_index::new();
            let mut index_vec: Vec<u8> = vec![0; hindex_len];
            let index_slice = index_vec.as_mut_slice();
            index.entries = 1 as u32;

            index.dump_into(index_slice).map_err(|_| libc::EIO)?;

            if self.writei(
                index_slice,
                index_offset,
                hindex_len,
                internals,
                parent_inum,
                handle,
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

            ine.dump_into(ine_slice).map_err(|_| libc::EIO)?;

            if self.writei(ine_slice, ine_offset, hentry_len, internals, parent_inum, handle)? != hentry_len
            {
                return Err(libc::EIO);
            }

            // write dirent to leafnode
            if self.writei(de_slice, de_offset, de_len, internals, parent_inum, handle)? != de_len {
                return Err(libc::EIO);
            }

            // update root info
            root.depth = 2;
            root.ind_entries = 1;
            root.blocks = 3;

            root.dump_into(root_slice).map_err(|_| libc::EIO)?;
            if self.writei(root_slice, 0, hroot_len, internals, parent_inum, handle)? != hroot_len {
                return Err(libc::EIO);
            }

            return Ok(0);
        }

        // directory is not empty

        // Add index entries in root node to a vec for binary search
        let mut index_vec: Vec<Htree_entry> = Vec::with_capacity((num_indeces + 1) as usize);
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

        // case: new hash < lowest hash value in root entries
        // need to add new index entry in root, create a new index node, and create a leafnode
        if target_hash < index_vec[0].name_hash {
            // directory reached maximum blocks allowed
            if num_blocks >= HTREE_MAXBLOCKS as usize - 2 {
                return Err(libc::ENOENT);
            }
            let index_offset = num_blocks * BSIZE;
            let ine_offset = index_offset + hindex_len;
            let de_offset = (num_blocks + 1) * BSIZE;

            // create index node
            let mut index = Htree_index::new();
            let mut index_bvec: Vec<u8> = vec![0; hindex_len];
            let index_slice = index_bvec.as_mut_slice();
            index.entries = 1 as u32;
            index.dump_into(index_slice).map_err(|_| libc::EIO)?;

            if self.writei(
                index_slice,
                index_offset,
                hindex_len,
                internals,
                parent_inum,
                handle,
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
            ine.dump_into(ine_slice).map_err(|_| libc::EIO)?;

            if self.writei(ine_slice, ine_offset, hentry_len, internals, parent_inum, handle)? != hentry_len
            {
                return Err(libc::EIO);
            }

            // write dirent to leafnode
            if self.writei(de_slice, de_offset, de_len, internals, parent_inum, handle)? != de_len {
                return Err(libc::EIO);
            }

            // add new index entry to previous entries in root node in sorted order
            let mut rie = Htree_entry::new();
            rie.name_hash = target_hash;
            rie.block = (index_offset / BSIZE) as u32;

            let mut index_vec_rev: Vec<Htree_entry> = Vec::with_capacity(index_vec.len());
            while let Some(hen) = index_vec.pop() {
                index_vec_rev.push(hen);
            }
            index_vec_rev.push(rie);

            let mut rie_idx = 0;
            while let Some(rie) = index_vec_rev.pop() {
                let mut rie_vec: Vec<u8> = vec![0; hentry_len];
                let rie_slice = rie_vec.as_mut_slice();
                rie.dump_into(rie_slice).map_err(|_| libc::EIO)?;
                let offset = hroot_len + rie_idx * hentry_len;
                if self.writei(rie_slice, offset, hentry_len, internals, parent_inum, handle)? != hentry_len
                {
                    return Err(libc::EIO);
                }

                rie_idx += 1;
            }

            // update root info
            let root2_slice = &mut root_arr_slice[0..hroot_len];
            root.ind_entries += 1;
            root.blocks += 2;
            root.dump_into(root2_slice).map_err(|_| libc::EIO)?;
            if self.writei(root2_slice, 0, hroot_len, internals, parent_inum, handle)? != hroot_len {
                return Err(libc::EIO);
            }
            return Ok(0);
        }

        // look for correct lowerbound for index node
        let ind_slice = index_vec.as_slice();
        let target_entry = match find_lowerbound(ind_slice, index_vec.len(), target_hash) {
            Some(index) => index,
            None => {
                return Err(libc::ENOENT);
            },
        };

        // read entire index block
        let target_lblock: u32 = index_vec[target_entry].block;
        let hindex_block_no = self.bmap(internals, target_lblock as usize, None)?;
        let mut hindex_bh = disk.bread(hindex_block_no as u64)?;
        let hindex_arr_slice = hindex_bh.data_mut();

        // get index header
        let mut index = Htree_index::new();
        let hindex_slice = &mut hindex_arr_slice[0..hindex_len];
        index.extract_from(hindex_slice).map_err(|_| libc::EIO)?;

        // create vec of entries in index node for binary search
        let num_entries = index.entries;
        let mut leaf_vec: Vec<Htree_entry> = Vec::with_capacity((num_entries + 1) as usize);
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
            leaf_vec.push(hentry);
        }

        // get correct lowerbound for leaf node
        let leaf_slice = leaf_vec.as_slice();
        let target_leaf = match find_lowerbound(leaf_slice, leaf_vec.len(), target_hash) {
            Some(index) => index,
            None => {
                return Err(libc::ENOENT);
            },
        };

        // read entire leaf node block
        let leaf_idx = leaf_vec[target_leaf].block;
        let leaf_block_no = self.bmap(internals, leaf_idx as usize, None)?;
        let mut leaf_bh = disk.bread(leaf_block_no as u64)?;
        let leaf_arr_slice = leaf_bh.data_mut();

        // look for an entry space in leafnode
        let mut final_off = None;
        for de_idx in 0..BSIZE / de_len {
            let mut de_temp = Xv6fsDirent::new();
            let de_slice_temp = &mut leaf_arr_slice[de_idx * de_len..(de_idx + 1) * de_len];
            de_temp.extract_from(de_slice_temp).map_err(|_| libc::EIO)?;
            if de_temp.inum == 0 {
                final_off = Some((leaf_idx as usize * BSIZE + de_idx * de_len) as u64);
            }

            // there is enough space in the leaf node
            if final_off.is_some() {
                let final_off = final_off.unwrap();
                if self.writei(de_slice, final_off as usize, de_len, internals, parent_inum, handle)?
                    != de_len
                {
                    return Err(libc::EIO);
                }
                return Ok(0);
            }
        }

        // not enough space in the current leaf node
        // need to split leaf nodes

        let mut de_map: BTreeMap<u32, Vec<Xv6fsDirent>> = BTreeMap::new();

        // add dirents with their hash value to map for sorting
        for de_idx in 0..BSIZE / de_len {
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
            let de_name_osstr = OsStr::new(de_name);
            let de_hash = calculate_hash(&de_name_osstr);

            if !de_map.contains_key(&de_hash) {
                de_map.insert(de_hash, Vec::with_capacity(3));
            }
            if let Some(x) = de_map.get_mut(&de_hash) {
                x.push(de_temp);
            }
        }

        if !de_map.contains_key(&target_hash) {
            de_map.insert(target_hash, Vec::with_capacity(3));
        }
        if let Some(x) = de_map.get_mut(&target_hash) {
            x.push(de);
        }

        // get the new lower bounds for the leaf nodes
        let mut keys: Vec<_> = de_map.keys().cloned().collect();
        let keys2 = keys.split_off(keys.len() / 2);
        let mut de_map2 = de_map.split_off(&keys2[0]);
        let leaf2_lower = keys2[0];

        // get vecs from the dirents maps
        let mut leaf1_dir_vec: Vec<Xv6fsDirent> = Vec::with_capacity(66);
        let mut leaf2_dir_vec: Vec<Xv6fsDirent> = Vec::with_capacity(66);

        {
            for key in keys {
                if let Some(mut val) = de_map.remove(&key) {
                    while let Some(dirent) = val.pop() {
                        leaf1_dir_vec.push(dirent);
                    }
                }
            }
        }
        {
            for key in keys2 {
                if let Some(mut val) = de_map2.remove(&key) {
                    while let Some(dirent) = val.pop() {
                        leaf2_dir_vec.push(dirent);
                    }
                }
            }
        }

        // keep half dirents into current leaf node
        {
            let mut leaf_vec: Vec<u8> = vec![0; BSIZE];
            let leaf_slice = leaf_vec.as_mut_slice();
            let mut idx = 0;
            while let Some(de) = leaf1_dir_vec.pop() {
                let leaf_idx_slice = &mut leaf_slice[idx * de_len..(idx + 1) * de_len];
                de.dump_into(leaf_idx_slice).map_err(|_| libc::EIO)?;
                idx += 1;
            }

            // need overwrite prev dirents with 0's
            let write_size = BSIZE;
            if self.writei(
                leaf_slice,
                leaf_idx as usize * BSIZE,
                write_size,
                internals,
                parent_inum,
                handle,
            )? != write_size
            {
                return Err(libc::EIO);
            }
        }

        // directory reached max blocks allowed
        if num_blocks >= HTREE_MAXBLOCKS as usize - 1 {
            return Err(libc::ENOENT);
        }

        // write other half into a new leafnode
        {
            let mut leaf_vec: Vec<u8> = vec![0; BSIZE];
            let leaf_slice = leaf_vec.as_mut_slice();
            let mut idx = 0;

            while let Some(de) = leaf2_dir_vec.pop() {
                let leaf_idx_slice = &mut leaf_slice[idx * de_len..(idx + 1) * de_len];
                de.dump_into(leaf_idx_slice).map_err(|_| libc::EIO)?;
                idx += 1;
            }

            // zero out possible trash values
            let write_size = BSIZE;
            if self.writei(
                leaf_slice,
                num_blocks * BSIZE,
                write_size,
                internals,
                parent_inum,
                handle,
            )? != write_size
            {
                return Err(libc::EIO);
            }
        }

        // create a new hentry in the index node
        let num_entries = index.entries as usize;

        // sort old entries in index node
        let mut ie_map: BTreeMap<u32, Htree_entry> = BTreeMap::new();
        while let Some(ie) = leaf_vec.pop() {
            ie_map.insert(ie.name_hash, ie);
        }

        let mut new_ie = Htree_entry::new();
        new_ie.name_hash = leaf2_lower;
        new_ie.block = num_blocks as u32;
        ie_map.insert(new_ie.name_hash, new_ie);

        // store ie's in reverse order [10, 9, 8, ..]
        {
            let mut keys: Vec<_> = ie_map.keys().cloned().collect();
            while let Some(key) = keys.pop() {
                if let Some(val) = ie_map.remove(&key) {
                    leaf_vec.push(val);
                }
            }
        }

        // enough space in current index node
        if num_entries < ((BSIZE - hindex_len) / hentry_len) {
            // insert in sorted order
            let mut index_vec: Vec<u8> = vec![0; BSIZE];
            let index_slice = index_vec.as_mut_slice();
            index.entries += 1;
            let index_header_slice = &mut index_slice[0..hindex_len];
            index.dump_into(index_header_slice).map_err(|_| libc::EIO)?;

            let mut ie_idx = 0;
            while let Some(ie) = leaf_vec.pop() {
                let ie_slice = &mut index_slice[hindex_len + ie_idx as usize * hentry_len
                    ..hindex_len + (ie_idx + 1) as usize * hentry_len];
                ie.dump_into(ie_slice).map_err(|_| libc::EIO)?;
                ie_idx += 1;
            }

            // zero out any possible trash value
            let write_size = BSIZE;
            if self.writei(
                index_slice,
                target_lblock as usize * BSIZE,
                write_size,
                internals,
                parent_inum,
                handle,
            )? != write_size
            {
                return Err(libc::EIO);
            }

            // update root info
            let root2_slice = &mut root_arr_slice[0..hroot_len];
            root.blocks += 1;
            root.dump_into(root2_slice).map_err(|_| libc::EIO)?;
            if self.writei(root2_slice, 0, hroot_len, internals, parent_inum, handle)? != hroot_len {
                return Err(libc::EIO);
            }
        } else {
            // not enough space, need to split root index nodes

            // directory reached maximum blocks allowed
            if num_blocks >= HTREE_MAXBLOCKS as usize - 2 {
                return Err(libc::ENOENT);
            }

            if (root.ind_entries as usize) < (BSIZE - hroot_len) / hentry_len {
                // enough space for new index entry in root node

                let mut new_rie = Htree_entry::new();
                let mut new_index = Htree_index::new();

                // leaf vec is in reverse order (i.e. 10, 9, 8, .. )
                // split original index node and update original index node
                // ie2_vec contains lower entries
                let mut ie2_vec = leaf_vec.split_off(leaf_vec.len() / 2);
                {
                    index.entries = ie2_vec.len() as u32;
                    let mut index_vec: Vec<u8> = vec![0; BSIZE];
                    let index_slice = index_vec.as_mut_slice();
                    let index_header_slice = &mut index_slice[0..hindex_len];
                    index.dump_into(index_header_slice).map_err(|_| libc::EIO)?;
                    let mut ie_idx = 0;
                    while let Some(ie) = ie2_vec.pop() {
                        let ie_slice = &mut index_slice[hindex_len + ie_idx as usize * hentry_len
                            ..hindex_len + (ie_idx + 1) as usize * hentry_len];
                        ie.dump_into(ie_slice).map_err(|_| libc::EIO)?;
                        ie_idx += 1;
                    }

                    // need to clear out possible trash values
                    let write_size = BSIZE;
                    if self.writei(
                        index_slice,
                        target_lblock as usize * BSIZE,
                        write_size,
                        internals,
                        parent_inum,
                        handle,
                    )? != write_size
                    {
                        return Err(libc::EIO);
                    }
                }

                // create new index node with the remaining entries for root indeces

                new_index.entries = leaf_vec.len() as u32;
                // should be the same as leaf2_lower
                let lower_bound = leaf_vec[leaf_vec.len() - 1].name_hash;
                new_rie.name_hash = lower_bound;
                new_rie.block = num_blocks as u32 + 1;

                {
                    let mut index_vec: Vec<u8> = vec![0; BSIZE];
                    let index_slice = index_vec.as_mut_slice();
                    let index_header_slice = &mut index_slice[0..hindex_len];
                    new_index
                        .dump_into(index_header_slice)
                        .map_err(|_| libc::EIO)?;

                    let mut ie_idx = 0;
                    while let Some(ie) = leaf_vec.pop() {
                        let ie_slice = &mut index_slice[hindex_len + ie_idx as usize * hentry_len
                            ..hindex_len + (ie_idx + 1) as usize * hentry_len];
                        ie.dump_into(ie_slice).map_err(|_| libc::EIO)?;
                        ie_idx += 1;
                    }

                    // zero out possible thrash values
                    let write_size = BSIZE;
                    if self.writei(
                        index_slice,
                        (num_blocks + 1 as usize) * BSIZE,
                        write_size,
                        internals,
                        parent_inum,
                        handle,
                    )? != write_size
                    {
                        return Err(libc::EIO);
                    }
                }
                // index_vec = index entries in the root node
                // leaf_vec = hentries in index node

                // udpate root
                let mut rie_map: BTreeMap<u32, Htree_entry> = BTreeMap::new();
                while let Some(ie) = index_vec.pop() {
                    rie_map.insert(ie.name_hash, ie);
                }
                rie_map.insert(new_rie.name_hash, new_rie);
                {
                    let mut keys: Vec<_> = rie_map.keys().cloned().collect();
                    // add index entries in reverse order
                    while let Some(key) = keys.pop() {
                        if let Some(val) = rie_map.remove(&key) {
                            index_vec.push(val);
                        }
                    }
                }

                // update root info and add new index entry to root node
                {
                    root.ind_entries += 1;
                    root.blocks += 2;
                    let root_header_slice = &mut root_arr_slice[0..hroot_len];
                    root.dump_into(root_header_slice).map_err(|_| libc::EIO)?;
                    let mut rie_idx = 0;
                    while let Some(rie) = index_vec.pop() {
                        let rie_slice = &mut root_arr_slice[hroot_len
                            + rie_idx as usize * hentry_len
                            ..hroot_len + (rie_idx + 1) as usize * hentry_len];
                        rie.dump_into(rie_slice).map_err(|_| libc::EIO)?;
                        rie_idx += 1;
                    }

                    // zero out possible trash values
                    let write_size = BSIZE;
                    if self.writei(root_arr_slice, 0, write_size, internals, parent_inum, handle)?
                        != write_size
                    {
                        return Err(libc::EIO);
                    }
                }
            } else {
                // root is cannot contain more index entries
                return Err(libc::EIO);
            }
        }

        // END
        return Ok(0);
    }
}
