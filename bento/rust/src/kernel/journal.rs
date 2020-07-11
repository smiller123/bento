/*
 * SPDX-License-Identifier: GPL-2.0
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 *
 */

use core::cell::UnsafeCell;

use kernel::ffi::*;
use kernel::kobj::*;
use kernel::raw::*;

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
    pub fn new(bdev: &RsBlockDevice, fs_dev: &RsBlockDevice, start: u64, len: i32, bsize: i32) -> Option<Journal> {
        let journal;
        unsafe {
            journal = rs_jbd2_journal_init_dev(bdev.get_raw() as *const c_void, 
                                                fs_dev.get_raw() as *const c_void, 
                                                start, 
                                                len, 
                                                bsize);
        }
        if journal.is_null() {
            return None;
        } else {
            unsafe {
                return Some(Journal { 
                    journal: UnsafeCell::new(RsJournal::from_raw(journal as *const c_void)),
                });
            }
        }
    }

    // begin transaction of size blocks
    pub fn begin_op(&mut self, blocks: u32) -> Option<Handle> {
        let handle;
        unsafe {
            handle = rs_jbd2_journal_start(self.journal.get() as *const c_void, blocks as i32)
        }
        if handle.is_null() {
            return None;
        } else {
            unsafe {
                return Some(Handle {
                    handle: UnsafeCell::new(RsHandle::from_raw(handle as *const c_void)),
                });
            }
        }
    }
}

impl Handle {
    // register a block as part of the transaction associated with this handle
    pub fn journal_write(&mut self, bh: &BufferHead) -> i32 {
        unsafe {
            return rs_jbd2_journal_get_write_access(self.handle.get() as *const c_void, bh.get_raw());
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
