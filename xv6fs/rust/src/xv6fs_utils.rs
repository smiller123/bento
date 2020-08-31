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
pub const PROVINO: u16 = 2;

pub const MAXOPBLOCKS: usize = 32;
#[allow(dead_code)]
pub const LOGSIZE: usize = 1032;

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
