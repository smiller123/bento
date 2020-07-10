use core::cell::UnsafeCell;
use core::ops::FnMut;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::std::sync::*;

use kernel::ffi::{rs_put_wait_queue_head, rs_get_wait_queue_head};
use kernel::kobj::*;
use kernel::raw::*;
use kernel::sync::{up_write, down_write};

pub static BLOCKER: AtomicBool = AtomicBool::new(false);

extern "C" fn wait_cont() -> bool {
    return BLOCKER.load(Ordering::SeqCst);
}

/// Wrapper around the kernel `wait_queue_head`.
#[derive(Debug)]
pub struct Condvar {
    wq_head: UnsafeCell<Option<RsWaitQueueHead>>,
}

impl Condvar {
    pub fn new() -> Condvar {
        Condvar {
            wq_head: UnsafeCell::new(get_wait_queue_head()),
        }
    }

    /// Wake up the Condvar.
    ///
    /// Notifies the Condvar that the condition may be true. Wakes up one thread waiting on the
    /// Condvar.
    pub fn notify_one(&self) {
        unsafe {
            BLOCKER.store(true, Ordering::SeqCst);
            if let Some(head) = &*self.wq_head.get() {
                head.wake_up();
            }
        }
    }

    /// Wake up the Condvar.
    ///
    /// Notifies the Condvar that the condition may be true. Wakes up all threads waiting on the
    /// Condvar.
    pub fn notify_all(&self) {
        unsafe {
            BLOCKER.store(true, Ordering::SeqCst);
            if let Some(head) = &*self.wq_head.get() {
                head.wake_up_all();
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
    /// let wait_q = Condvar::new();
    /// wait_q.wait_while(wait_condition);
    /// ...
    /// wait_q.notify_one();
    /// ```
    pub fn wait_while<'a, T, F>(
        &self,
        mut guard: MutexGuard<'a, T>,
        mut condition: F
    ) -> LockResult<MutexGuard<'a, T>>
    where
        F: FnMut(&mut T) -> bool
    {
        BLOCKER.store(false, Ordering::SeqCst);
        loop {
            let sem = guard_lock(&guard);
            let _ = up_write(sem);
            unsafe {
                if let Some(head) = &*self.wq_head.get() {
                    head.wait_event(wait_cont);
                }
            }
            let _ = down_write(sem);
            if condition(&mut *guard) {
                break;
            }
        }
        return Ok(guard)
    }

    pub fn wait<'a, T>(
        &self,
        guard: MutexGuard<'a, T>
    ) -> LockResult<MutexGuard<'a, T>> {
        let sem = guard_lock(&guard);
        let _ = up_write(sem);
        BLOCKER.store(false, Ordering::SeqCst);
        unsafe {
            if let Some(head) = &*self.wq_head.get() {
                head.wait_event(wait_cont);
            }
        }
        let _ = down_write(sem);
        return Ok(guard)
    }
}

unsafe impl Send for Condvar {}
unsafe impl Sync for Condvar {}

impl Drop for Condvar {
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
