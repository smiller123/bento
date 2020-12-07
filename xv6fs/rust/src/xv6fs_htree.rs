/*
* SPDX-License-Identifier: GPL-2.0 OR MIT
*
* Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
     Anderson, Ang Chen, University of Washington
* Copyright (C) 2006-2018 Frans Kaashoek, Robert Morris, Russ Cox,
*                      Massachusetts Institute of Technology
*/

#[cfg(not(feature = "user"))]
use crate::hash32::{Hash, Hasher};
#[cfg(not(feature = "user"))]
use crate::std;

use core::mem;
use datablock::DataBlock;

use crate::xv6fs_utils::*;

use std::ffi::OsStr;
#[cfg(feature = "user")]
use std::hash::{Hash, Hasher};

pub const HTREE_MAXBLOCKS: u32 =
    (((BSIZE - mem::size_of::<Htree_root>()) / mem::size_of::<Htree_entry>())
        * ((BSIZE - mem::size_of::<Htree_index>()) / mem::size_of::<Htree_entry>())) as u32;

// Htree data structures

#[repr(C)]
#[derive(DataBlock)]
pub struct Htree_root {
    pub dot: Xv6fsDirent,
    pub dotdot: Xv6fsDirent,
    pub depth: u32,
    pub blocks: u32,
    pub ind_entries: u32,
}

impl Htree_root {
    pub const fn new() -> Self {
        Self {
            dot: Xv6fsDirent::new(),
            dotdot: Xv6fsDirent::new(),
            depth: 0,
            blocks: 0,
            ind_entries: 0,
        }
    }
}

#[repr(C)]
#[derive(DataBlock)]
pub struct Htree_index {
    pub fake_dirent: Xv6fsDirent,
    pub entries: u32,
}

impl Htree_index {
    pub const fn new() -> Self {
        Self {
            fake_dirent: Xv6fsDirent::new(),
            entries: 0,
        }
    }
}

#[repr(C)]
#[derive(DataBlock, Copy, Clone, Debug, PartialEq, Eq)]
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

// find the lowest bound for a target in a given array of Htree entries.
// Returns None if no bound is found or if array is empty
pub fn find_lowerbound(arr: &[Htree_entry], len: usize, target: u32) -> Option<usize> {
    if len < 1 {
        return None;
    }
    let mut lo: u32 = 0;
    let mut hi: u32 = len as u32 - 1;

    while lo <= hi {
        let mid = ((hi - lo) / 2) + lo;
        let mid_index = mid as usize;
        let val = &arr[mid_index].name_hash;
        if lo == mid {
            let val2 = &arr[hi as usize].name_hash;
            if *val2 <= target {
                return Some(hi as usize);
            }
            if *val > target {
                return None;
            }
            return Some(lo as usize);
        }
        if *val <= target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    return None;
}

// Calculates the hash value for a given OsStr
pub fn calculate_hash(target: &OsStr) -> u32 {
    let mut s = SliceHasher::new();
    target.hash(&mut s);
    s.finish() as u32
}

pub struct SliceHasher {
    state: u32,
}

impl SliceHasher {
    pub fn new() -> Self {
        SliceHasher { state: 5381 as u32 }
    }

    // djb2_hash
    pub fn write_u8(&mut self, i: u8) {
        self.state = ((self.state << 5) + self.state) + i as u32;
    }
}

#[cfg(not(feature = "user"))]
impl Hasher for SliceHasher {
    fn finish(&self) -> u32 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        for i in bytes {
            self.write_u8(*i);
        }
    }
}

#[cfg(feature = "user")]
impl Hasher for SliceHasher {
    fn finish(&self) -> u64 {
        self.state as u64
    }

    fn write(&mut self, bytes: &[u8]) {
        for i in bytes {
            self.write_u8(*i);
        }
    }
}
