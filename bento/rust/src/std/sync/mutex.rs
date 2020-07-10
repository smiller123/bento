use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

use crate::kernel::kobj::*;
use crate::kernel::sync::*;

use crate::std::sys_common::poison::{LockResult, TryLockError, TryLockResult};

/// A wrapper around the kernel mutex.
///
/// This allows multiple readers at once, but only one writer.
///
/// The type parameter `T` represents the data that this lock protects. It is
/// required that `T` satisfies `Send` to be shared across tasks and `Sync` to
/// allow concurrent access through readers. The RAII guards returned from the
/// locking methods implement `Deref` (and `DerefMut` for the `write` methods)
/// to allow access to the contained of the lock.
///
/// Based on spin-rs, a no_std Rust implementation of several locks.
///
/// # Examples
///
/// ```
/// use bento::std::sync::Mutex;
///
/// let lock = Mutex::new(5);
///
/// // many reader locks can be held at once
/// {
///     lock.init();
///     let r1 = lock.read();
///     let r2 = lock.read();
///     assert_eq!(*r1, 5);
///     assert_eq!(*r2, 5);
/// } // read locks are dropped at this point
///
/// // only one write lock may be held, however
/// {
///     let mut w = lock.write();
///     *w += 1;
///     assert_eq!(*w, 6);
/// } // write lock is dropped here
/// ```
pub struct Mutex<T: ?Sized> {
    lock: UnsafeCell<Option<RsRwSemaphore>>,
    data: UnsafeCell<T>,
}

/// A guard to which the protected data can be written
///
/// When the guard falls out of scope it will release the lock.
pub struct MutexGuard<'a, T: 'a + ?Sized> {
    lock: &'a Mutex<T>,
}

// Same unsafe impls as `std::sync::Mutex`
unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    /// Creates a new mutex wrapping the supplied data.
    ///
    /// May be used statically:
    ///
    /// ```
    /// use bento::std::sync::Mutex;
    ///
    /// static SEM: Mutex<()> = Mutex::new(());
    ///
    /// fn demo() {
    ///     SEM.init();
    ///     let lock = RW_LOCK.read();
    ///     // do something with lock
    ///     drop(lock);
    /// }
    /// ```
    pub fn new(user_data: T) -> Mutex<T> {
        Mutex {
            lock: UnsafeCell::new(get_semaphore()),
            data: UnsafeCell::new(user_data),
        }
    }
}

impl<T: ?Sized> Mutex<T> {
    /// Lock this mutex with exclusive write access, blocking the current
    /// thread until it can be acquired.
    ///
    /// This function will not return while other writers or other readers
    /// currently have access to the lock.
    ///
    /// Returns an RAII guard which will drop the write access of this mutex
    /// when dropped.
    ///
    /// ```
    /// use bento::std::sync::Mutex;
    ///
    /// let mylock = Mutex::new(0);
    /// mylock.init();
    /// {
    ///     let mut data = mylock.lock();
    ///     // The lock is now locked and the data can be written
    ///     *data += 1;
    ///     // The lock is dropped
    /// }
    /// ```
    pub fn lock(&self) -> LockResult<MutexGuard<T>> {
        unsafe {
            let _ = down_write(&*self.lock.get());
        }
        Ok(MutexGuard {
            lock: self,
        })
    }

    /// Tries to lock this mutex with exclusive write access without
    /// blocking. If the mutex is already held by another thread, this
    /// won't lock the mutex.
    ///
    /// Returns an RAII guard which will drop the write access of this mutex
    /// when dropped if the lock was successful. Otherwise, returns None.
    ///
    /// ```
    /// use bento::std::sync::Mutex;
    ///
    /// let mylock = Mutex::new(0);
    /// mylock.init();
    /// {
    ///     let mut data = mylock.try_lock();
    ///     // The lock is now locked and the data can be written
    ///     *data += 1;
    ///     // The lock is dropped
    /// }
    /// ```
    #[inline]
    pub fn try_lock(&self) -> TryLockResult<MutexGuard<T>> {
        let write_ret = unsafe { down_write_trylock(&*self.lock.get()) };
        if write_ret == Ok(1) {
            return Ok(MutexGuard {
                lock: self,
            });
        } else {
            return Err(TryLockError::WouldBlock);
        }
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// Since this call borrows the `Mutex` mutably, no actual locking needs to
    /// take place -- the mutable borrow statically guarantees no locks exist.
    ///
    /// # Examples
    ///
    /// ```
    /// use bento::std::sync::Mutex;
    ///
    /// let mut lock = Mutex::new(0);
    /// *lock.get_mut() = 10;
    /// assert_eq!(*lock.read(), 10);
    /// ```
    pub fn get_mut(&mut self) -> LockResult<&mut T> {
        // We know statically that there are no other references to `self`, so
        // there's no need to lock the inner lock.
        unsafe { Ok(&mut *self.data.get()) }
    }
}

impl<'rw, T: ?Sized> Deref for MutexGuard<'rw, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'rw, T: ?Sized> DerefMut for MutexGuard<'rw, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'rw, T: ?Sized> Drop for MutexGuard<'rw, T> {
    fn drop(&mut self) {
        unsafe {
            let _ = up_write(&*self.lock.lock.get());
        }
    }
}

pub fn guard_lock<'a, T: ?Sized>(guard: &MutexGuard<'a, T>) -> &'a Option<RsRwSemaphore> {
    unsafe { &*guard.lock.lock.get() }
}
