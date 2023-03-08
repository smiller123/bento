/*
 * SPDX-License-Identifier: GPL-2.0
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 *
 */

use kernel;
use kernel::fs::*;
use kernel::raw::*;

use core::slice;

use crate::libc;
use core::ops::Deref;
use core::ops::DerefMut;

// /// A wrapper around the kernel `super_block` type.
def_kernel_obj_type!(RsSuperBlock);
// /// A wrapper around the kernel `buffer_head` type.
// ///
// /// Acquired by using `sb_bread_rust`. Since each bread must be accompanied by an associated brelse
// /// to release the buffer, this calls `brelse` on `drop`.
def_kernel_obj_type!(BufferHead);
// /// A wrapper around the kernel `semaphore` type.
def_kernel_obj_type!(RsRwSemaphore);
// /// A wrapper around the kernel `rwlock_t` type.
def_kernel_obj_type!(RsRwLock);
// /// A wrapper around the kernel `wait_queue_head` type.
def_kernel_obj_type!(RsWaitQueueHead);
// /// A wrapper around the kernel `block_device` type.
def_kernel_obj_type!(RsBlockDevice);

// /// A wrapper around the kernel journal_t type TODO
def_kernel_obj_type!(RsJournal);
// /// A wrapper around the kernel handle_t type
def_kernel_obj_type!(RsHandle);

def_kernel_val_getter!(BufferHead, b_data, buffer_head, *const c_void);
def_kernel_val_getter!(BufferHead, b_size, buffer_head, c_size_t);
def_kernel_val_getter!(BufferHead, b_blocknr, buffer_head, c_size_t);


use kernel::ffi::*;

def_kernel_obj_getter!(RsSuperBlock, s_bdev, super_block, RsBlockDevice);
def_kobj_op!(RsSuperBlock, dump, rs_dump_super_block, ());

def_kernel_val_getter!(RsBlockDevice, bd_dev, block_device, u32);

def_kobj_op!(BufferHead, brelse, __brelse, ());
def_kobj_op!(BufferHead, mark_buffer_dirty, mark_buffer_dirty, ());
def_kobj_op!(BufferHead, set_buffer_uptodate, rs_set_buffer_uptodate, ());
def_kobj_op!(BufferHead, sync_dirty_buffer, sync_dirty_buffer, i32);

def_kobj_immut_op!(RsRwSemaphore, down_read, down_read, ());
def_kobj_immut_op!(RsRwSemaphore, up_read, up_read, ());
def_kobj_immut_op!(RsRwSemaphore, down_write, down_write, ());
def_kobj_immut_op!(RsRwSemaphore, down_write_trylock, down_write_trylock, i32);
def_kobj_immut_op!(RsRwSemaphore, down_read_trylock, down_read_trylock, i32);
def_kobj_immut_op!(RsRwSemaphore, up_write, up_write, ());
def_kobj_op!(RsRwSemaphore, put, rs_put_semaphore, ());

def_kobj_immut_op!(RsRwLock, read_lock, rs_read_lock, ());
def_kobj_immut_op!(RsRwLock, read_unlock, rs_read_unlock, ());
def_kobj_immut_op!(RsRwLock, write_lock, rs_write_lock, ());
def_kobj_immut_op!(RsRwLock, write_unlock, rs_write_unlock, ());
def_kobj_op!(RsRwLock, put, rs_put_rwlock, ());

def_kobj_immut_op!(RsWaitQueueHead, wake_up, rs_wake_up, ());
def_kobj_immut_op!(RsWaitQueueHead, wake_up_all, rs_wake_up_all, ());

impl RsBlockDevice {
    pub fn new(name: &str) -> Self {
        unsafe {
            Self::from_raw(lookup_bdev(name.as_ptr() as *const c_char, FMODE_READ | FMODE_WRITE | FMODE_EXCL))
        }
    }
    pub fn bread(&self, blockno: u64, size: u32) -> Option<BufferHead> {
        let bh = unsafe {
            bread_helper(self.get_raw() as *const c_void, blockno, size)
        };
        if bh.is_null() {
            return None;
        } else {
            unsafe {
                return Some(BufferHead::from_raw(bh as *const c_void));
            }
        }
    }

    pub fn getblk(&self, blockno: u64, size: u32) -> Option<BufferHead> {
        let bh = unsafe {
            rs_getblk(self.get_raw() as *const c_void, blockno, size)
        };
        if bh.is_null() {
            return None;
        } else {
            unsafe {
                return Some(BufferHead::from_raw(bh as *const c_void));
            }
        }
    }

    pub fn put(&self) {
        unsafe {
            blkdev_put(self.get_raw(), 0x80);
        }
    }
}

unsafe impl Send for RsBlockDevice {}
unsafe impl Sync for RsBlockDevice {}

impl Drop for BufferHead {
    fn drop(&mut self) {
        self.brelse();
    }
}

impl Drop for RsRwSemaphore {
    fn drop(&mut self) {
        self.put();
    }
}

/// A Rust representation of a C-style, null-terminated string.
///
/// Modeled after std::ffi::CStr.
pub struct CStr {
    inner: *const c_char,
}

impl CStr {
    /// Return the pointer representation of the CStr
    pub fn to_raw(&self) -> *const c_char {
        self.inner
    }

    pub unsafe fn from_raw(inner: *const c_char) -> Self {
        Self { inner: inner }
    }

    /// Calculate the length of the CStr.
    pub fn len(&self) -> usize {
        let mut i = 0;
        let mut ptr = self.inner;
        unsafe {
            while *ptr != 0 {
                i += 1;
                ptr = ptr.offset(1);
            }
        }
        return i;
    }

    /// Convert the CStr into a `u8` slice.
    pub fn to_bytes_with_nul(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.inner as *const u8, self.len()) }
    }

    /// Create a CStr from a `u8` slice.
    ///
    /// Will return an error if the byte array doesn't contain a null character.
    pub fn from_bytes_with_nul(bytes: &[u8]) -> Result<CStr, libc::c_int> {
        let mut nul_pos = None;
        for (iter, byte) in bytes.iter().enumerate() {
            if *byte == 0 {
                nul_pos = Some(iter);
                break;
            }
        }
        if let Some(nul_pos) = nul_pos {
            if nul_pos + 1 != bytes.len() {
                return Err(libc::EIO);
            }
            Ok(CStr {
                inner: bytes.as_ptr() as *const i8,
            })
        } else {
            return Err(libc::EIO);
        }
    }
}

pub struct BHLockGuard<'a> {
    bh: &'a mut BufferHead
}

impl Drop for BHLockGuard<'_> {
    fn drop(&mut self) {
        self.bh.unlock();
    }
}

impl Deref for BHLockGuard<'_> {
    type Target = BufferHead;

    fn deref(&self) -> &BufferHead {
        unsafe { &*self.bh }
    }
}

impl DerefMut for BHLockGuard<'_> {
    fn deref_mut(&mut self) -> &mut BufferHead {
        unsafe { &mut *self.bh }
    }
}


impl BufferHead {
    /// Return the associated data as a `u8` slice.
    pub fn data(&self) -> &[u8] {
        let b_data = self.b_data();
        let size = self.b_size();
        unsafe {
            return slice::from_raw_parts::<c_uchar>(b_data as *mut u8, size as usize);
        }
    }

    /// Return the associated data as a mutable `u8` slice.
    pub fn data_mut(&mut self) -> &mut [u8] {
        let b_data = self.b_data();
        let size = self.b_size();
        unsafe {
            return slice::from_raw_parts_mut::<c_uchar>(b_data as *mut c_uchar, size as usize);
        }
    }

    pub fn blocknr(&self) -> u64 {
        return self.b_blocknr();
    }

    pub fn lock(&mut self) -> BHLockGuard<'_> {
        unsafe {
            rs_lock_buffer(self.get_raw() as *const c_void);
        }
        return BHLockGuard {
            bh: self
        }
    }

    pub fn unlock(&mut self) {
        unsafe {
            unlock_buffer(self.get_raw() as *const c_void);
        }
    }
}

impl RsWaitQueueHead {
    /// Block waiting on an event.
    ///
    /// This calls the `wait_event` function in the kernel. The function will unblock when the
    /// condition may be true. Users should check the condition again after unblocking.
    pub unsafe fn wait_event(&self, condition: Condition) {
        rs_wait_event(self.get_raw() as *const c_void, condition);
    }
}
