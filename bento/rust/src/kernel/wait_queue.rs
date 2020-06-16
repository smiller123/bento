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

/// Wrapper around the kernel `wait_queue_head`.
///
/// This can be created statically, but must be intialized after creation.
#[derive(Debug)]
pub struct WaitQueue {
    wq_head: UnsafeCell<Option<RsWaitQueueHead>>,
}

impl WaitQueue {
    pub const fn new() -> WaitQueue {
        WaitQueue {
            wq_head: UnsafeCell::new(None),
        }
    }

    /// Initialize a WaitQueue.
    ///
    /// Since this initialization call a C function, it must be done outside of `new`. TODO: try to
    /// get all initialization done in `new`.
    pub fn init(&self) {
        let wq_head = unsafe { &mut *self.wq_head.get() };
        *wq_head = get_wait_queue_head();
    }

    /// Wake up the WaitQueue.
    ///
    /// Notifies the WaitQueue that the condition may be true. Wakes up all threads waiting on the
    /// WaitQueue.
    pub fn wake_up(&self) {
        unsafe {
            if let Some(head) = &*self.wq_head.get() {
                head.wake_up();
            }
        }
    }

    /// Block waiting on an event.
    ///
    /// `condition` should be a reference to an `extern "C"` function that returns a `bool`.
    ///
    /// This function will block until the condition function returns true. Waiting threads should
    /// check the condition again after woken up.
    ///
    /// Examples:
    /// ```
    /// extern "C" fn wait_condition() -> bool {
    ///     return true;
    /// }
    ///
    /// let wait_q = WaitQueue::new();
    /// wait_q.init();
    /// wait_q.wait_event(wait_condition);
    /// ...
    /// wait_q.wake_up();
    /// ```
    pub fn wait_event(&self, condition: Condition) {
        unsafe {
            if let Some(head) = &*self.wq_head.get() {
                head.wait_event(condition);
            }
        }
    }
}

unsafe impl Send for WaitQueue {}
unsafe impl Sync for WaitQueue {}

impl Drop for WaitQueue {
    fn drop(&mut self) {
        let wq_head = unsafe { &mut *self.wq_head.get() };
        put_wait_queue_head(wq_head);
    }
}

fn get_wait_queue_head() -> Option<RsWaitQueueHead> {
    let wq_head;
    unsafe {
        wq_head = rs_get_wait_queue_head();
    }
    if wq_head.is_null() {
        return None;
    } else {
        unsafe {
            return Some(RsWaitQueueHead::from_raw(wq_head as *const c_void));
        }
    }
}

fn put_wait_queue_head(wq_head_opt: &mut Option<RsWaitQueueHead>) {
    if let Some(wq_head) = wq_head_opt {
        unsafe {
            rs_put_wait_queue_head(wq_head.get_raw());
        }
    }
}
