/*
 * SPDX-License-Identifier: GPL-2.0
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 *
 * Based on a no_std lock implementation provided by the spin_rs crate.
 */

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

use crate::kernel::kobj::*;
use crate::kernel::sync::*;
use crate::std::sys_common::poison::{LockResult, TryLockError, TryLockResult};

/// A wrapper around the kernel semaphore.
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
/// use bento::std::sync::RwLock;
///
/// let lock = RwLock::new(5);
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
pub struct RwLock<T: ?Sized> {
    //lock: UnsafeCell<Option<RsRwSemaphore>>,
    lock: UnsafeCell<Option<RsRwLock>>,
    data: UnsafeCell<T>,
}

/// A guard from which the protected data can be read
///
/// When the guard falls out of scope it will decrement the read count,
/// potentially releasing the lock.
pub struct RwLockReadGuard<'a, T: ?Sized> {
    lock: &'a RwLock<T>,
}

/// A guard to which the protected data can be written
///
/// When the guard falls out of scope it will release the lock.
pub struct RwLockWriteGuard<'a, T: 'a + ?Sized> {
    lock: &'a RwLock<T>,
}

// Same unsafe impls as `std::sync::RwLock`
unsafe impl<T: ?Sized + Send> Send for RwLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    /// Creates a new semaphore wrapping the supplied data.
    ///
    /// May be used statically:
    ///
    /// ```
    /// use bento::std::sync::RwLock;
    ///
    /// static SEM: RwLock<()> = RwLock::new(());
    ///
    /// fn demo() {
    ///     SEM.init();
    ///     let lock = RW_LOCK.read();
    ///     // do something with lock
    ///     drop(lock);
    /// }
    /// ```
    pub fn new(user_data: T) -> RwLock<T> {
        RwLock {
            lock: UnsafeCell::new(get_rwlock()),
            data: UnsafeCell::new(user_data),
        }
    }
}

impl<T: ?Sized> RwLock<T> {
    /// Locks this semaphore with shared read access, blocking the current thread
    /// until it can be acquired.
    ///
    /// The calling thread will be blocked until there are no more writers which
    /// hold the lock. There may be other readers currently inside the lock when
    /// this method returns. This method does not provide any guarantees with
    /// respect to the ordering of whether contentious readers or writers will
    /// acquire the lock first.
    ///
    /// Returns an RAII guard which will release this thread's shared access
    /// once it is dropped.
    ///
    /// ```
    /// use bento::std::sync::RwLock;
    ///
    /// let mylock = RwLock::new(0);
    /// mylock.init();
    /// {
    ///     let mut data = mylock.read().unwrap();
    ///     // The lock is now locked and the data can be read
    ///     println!("{}", *data);
    ///     // The lock is dropped
    /// }
    /// ```
    #[inline]
    pub fn read(&self) -> LockResult<RwLockReadGuard<'_, T>> {
        unsafe {
            //let _ = down_read(&*self.lock.get());
            let _ = read_lock(&*self.lock.get());
        }
        Ok(RwLockReadGuard {
            lock: self,
        })
    }

    /// Tries to lock this semaphore with read access without
    /// blocking. If the lock is already held by another thread, this
    /// won't lock.
    ///
    /// Returns an RAII guard which will drop the read access of this lock
    /// when dropped if the lock was successful. Otherwise, returns None.
    ///
    /// ```
    /// use bento::std::sync::RwLock;
    ///
    /// let mylock = RwLock::new(0);
    /// mylock.init();
    /// {
    ///     let mut data = mylock.try_read().unwrap();
    ///     // The lock is now locked and the data can be written
    ///     *data += 1;
    ///     // The lock is dropped
    /// }
    /// ```
    //#[inline]
    //pub fn try_read(&self) -> TryLockResult<RwLockReadGuard<'_, T>> {
    //    let write_ret = unsafe { down_read_trylock(&*self.lock.get()) };
    //    if write_ret == Ok(1) {
    //        return Ok(RwLockReadGuard {
    //            lock: self,
    //        });
    //    } else {
    //        return Err(TryLockError::WouldBlock);
    //    }
    //}

    /// Lock this semaphore with exclusive write access, blocking the current
    /// thread until it can be acquired.
    ///
    /// This function will not return while other writers or other readers
    /// currently have access to the lock.
    ///
    /// Returns an RAII guard which will drop the write access of this semaphore
    /// when dropped.
    ///
    /// ```
    /// use bento::std::sync::RwLock;
    ///
    /// let mylock = RwLock::new(0);
    /// mylock.init();
    /// {
    ///     let mut data = mylock.write().unwrap();
    ///     // The lock is now locked and the data can be written
    ///     *data += 1;
    ///     // The lock is dropped
    /// }
    /// ```
    #[inline]
    pub fn write(&self) -> LockResult<RwLockWriteGuard<'_, T>> {
        unsafe {
            //let _ = down_write(&*self.lock.get());
            let _ = write_lock(&*self.lock.get());
        }
        Ok(RwLockWriteGuard {
            lock: self,
        })
    }

    /// Tries to lock this semaphore with exclusive write access without
    /// blocking. If the semaphore is already held by another thread, this
    /// won't lock the semaphore.
    ///
    /// Returns an RAII guard which will drop the write access of this semaphore
    /// when dropped if the lock was successful. Otherwise, returns None.
    ///
    /// ```
    /// use bento::std::sync::RwLock;
    ///
    /// let mylock = RwLock::new(0);
    /// mylock.init();
    /// {
    ///     let mut data = mylock.try_write().unwrap();
    ///     // The lock is now locked and the data can be written
    ///     *data += 1;
    ///     // The lock is dropped
    /// }
    /// ```
    //#[inline]
    //pub fn try_write(&self) -> TryLockResult<RwLockWriteGuard<'_, T>> {
    //    let write_ret = unsafe { down_write_trylock(&*self.lock.get()) };
    //    if write_ret == Ok(1) {
    //        return Ok(RwLockWriteGuard {
    //            lock: self,
    //        });
    //    } else {
    //        return Err(TryLockError::WouldBlock);
    //    }
    //}

    /// Returns a mutable reference to the underlying data.
    ///
    /// Since this call borrows the `RwLock` mutably, no actual locking needs to
    /// take place -- the mutable borrow statically guarantees no locks exist.
    ///
    /// # Examples
    ///
    /// ```
    /// use bento::std::sync::RwLock;
    ///
    /// let mut lock = RwLock::new(0);
    /// *lock.get_mut() = 10;
    /// assert_eq!(*lock.read(), 10);
    /// ```
    pub fn get_mut(&mut self) -> LockResult<&mut T> {
        // We know statically that there are no other references to `self`, so
        // there's no need to lock the inner lock.
        unsafe { Ok(&mut *self.data.get()) }
    }
}

impl<T: ?Sized> Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for RwLockReadGuard<'_,T> {
    fn drop(&mut self) {
        unsafe {
            //let _ = up_read(&*self.lock.lock.get());
            let _ = read_unlock(&*self.lock.lock.get());
        }
    }
}

impl<T: ?Sized> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        unsafe {
            //let _ = up_write(&*self.lock.lock.get());
            let _ = write_unlock(&*self.lock.lock.get());
        }
    }
}
