/*
* SPDX-License-Identifier: GPL-2.0 OR MIT
*
* Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
     Anderson, Ang Chen, University of Washington
* Copyright (C) 2006-2018 Frans Kaashoek, Robert Morris, Russ Cox,
*                      Massachusetts Institute of Technology
*/

use core::mem;
use datablock::DataBlock;

pub const BSIZE: usize = 4096;

pub const T_DIR: u16 = 1;
pub const T_FILE: u16 = 2;
#[allow(dead_code)]
pub const T_DEV: u16 = 3;
pub const T_LNK: u16 = 4;

pub const DIRSIZ: u16 = 60;
pub const NDIRECT: u32 = 10;
pub const NINDIRECT: u32 = (BSIZE / mem::size_of::<u32>()) as u32;
pub const NDINDIRECT: u32 = NINDIRECT * NINDIRECT;
pub const MAXFILE: u32 = NDIRECT + NINDIRECT + NDINDIRECT;

pub const IPB: usize = BSIZE / mem::size_of::<Xv6fsInode>();
#[allow(dead_code)]
pub const DPB: usize = BSIZE / mem::size_of::<Xv6fsDirent>();

pub const BPB: usize = BSIZE * 8;

pub const NINODE: usize = 300;

pub const MAXOPBLOCKS: usize = 32;
pub const LOGSIZE: usize = MAXOPBLOCKS * 3;

#[allow(dead_code)]
pub const HTREE_ROOT_INDEXSIZE: usize =
    (BSIZE - (mem::size_of::<Xv6fsDirent>() * 2) - mem::size_of::<u32>())
        / mem::size_of::<u32>() as usize;

pub const HTREE_MAXDEPTH: u32 = 2;

pub const HTREE_M: usize =
    (BSIZE - mem::size_of::<Xv6fsDirent>()) / mem::size_of::<Htree_entry>() as usize;

// pub const HTREE_L: u32 = BSIZE / (mem::size_of::<u32>() + mem::size_of::<Xv6fsDirent>()) as u32;

pub fn iblock(i: usize, sb: &Xv6fsSB) -> usize {
    i / IPB + sb.inodestart as usize
}

pub fn bblock(b: usize, sb: &Xv6fsSB) -> usize {
    b / BPB + sb.bmapstart as usize
}

#[repr(C)]
#[derive(DataBlock, Copy, Clone)]
pub struct Xv6fsInode {
    pub inode_type: u16,
    pub major: u16,
    pub minor: u16,
    pub nlink: u16,
    pub size: u64,
    pub addrs: [u32; NDIRECT as usize + 2],
}

impl Xv6fsInode {
    pub const fn new() -> Self {
        Self {
            inode_type: 0,
            major: 0,
            minor: 0,
            nlink: 0,
            size: 0,
            addrs: [0; NDIRECT as usize + 2],
        }
    }
}

#[repr(C)]
#[derive(DataBlock)]
pub struct Xv6fsSB {
    pub size: u32,
    pub nblocks: u32,
    pub ninodes: u32,
    pub nlog: u32,
    pub logstart: u32,
    pub inodestart: u32,
    pub bmapstart: u32,
}

#[repr(C)]
#[derive(DataBlock)]
pub struct Xv6fsDirent {
    pub inum: u32,
    pub name: [u8; DIRSIZ as usize],
}

impl Xv6fsDirent {
    pub const fn new() -> Self {
        Self {
            inum: 0,
            name: [0; DIRSIZ as usize],
        }
    }
}

// Htree data structures
// At the moment, each data structure fits to one disk block

#[repr(C)]
#[derive(DataBlock)]
pub struct Htree_root {
    pub dot: Xv6fsDirent,
    pub dotdot: Xv6fsDirent,
    pub depth: u32,
    pub htree_indeces: [u32; HTREE_ROOT_INDEXSIZE as usize],
}

impl Htree_root {
    pub const fn new() -> Self {
        Self {
            dot: Xv6fsDirent::new(),
            dotdot: Xv6fsDirent::new(),
            depth: 0,
            htree_indeces: [0; HTREE_ROOT_INDEXSIZE as usize],
        }
    }
}
// #[derive(DataBlock)]
// pub struct Htree_index {
//     pub fake_dirent: Xv6fsDirent,
//     pub hash: [u32; HTREE_M - 1],
//     pub blockno: [u32; HTREE_M],
// }

#[repr(C)]
#[derive(DataBlock)]
pub struct Htree_index {
    pub fake_dirent: Xv6fsDirent,
    pub htree_entries: [Htree_entry; HTREE_M as usize],
}

impl Htree_index {
    pub const fn new() -> Self {
        Self {
            fake_dirent: Xv6fsDirent::new(),
            htree_entries: [Htree_entry::new(); HTREE_M as usize],
        }
    }
}

#[repr(C)]
#[derive(DataBlock, Copy, Clone)]
pub struct Htree_entry {
    pub name_hash: u32,
    pub block: u32,
}

impl Htree_entry {
    pub const fn new() -> Self {
        Self {
            name_hash: 0,
            block: 0,
        }
    }
}
