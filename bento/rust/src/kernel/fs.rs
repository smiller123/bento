/*
 * SPDX-License-Identifier: GPL-2.0
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 *
 */

use kernel::ffi::*;
use kernel::kobj::*;
use kernel::raw::*;

use core::cell::UnsafeCell;


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
    bdev: RsBlockDevice,
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
}

impl AsRawFd for BlockDevice {
    fn as_raw_fd(&self) -> RawFd {
        self.bdev.bd_dev() as RawFd
    }
}




/// Wrapper around the kernel `journal_t`.
#[derive(Debug)]
pub struct Journal {
    journal: UnsafeCell<RsJournal>,
}

/// Wrapper around the kernel `handle_t`.
#[derive(Debug)]
pub struct Handle {
    handle: UnsafeCell<RsHandle>,
}

impl Journal {
    pub fn new(bdev: &BlockDevice, fs_dev: &BlockDevice, start: u64, len: i32, bsize: i32) -> Option<Journal> {
        println!("initializing journal");

        let journal;
        unsafe {
            journal = rs_jbd2_journal_init_dev(bdev.bdev.get_raw() as *const c_void, 
                                                fs_dev.bdev.get_raw() as *const c_void, 
                                                start, 
                                                len, 
                                                bsize);
        }
        if journal.is_null() {
            return None;
        } else {
            unsafe {
                // TODO call jbd2_journal_load
                /*if rs_jbd2_journal_load(journal) != 0 {
                    return None;
                }*/

                return Some(Journal { 
                    journal: UnsafeCell::new(RsJournal::from_raw(journal as *const c_void)),
                });
            }
        }
    }

    // begin transaction of size blocks
    pub fn begin_op(&self, blocks: u32) -> Handle {
        let handle;
        unsafe {
            handle = rs_jbd2_journal_start(self.journal.get() as *const c_void, blocks as i32)
        }
        if handle.is_null() {
            panic!("transaction begin failed")
        } else {
            unsafe {
                return Handle {
                    handle: UnsafeCell::new(RsHandle::from_raw(handle as *const c_void)),
                };
            }
        }
    }

    // force completed transactions to write to disk
    pub fn force_commit(&self) -> i32 {
        unsafe {
            return rs_jbd2_journal_force_commit(self.journal.get() as *const c_void);
        }
    }
}

impl Drop for Journal {
    fn drop(&mut self) {
        //TODO destroy journal
        unsafe {
            rs_jbd2_journal_destroy(self.journal.get() as *const c_void);
        }
    }
}


impl Handle {
    // notify intent to modify BufferHead as a part of this transaction
    pub fn get_write_access(&self, bh: &BufferHead) -> i32 {
        unsafe {
            return rs_jbd2_journal_get_write_access(self.handle.get() as *const c_void, bh.get_raw());
        }
    }

    // register a block as part of the transaction associated with this handle
    pub fn journal_write(&self, bh: &BufferHead) -> i32 {
        unsafe {
            return rs_jbd2_journal_dirty_metadata(self.handle.get() as *const c_void, bh.get_raw());
        }
    }
}

// ends transaction
impl Drop for Handle {
    fn drop(&mut self) {
        let res;
        unsafe {
            res = rs_jbd2_journal_stop(self.handle.get() as *const c_void);
        }
        if res == 0 {
             ()
        } else {
             println!("some log transaction was aborted");
             //TODO 
             loop {};
        }
    }
}

unsafe impl Sync for Journal {}
