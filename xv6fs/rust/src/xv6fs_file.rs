/*
 * SPDX-License-Identifier: GPL-2.0 OR MIT
 *
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 * Copyright (C) 2006-2018 Frans Kaashoek, Robert Morris, Russ Cox,
 *                      Massachusetts Institute of Technology
 */

#[cfg(not(feature = "user"))]
use crate::std;

use crate::xv6fs_fs::*;
use crate::xv6fs_utils::*;
use crate::xv6fs_ll::*;

use std::sync::RwLock;

pub struct CachedInode<'a> {
    pub idx: usize,
    pub inum: u32,
    pub fs: &'a Xv6FileSystem,
}

impl Drop for CachedInode<'_> {
    fn drop(&mut self) {
        let _ = self.fs.iput(self);
    }
}

pub struct Inode {
    pub dev: u32,
    pub inum: u32,
    pub nref: i32,
    pub internals: RwLock<InodeInternal>,
}

impl Inode {
    pub fn new() -> Self {
        Inode {
            dev: 0,
            inum: 0,
            nref: 0,
            internals: RwLock::new(InodeInternal::new()),
        }
    }
}

pub struct InodeInternal {
    pub valid: i32,

    pub inode_type: u16,
    pub major: u16,
    pub minor: u16,
    pub nlink: u16,
    pub size: u64,
    pub addrs: [u32; NDIRECT as usize + 2],
}

impl InodeInternal {
    pub const fn new() -> Self {
        InodeInternal {
            valid: 0,
            inode_type: 0,
            major: 0,
            minor: 0,
            nlink: 0,
            size: 0,
            addrs: [0; NDIRECT as usize + 2],
        }
    }
}
