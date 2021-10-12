/*
 * SPDX-License-Identifier: GPL-2.0
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 *
 */

use core::mem::size_of;

use crate::bindings::*;

pub const FUSE_NAME_OFFSET: usize = 24;

pub const FUSE_MAX_MAX_PAGES: u32 = 256;
pub const FUSE_DEFAULT_MAX_PAGES_PER_REQ: u32 = 32;

pub const FUSE_BUFFER_HEADER_SIZE: u32 = 0x1000;

/// Calculate the next correct `fuse_dirent` alignment after the provided offset.
pub fn fuse_dirent_align(x: usize) -> usize {
    let size = size_of::<u64>();
    let left = x + size - 1;
    let right = !(size - 1);
    let ret = left & right;
    return ret;
}

impl fuse_attr {
    pub fn new() -> Self {
        Self {
            ino: 0,
            size: 0,
            blocks: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            atimensec: 0,
            mtimensec: 0,
            ctimensec: 0,
            mode: 0,
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 0,
            flags: 0,
        }
    }
}
