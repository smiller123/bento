/*
 * SPDX-License-Identifier: GPL-2.0
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 *
 */

use kernel::ffi::*;
use kernel::kobj::*;
use kernel::raw::*;

use crate::std::os::unix::io::*;

use crate::libc;

use bindings::*;

pub const FMODE_READ: u32 = 0x1;
pub const FMODE_WRITE: u32 = 0x2;
pub const FMODE_EXCL: u32 = 0x80;

/// Read a block from disk.
///
/// Calls the kernel `sb_bread` function. If that returns a NULL pointer, this function will
/// return `None`. Otherwise, this function will return `Some`.
///
/// TODO: Make this a method on the RsSuperBlock.
///
/// Arguments:
/// * `sb: &RsSuperBlock` - The kernel-provided superblock of the device
/// * `blockno: u64` - The block number to be read.
pub fn sb_bread_rust(sb: &RsSuperBlock, blockno: u64) -> Option<BufferHead> {
    let bh;
    unsafe {
        bh = sb_bread(sb.get_raw() as *const c_void, blockno);
    }
    if bh.is_null() {
        return None;
    } else {
        unsafe {
            return Some(BufferHead::from_raw(bh as *const c_void));
        }
    }
}

/// Flush a block device.
///
/// This function calls the kernel `blkdev_issue_flush` function.
///
/// If there's an error, it will be written to `error_sector`.
///
/// Arguments:
/// * `bdev: &RsBlockDevice` - The block device to flush.
/// * `gfp_mask: usize` - Memory allocation flags.
/// * `error_section: &mut u64` - Holder for error location.
pub fn blkdev_issue_flush_rust(
    bdev: &RsBlockDevice,
    gfp_mask: usize,
    error_sector: &mut u64,
) -> isize {
    unsafe {
        return blkdev_issue_flush(bdev.get_raw() as *const c_void, gfp_mask, error_sector);
    }
}

#[derive(Debug)]
pub struct BlockDevice {
    pub bdev: RsBlockDevice,
    bsize: u32,
}

impl BlockDevice {
    pub fn new(dev_name: &str, bsize: u32) -> Self {
        Self {
            bdev: RsBlockDevice::new(dev_name),
            bsize: bsize,
        }
    }

    pub fn sync_all(&self) -> Result<(), i32> {
        let mut error_sector = 0;
        blkdev_issue_flush_rust(&self.bdev, GFP_KERNEL as usize, &mut error_sector);
        match error_sector {
            0 => Ok(()),
            _ => Err(error_sector as i32),
        }
    }

    pub fn sync_data(&self) -> Result<(), i32> {
        let mut error_sector = 0;
        blkdev_issue_flush_rust(&self.bdev, GFP_KERNEL as usize, &mut error_sector);
        match error_sector {
            0 => Ok(()),
            _ => Err(error_sector as i32),
        }
    }

    pub fn sync_block(&self, sector: u64) -> Result<(), libc::c_int> {
        if let Some(mut bh) = self.bdev.bread(sector, self.bsize) {
            bh.sync_dirty_buffer();
        }
        Ok(())
    }

    pub fn bread(&self, blockno: u64) -> Result<BufferHead, libc::c_int> {
        self.bdev.bread(blockno, self.bsize).ok_or(libc::EIO)
    }

    pub fn getblk(&self, blockno: u64) -> Result<BufferHead, libc::c_int> {
        self.bdev.getblk(blockno, self.bsize).ok_or(libc::EIO)
    }
}

impl AsRawFd for BlockDevice {
    fn as_raw_fd(&self) -> RawFd {
        self.bdev.bd_dev() as RawFd
    }
}
