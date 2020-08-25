/*
* SPDX-License-Identifier: GPL-2.0 OR MIT
*
* Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
     Anderson, Ang Chen, University of Washington
* Copyright (C) 2006-2018 Frans Kaashoek, Robert Morris, Russ Cox,
*                      Massachusetts Institute of Technology
*/

#[cfg(not(feature = "user"))]
use crate::bento_utils;
#[cfg(not(feature = "user"))]
use crate::fuse;
#[cfg(not(feature = "user"))]
use crate::libc;
#[cfg(not(feature = "user"))]
use crate::std;
#[cfg(not(feature = "user"))]
use crate::time;

use alloc::collections::btree_map::BTreeMap;

use alloc::string::String;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;

use core::mem;
use core::str;

use bento_utils::BentoFilesystem;

use datablock::DataBlock;

use bento_utils::consts::*;
use bento_utils::*;
use fuse::consts::*;

use fuse::*;

#[cfg(not(feature = "user"))]
use bento::kernel::journal::*;
#[cfg(feature = "user")]
use crate::xv6fs_log::*;

#[cfg(not(feature = "user"))]
use crate::println;

use std::ffi::OsStr;
use std::path::Path;
use std::sync::RwLock;

use time::*;

use serde::{Serialize, Deserialize};

use crate::xv6fs_file::*;
use crate::xv6fs_htree::*;
use crate::xv6fs_utils::*;

#[derive(Serialize, Deserialize)]
pub struct Xv6State {
    diskname: String,
}

pub struct Xv6FileSystem {
    pub log: Option<Journal>,
    pub sb: Option<Xv6fsSB>,
    pub disk: Option<Arc<Disk>>,
    pub ilock_cache: Option<Vec<RwLock<Inode>>>,
    pub icache_map: Option<RwLock<BTreeMap<u64, usize>>>,
    pub ialloc_lock: Option<RwLock<usize>>,
    pub balloc_lock: Option<RwLock<usize>>,
    pub diskname: Option<String>,
}

impl BentoFilesystem<'_, Xv6State,Xv6State> for Xv6FileSystem {
    fn get_name(&self) -> &'static str {
        Xv6FileSystem::NAME
    }

    fn bento_destroy(&mut self, _req: &Request) {
        self.log.as_ref().unwrap().destroy();
    }


    fn bento_init(
        &mut self,
        _req: &Request,
        devname: &OsStr,
        fc_info: &mut FuseConnInfo,
    ) -> Result<(), i32> {
        fc_info.proto_major = BENTO_KERNEL_VERSION;
        fc_info.proto_minor = BENTO_KERNEL_MINOR_VERSION;
        fc_info.want = 0;

        let mut max_readahead = u32::MAX;
        if fc_info.max_readahead < max_readahead {
            max_readahead = fc_info.max_readahead;
        }

        if self.disk.is_none() {
            let devname_str = devname.to_str().unwrap();
            let disk = Disk::new(devname_str, BSIZE as u64);
            let mut disk_string = devname_str.to_string();
            disk_string.push('\0');
            self.diskname = Some(disk_string);
            self.disk = Some(Arc::new(disk));
        }

        let sb_lock = Xv6fsSB {
            size: 0,
            nblocks: 0,
            ninodes: 0,
            nlog: 0,
            logstart: 0,
            inodestart: 0,
            bmapstart: 0,
        };
        self.sb = Some(sb_lock);

        self.iinit();

        fc_info.want |= FUSE_BIG_WRITES;
        fc_info.want |= FUSE_ATOMIC_O_TRUNC;
        fc_info.want |= FUSE_WRITEBACK_CACHE;

        fc_info.max_readahead = max_readahead;
        fc_info.max_background = 0;
        fc_info.congestion_threshold = 0;
        fc_info.time_gran = 1;

        return Ok(());
    }

    fn bento_statfs(&self, _req: &Request, _ino: u64, reply: ReplyStatfs) {
        let sb_lock = self.sb.as_ref().unwrap();
        let fs_size = sb_lock.size;
        reply.statfs(fs_size as u64, 0, 0, 0, 0, BSIZE as u32, DIRSIZ as u32, 0);
    }

    fn bento_open(&self, _req: &Request, nodeid: u64, flags: u32, reply: ReplyOpen) {
        let log = self.log.as_ref().unwrap();
        let inode = match self.iget(nodeid) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };

        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };
        let mut internals = match inode_guard.internals.write() {
            Ok(x) => x,
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };

        // Check if inode is a file
        if internals.inode_type != T_FILE {
            reply.error(libc::EISDIR);
            return;
        }

        if flags & libc::O_TRUNC as u32 != 0 {
            let handle = log.begin_op(2);
            internals.size = 0;
            if let Err(x) = self.iupdate(&internals, inode.inum, &handle) {
                reply.error(x);
                return;
            }
        }

        let fh = 0;
        let open_flags = FOPEN_KEEP_CACHE;
        reply.opened(fh, open_flags);
    }

    fn bento_opendir(&self, _req: &Request, nodeid: u64, _flags: u32, reply: ReplyOpen) {
        let inode = match self.iget(nodeid) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };

        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };
        let internals = match inode_guard.internals.write() {
            Ok(x) => x,
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };

        if internals.inode_type != T_DIR {
            reply.error(libc::ENOTDIR);
        } else {
            let fh = 0;
            let open_flags = 0;
            reply.opened(fh, open_flags);
        }
    }

    fn bento_getattr(&self, _req: &Request, nodeid: u64, reply: ReplyAttr) {
        let inode = match self.iget(nodeid) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };

        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };
        let internals = match inode_guard.internals.read() {
            Ok(x) => x,
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };
        let attr_valid = Timespec::new(1, 999999999);
        match self.stati(nodeid, &internals) {
            Ok(attr) => {
                reply.attr(&attr_valid, &attr);
            }
            Err(x) => {
                reply.error(x);
            }
        };
    }

    fn bento_setattr(
        &self,
        _req: &Request,
        ino: u64,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        _size: Option<u64>,
        _atime: Option<Timespec>,
        _mtime: Option<Timespec>,
        _fh: Option<u64>,
        _crtime: Option<Timespec>,
        _chgtime: Option<Timespec>,
        _bkuptime: Option<Timespec>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        
        let inode = match self.iget(ino) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };

        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };
        let internals = match inode_guard.internals.read() {
            Ok(x) => x,
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };
        let attr_valid = Timespec::new(1, 999999999);
        match self.stati(ino, &internals) {
            Ok(attr) => reply.attr(&attr_valid, &attr),
            Err(x) => reply.error(x),
        }
    }

    fn bento_lookup(&self, _req: &Request, nodeid: u64, name: &OsStr, reply: ReplyEntry) {
        // Get inode number from nodeid
        let inode = match self.iget(nodeid) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };
        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };
        let mut internals = match inode_guard.internals.write() {
            Ok(x) => x,
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };
        let mut poff = 0;
        let child = match self.dirlookup(&mut internals, name, &mut poff) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };

        let outarg_nodeid = child.inum as u64;
        let outarg_generation = 0;
        let attr_valid = Timespec::new(1, 999999999);

        let child_inode_guard = match self.ilock(child.idx, &icache, child.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };
        let child_internals = match child_inode_guard.internals.read() {
            Ok(x) => x,
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };
        match self.stati(outarg_nodeid, &child_internals) {
            Ok(outarg_attr) => {
                reply.entry(&attr_valid, &outarg_attr, outarg_generation);
            }
            Err(x) => {
                reply.error(x);
            }
        };
    }

    fn bento_read(
        &self,
        _req: &Request,
        nodeid: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        reply: ReplyData,
    ) {
        // Get inode number nodeid
        let inode = match self.iget(nodeid) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };

        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };
        let mut internals = match inode_guard.internals.write() {
            Ok(x) => x,
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };

        // Check if inode is a file
        if internals.inode_type != T_FILE {
            reply.error(libc::EISDIR);
            return;
        }

        let off = offset as usize;
        let n = size as usize;

        let mut buf_vec: Vec<u8> = vec![0; n as usize];
        let buf_slice = buf_vec.as_mut_slice();

        let read_rs = match self.readi(buf_slice, off, n, &mut internals) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };
        reply.data(&buf_slice[0..read_rs as usize]);
    }

    fn bento_write(
        &self,
        _req: &Request,
        nodeid: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _flags: u32,
        reply: ReplyWrite,
    ) {
        // Get the inode at nodeid
        let max = ((MAXOPBLOCKS - 1 - 1 - 2) / 2) * BSIZE;
        let mut i = 0;
        let n = data.len();
        let mut off = offset as usize;
        let mut file_off = 0;
        //let nblocks = 1 + 1 + 2 + (off + n + BSIZE - 1)/BSIZE - off/BSIZE;
        while i < n {
            let log = self.log.as_ref().unwrap();
            let handle = log.begin_op(MAXOPBLOCKS as u32);
            let inode = match self.iget(nodeid) {
                Ok(x) => x,
                Err(x) => {
                    reply.error(x);
                    return;
                }
            };

            let icache = self.ilock_cache.as_ref().unwrap();
            let inode_guard = match self.ilock(inode.idx, &icache, inode.inum) {
                Ok(x) => x,
                Err(x) => {
                    reply.error(x);
                    return;
                }
            };
            let mut internals = match inode_guard.internals.write() {
                Ok(x) => x,
                Err(_) => {
                    reply.error(libc::EIO);
                    return;
                }
            };

            // Check if inode is a file
            if internals.inode_type != T_FILE {
                reply.error(libc::EISDIR);
                return;
            }

            let mut n1 = n - i;
            if n1 > max {
                n1 = max;
            }
            let data_region = &data[file_off..];
            let r = match self.writei(data_region, off, n1, &mut internals, inode.inum, &handle) {
                Ok(x) => x,
                Err(x) => {
                    reply.error(x);
                    return;
                }
            };

            off += r;
            file_off += r;
            i += r;
        }
        reply.written(n as u32);
    }

    #[allow(unused_mut)]
    fn bento_readdir(
        &self,
        _req: &Request,
        nodeid: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        // Get inode number nodeid
        let inode = match self.iget(nodeid) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };

        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };
        let mut internals = match inode_guard.internals.write() {
            Ok(x) => x,
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };

        // Check if inode is directory
        if internals.inode_type != T_DIR {
            reply.error(libc::ENOTDIR);
            return;
        }
        let hroot_len = mem::size_of::<Htree_root>();
        let hindex_len = mem::size_of::<Htree_index>();
        let hentry_len = mem::size_of::<Htree_entry>();
        let de_len = mem::size_of::<Xv6fsDirent>();
        let mut hroot_vec: Vec<u8> = vec![0; hroot_len];
        let mut buf_off = 1;
        let mut inarg_offset = offset as usize;
        let hroot_slice = hroot_vec.as_mut_slice();

        // try reading directory root
        let mut root = Htree_root::new();
        match self.readi(hroot_slice, 0, hroot_len, &mut internals) {
            Ok(x) if x != hroot_len => {
                reply.error(1);
                return;
            }
            Err(x) => {
                reply.error(x);
                return;
            }
            _ => {}
        };
        if root.extract_from(hroot_slice).is_err() {
            reply.error(libc::EIO);
            return;
        }

        let num_indeces = root.ind_entries;
        if num_indeces == 0 {
            reply.ok();
            return;
        }

        let mut hie_vec: Vec<u8> = vec![0; hentry_len];
        let hie_slice = hie_vec.as_mut_slice();

        // check the index pointers stored in the root node
        for off in (hroot_len..(num_indeces as usize * hentry_len) + hroot_len).step_by(hentry_len)
        {
            if off >= BSIZE {
                break;
            }
            let mut hie = Htree_entry::new();
            match self.readi(hie_slice, off as usize, hentry_len, &mut internals) {
                Ok(x) if x != hentry_len => {
                    reply.error(1);
                    return;
                }
                Err(x) => {
                    reply.error(x);
                    return;
                }
                _ => {}
            }
            if hie.extract_from(hie_slice).is_err() {
                reply.error(libc::EIO);
                return;
            }

            // check the index block for entries
            let mut ind_arr_vec: Vec<u8> = vec![0; BSIZE];
            let ind_arr_slice = ind_arr_vec.as_mut_slice();
            match self.readi(
                ind_arr_slice,
                BSIZE * hie.block as usize,
                BSIZE,
                &mut internals,
            ) {
                Ok(x) if x != BSIZE => {
                    reply.error(1);
                    return;
                }
                Err(x) => {
                    reply.error(x);
                    return;
                }
                _ => {}
            }

            let ind_header_slice = &mut ind_arr_slice[0..hindex_len];
            let mut index = Htree_index::new();
            if index.extract_from(ind_header_slice).is_err() {
                reply.error(libc::EIO);
                return;
            }

            let num_entries = index.entries;

            if num_entries == 0 {
                continue;
            }

            // check entries in index node
            for ine_idx in
                (hindex_len..hindex_len + (hentry_len * index.entries as usize)).step_by(hentry_len)
            {
                let ine_slice = &mut ind_arr_slice[ine_idx..ine_idx + hentry_len];
                let mut ine = Htree_entry::new();
                if ine.extract_from(ine_slice).is_err() {
                    reply.error(libc::EIO);
                    return;
                }
                let dblock_off = ine.block;
                if dblock_off == 0 {
                    continue;
                }
                let mut de_block_vec: Vec<u8> = vec![0; BSIZE];
                let de_block_slice = de_block_vec.as_mut_slice();

                match self.readi(
                    de_block_slice,
                    BSIZE * dblock_off as usize,
                    BSIZE,
                    &mut internals,
                ) {
                    Err(x) => {
                        reply.error(x);
                        return;
                    }
                    _ => {}
                }

                // check dirents in leaf node
                for de_off in (0..BSIZE).step_by(de_len) {
                    let de_slice = &mut de_block_slice[de_off..de_off + de_len];
                    let mut de = Xv6fsDirent::new();
                    if de.extract_from(de_slice).is_err() {
                        reply.error(libc::EIO);
                        return;
                    }

                    if de.inum == 0 {
                        continue;
                    }
                    if inarg_offset >= 1 {
                        inarg_offset -= 1;
                        buf_off += 1;
                        continue;
                    }

                    let i_type;
                    if de.inum as u64 == nodeid {
                        i_type = FileType::Directory;
                    } else {
                        let entry = match self.iget(de.inum as u64) {
                            Ok(x) => x,
                            Err(x) => {
                                reply.error(x);
                                return;
                            }
                        };

                        let entry_inode_guard = match self.ilock(entry.idx, &icache, de.inum) {
                            Ok(x) => x,
                            Err(x) => {
                                reply.error(x);
                                return;
                            }
                        };
                        let entry_internals = match entry_inode_guard.internals.read() {
                            Ok(x) => x,
                            Err(_) => {
                                reply.error(libc::EIO);
                                return;
                            }
                        };

                        i_type = match entry_internals.inode_type {
                            T_DIR => FileType::Directory,
                            T_LNK => FileType::Symlink,
                            _ => FileType::RegularFile,
                        };
                    }

                    let name_str = match str::from_utf8(&de.name) {
                        Ok(x) => x,
                        Err(_) => "",
                    };

                    if reply.add(de.inum as u64, buf_off, i_type, name_str) {
                        reply.ok();
                        return;
                    }
                    buf_off += 1;
                }
            }
        }
        reply.ok();
    }

    fn bento_create(
        &self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _flags: u32,
        reply: ReplyCreate,
    ) {
        // Check if the file already exists
        let log = self.log.as_ref().unwrap();
        let handle = log.begin_op(16);
        let child = match self.create_internal(parent, T_FILE, name, &handle) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };

        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(child.idx, &icache, child.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };
        let internals = match inode_guard.internals.read() {
            Ok(x) => x,
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };

        let fh = 0;
        let open_flags = FOPEN_KEEP_CACHE;
        let nodeid = child.inum as u64;
        let generation = 0;
        let attr_valid = Timespec::new(1, 999999999);
        match self.stati(nodeid, &internals) {
            Ok(attr) => {
                reply.created(&attr_valid, &attr, generation, fh, open_flags);
            }
            Err(x) => {
                reply.error(x);
            }
        }
    }

    fn bento_mkdir(
        &self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        reply: ReplyEntry,
    ) {
        let log = self.log.as_ref().unwrap();
        let handle = log.begin_op(MAXOPBLOCKS as u32);
        let child = match self.create_internal(parent, T_DIR, &name, &handle) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };

        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(child.idx, &icache, child.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };

        let internals = match inode_guard.internals.read() {
            Ok(x) => x,
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };

        let out_nodeid = child.inum as u64;
        let generation = 0;
        let attr_valid = Timespec::new(1, 999999999);
        match self.stati(out_nodeid, &internals) {
            Ok(attr) => {
                reply.entry(&attr_valid, &attr, generation);
            }
            Err(x) => {
                reply.error(x);
                return;
            }
        }
    }

    fn bento_rmdir(&self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let log = self.log.as_ref().unwrap();
        let handle = log.begin_op(MAXOPBLOCKS as u32);
        match self.dounlink(parent, name, &handle) {
            Ok(_) => reply.ok(),
            Err(x) => reply.error(x),
        }
    }

    fn bento_unlink(&self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let log = self.log.as_ref().unwrap();
        let handle = log.begin_op(MAXOPBLOCKS as u32);
        match self.dounlink(parent, name, &handle) {
            Ok(_) => reply.ok(),
            Err(x) => reply.error(x),
        }
    }

    fn bento_fsync(&self, _req: &Request, _ino: u64, _fh: u64, _datasync: bool, reply: ReplyEmpty) {
        let log = self.log.as_ref().unwrap();
        log.force_commit();
        reply.ok();
    }

    fn bento_symlink(
        &self,
        _req: &Request,
        nodeid: u64,
        name: &OsStr,
        linkname: &Path,
        reply: ReplyEntry,
    ) {
        let log = self.log.as_ref().unwrap();
        let handle = log.begin_op(MAXOPBLOCKS as u32);
        // Create new file
        let child = match self.create_internal(nodeid, T_LNK, name, &handle) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };

        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(child.idx, &icache, child.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };
        let mut internals = match inode_guard.internals.write() {
            Ok(x) => x,
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };

        let mut len_slice = [0; mem::size_of::<u32>()];
        let linkname_str = linkname.to_str().unwrap();
        let str_length: u32 = linkname_str.len() as u32 + 1;
        let strlen_slice = str_length.to_ne_bytes();
        len_slice.copy_from_slice(&strlen_slice);
        if let Err(x) = self.writei(
            &len_slice,
            0,
            mem::size_of::<u32>(),
            &mut internals,
            child.inum,
            &handle,
        ) {
            reply.error(x);
            return;
        };

        if let Err(x) = self.writei(
            linkname_str.as_bytes(),
            mem::size_of::<u32>(),
            linkname_str.len(),
            &mut internals,
            child.inum,
            &handle,
        ) {
            reply.error(x);
            return;
        };
        let out_nodeid = child.inum as u64;
        let generation = 0;
        let attr_valid = Timespec::new(1, 999999999);
        match self.stati(out_nodeid, &internals) {
            Ok(attr) => reply.entry(&attr_valid, &attr, generation),
            Err(x) => {
                reply.error(x);
            }
        }
    }

    fn bento_readlink(&self, _req: &Request, nodeid: u64, reply: ReplyData) {
        let inode = match self.iget(nodeid) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };

        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };
        let mut internals = match inode_guard.internals.write() {
            Ok(x) => x,
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };

        // Check if inode is a file
        if internals.inode_type != T_LNK {
            reply.error(1);
            return;
        }

        let mut len_slice = [0; 4];

        match self.readi(&mut len_slice, 0, mem::size_of::<u32>(), &mut internals) {
            Ok(x) if x != mem::size_of::<u32>() => {
                reply.error(libc::EIO);
                return;
            }
            Err(x) => {
                reply.error(x);
                return;
            }
            _ => {}
        }
        let mut str_len_bytes = [0; 4];
        str_len_bytes.copy_from_slice(&len_slice);
        let str_len = u32::from_ne_bytes(str_len_bytes);

        let mut buf_vec: Vec<u8> = vec![0; str_len as usize];
        let buf_slice = buf_vec.as_mut_slice();

        match self.readi(
            buf_slice,
            mem::size_of::<u32>(),
            str_len as usize,
            &mut internals,
        ) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };
        reply.data(buf_slice);
    }

    fn bento_update_prepare(&mut self) -> Option<Xv6State> {
        let state = Xv6State {
            diskname: self.diskname.as_ref().unwrap().clone(),
        };
        Some(state)
    }

    fn bento_update_transfer(&mut self, state_opt: Option<Xv6State>) {
        if let Some(state) = state_opt {
            let disk = Arc::new(Disk::new(state.diskname.as_str(), 4096));
            self.disk = Some(disk);
            self.diskname = Some(state.diskname);
            let sb_lock = Xv6fsSB {
                size: 0,
                nblocks: 0,
                ninodes: 0,
                nlog: 0,
                logstart: 0,
                inodestart: 0,
                bmapstart: 0,
            };
            self.sb = Some(sb_lock);

            self.iinit();
        }
    }
}

impl Xv6FileSystem {
    const NAME: &'static str = "xv6fs_ll\0";

    fn create_internal<'a>(
        &'a self,
        nodeid: u64,
        itype: u16,
        name: &OsStr,
        handle: &Handle
    ) -> Result<CachedInode<'a>, libc::c_int> {
        // Get inode for parent directory
        let parent = self.iget(nodeid)?;
        let icache = self.ilock_cache.as_ref().unwrap();
        // Get inode for new file
        let parent_inode_guard = self.ilock(parent.idx, &icache, parent.inum)?;
        let mut parent_internals = parent_inode_guard
            .internals
            .write()
            .map_err(|_| libc::EIO)?;

        let inode = self.ialloc(itype, handle)?;
        if (parent_internals.size as usize + mem::size_of::<Xv6fsDirent>())
            > (MAXFILE as usize * BSIZE)
        {
            return Err(libc::EIO);
        }

        let inode_guard = self.ilock(inode.idx, &icache, inode.inum)?;
        let mut internals = inode_guard.internals.write().map_err(|_| libc::EIO)?;

        internals.major = parent_internals.major;
        internals.minor = parent_internals.minor;
        internals.nlink = 1;
    
        self.iupdate(&internals, inode.inum, handle)?;
    
        if itype == T_DIR {
            parent_internals.nlink += 1;
            self.iupdate(&parent_internals, parent.inum, handle)?;
            let d = OsStr::new(".");
            self.dirlink(&mut internals, &d, inode.inum, inode.inum, handle)?;
    
            let dd = OsStr::new("..");
            self.dirlink(&mut internals, &dd, nodeid as u32, inode.inum, handle)?;
        }
    
        self.dirlink(&mut parent_internals, name, inode.inum, parent.inum, handle)?;
        return Ok(inode);
    }

    fn isdirempty(&self, internals: &mut InodeInternal) -> Result<bool, libc::c_int> {
        let hroot_len = mem::size_of::<Htree_root>();
        let hindex_len = mem::size_of::<Htree_index>();
        let hentry_len = mem::size_of::<Htree_entry>();
        let de_len = mem::size_of::<Xv6fsDirent>();
        let mut hroot_vec: Vec<u8> = vec![0; hroot_len];

        let hroot_slice = hroot_vec.as_mut_slice();

        // try reading directory root
        let mut root = Htree_root::new();
        match self.readi(hroot_slice, 0, hroot_len, internals) {
            Ok(x) if x != hroot_len => return Err(libc::EIO),
            Err(x) => {
                return Err(x);
            }
            _ => {}
        };

        root.extract_from(hroot_slice).map_err(|_| libc::EIO)?;

        let num_indeces = root.ind_entries;
        if num_indeces == 0 {
            return Ok(true);
        }

        // check the index pointers stored in the root
        for off in (hroot_len..(num_indeces as usize * hentry_len) + hroot_len).step_by(hentry_len)
        {
            if off >= BSIZE {
                break;
            }
            let mut rie = Htree_entry::new();
            let mut rie_vec: Vec<u8> = vec![0; hentry_len];
            let rie_slice = rie_vec.as_mut_slice();
            match self.readi(rie_slice, off as usize, hentry_len, internals) {
                Ok(x) if x != hentry_len => return Err(libc::EIO),
                Err(x) => {
                    return Err(x);
                }
                _ => {}
            }

            rie.extract_from(rie_slice).map_err(|_| libc::EIO)?;

            // check the index block for entries
            let mut ind_arr_vec: Vec<u8> = vec![0; BSIZE];
            let ind_arr_slice = ind_arr_vec.as_mut_slice();
            match self.readi(ind_arr_slice, BSIZE * rie.block as usize, BSIZE, internals) {
                Ok(x) if x != BSIZE => return Err(libc::EIO),
                Err(x) => {
                    return Err(x);
                }
                _ => {}
            }

            let ind_header_slice = &mut ind_arr_slice[0..hindex_len];
            let mut index = Htree_index::new();
            index
                .extract_from(ind_header_slice)
                .map_err(|_| libc::EIO)?;

            let num_entries = index.entries;
            if num_entries == 0 {
                break;
            }

            // check entries in index node
            for ine_idx in
                (hindex_len..hindex_len + (hentry_len * index.entries as usize)).step_by(hentry_len)
            {
                if ine_idx / hentry_len >= num_entries as usize || ine_idx >= BSIZE {
                    break;
                }

                let ine_slice = &mut ind_arr_slice[ine_idx..ine_idx + hentry_len];
                let mut ine = Htree_entry::new();

                ine.extract_from(ine_slice).map_err(|_| libc::EIO)?;

                let dblock_off = ine.block;
                if dblock_off == 0 {
                    break;
                }
                let mut de_block_vec: Vec<u8> = vec![0; BSIZE];
                let de_block_slice = de_block_vec.as_mut_slice();

                match self.readi(
                    de_block_slice,
                    BSIZE * dblock_off as usize,
                    BSIZE,
                    internals,
                ) {
                    Ok(x) if x != BSIZE => return Err(libc::EIO),
                    Err(x) => {
                        return Err(x);
                    }
                    _ => {}
                }

                // check dirents in leaf node
                for de_off in (0..BSIZE).step_by(de_len) {
                    let de_slice = &mut de_block_slice[de_off..de_off + de_len];
                    let mut de = Xv6fsDirent::new();

                    de.extract_from(de_slice).map_err(|_| libc::EIO)?;

                    if de.inum != 0 {
                        return Ok(false);
                    }
                }
            }
        }

        return Ok(true);
    }
    
    fn dounlink(&self, nodeid: u64, name: &OsStr, handle: &Handle) -> Result<usize, libc::c_int> {
        let parent = self.iget(nodeid)?;
        let icache = self.ilock_cache.as_ref().unwrap();
        let parent_inode_guard = self.ilock(parent.idx, &icache, parent.inum)?;
        let mut parent_internals = parent_inode_guard
            .internals
            .write()
            .map_err(|_| libc::EIO)?;
        let mut poff = 0;
        let name_str = name.to_str().unwrap();
        if name_str == "." || name_str == ".." {
            return Err(libc::EIO);
        }
        let inode = self.dirlookup(&mut parent_internals, name, &mut poff)?;

        let inode_guard = self.ilock(inode.idx, &icache, inode.inum)?;
        let mut inode_internals = inode_guard.internals.write().map_err(|_| libc::EIO)?;

        if inode_internals.nlink < 1 {
            return Err(libc::EIO);
        }

        if inode_internals.inode_type == T_DIR {
            match self.isdirempty(&mut inode_internals) {
                Ok(true) => {}
                _ => {
                    return Err(libc::ENOTEMPTY);
                }
            }
        }

        let de_arr = [0; mem::size_of::<Xv6fsDirent>()];
        let buf_len = mem::size_of::<Xv6fsDirent>();
        let r = self.writei(
            &de_arr,
            poff as usize,
            buf_len,
            &mut parent_internals,
            parent.inum,
            handle
        )?;

        if r != buf_len {
            return Err(libc::EIO);
        }

        if inode_internals.inode_type == T_DIR {
            parent_internals.nlink -= 1;
            self.iupdate(&parent_internals, parent.inum, handle)?;
        }

        inode_internals.nlink -= 1;
        self.iupdate(&inode_internals, inode.inum, handle)?;
    
        return Ok(0);
    }
}
