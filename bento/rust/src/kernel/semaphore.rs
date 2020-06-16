/*
 * SPDX-License-Identifier: GPL-2.0
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 *
 * Based on a no_std lock implementation provided by the spin_rs crate.
 */

use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

use kernel::kobj::*;
use kernel::sync::*;

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
/// use bento::kernel;
///
/// let lock = kernel::Semaphore::new(5);
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
pub struct Semaphore<T: ?Sized> {
    lock: UnsafeCell<Option<RsRwSemaphore>>,
    data: UnsafeCell<T>,
}

/// A guard from which the protected data can be read
///
/// When the guard falls out of scope it will decrement the read count,
/// potentially releasing the lock.
#[derive(Debug)]
pub struct SemaphoreReadGuard<T: ?Sized> {
    lock: NonNull<Option<RsRwSemaphore>>,
    data: NonNull<T>,
}

/// A guard to which the protected data can be written
///
/// When the guard falls out of scope it will release the lock.
#[derive(Debug)]
pub struct SemaphoreWriteGuard<'a, T: 'a + ?Sized> {
    lock: NonNull<Option<RsRwSemaphore>>,
    data: NonNull<T>,
    #[doc(hidden)]
    _invariant: PhantomData<&'a mut T>,
}

// Same unsafe impls as `std::sync::RwLock`
unsafe impl<T: ?Sized + Send> Send for Semaphore<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for Semaphore<T> {}

impl<T> Semaphore<T> {
    /// Creates a new semaphore wrapping the supplied data.
    ///
    /// May be used statically:
    ///
    /// ```
    /// use bento::kernel;
    ///
    /// static SEM: kernel::Semaphore<()> = spin::Semaphore::new(());
    ///
    /// fn demo() {
    ///     SEM.init();
    ///     let lock = RW_LOCK.read();
    ///     // do something with lock
    ///     drop(lock);
    /// }
    /// ```
    pub const fn new(user_data: T) -> Semaphore<T> {
        Semaphore {
            lock: UnsafeCell::new(None),
            data: UnsafeCell::new(user_data),
        }
    }

    /// Initializes the semaphore.
    ///
    /// Must be called before the semaphore is used. To use a kernel semaphore,
    /// init_rwsem() must be called. Since this is a kernel function, it can't
    /// be called from new().
    ///
    /// TODO: Try to get rid of this by doing setup along with new.
    pub fn init(&self) {
        let lock = unsafe { &mut *self.lock.get() };
        *lock = get_semaphore();
    }
}

impl<T: ?Sized> Semaphore<T> {
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
    /// let mylock = kernel::Semaphore::new(0);
    /// mylock.init();
    /// {
    ///     let mut data = mylock.read();
    ///     // The lock is now locked and the data can be read
    ///     println!("{}", *data);
    ///     // The lock is dropped
    /// }
    /// ```
    #[inline]
    pub fn read(&self) -> SemaphoreReadGuard<T> {
        unsafe {
            let _ = down_read(&*self.lock.get());
        }
        SemaphoreReadGuard {
            lock: unsafe { NonNull::new_unchecked(self.lock.get()) },
            data: unsafe { NonNull::new_unchecked(self.data.get()) },
        }
    }

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
    /// let mylock = kernel::Semaphore::new(0);
    /// mylock.init();
    /// {
    ///     let mut data = mylock.write();
    ///     // The lock is now locked and the data can be written
    ///     *data += 1;
    ///     // The lock is dropped
    /// }
    /// ```
    #[inline]
    pub fn write(&self) -> SemaphoreWriteGuard<T> {
        unsafe {
            let _ = down_write(&*self.lock.get());
        }
        SemaphoreWriteGuard {
            lock: unsafe { NonNull::new_unchecked(self.lock.get()) },
            data: unsafe { NonNull::new_unchecked(self.data.get()) },
            _invariant: PhantomData,
        }
    }

    /// Tries to lock this semaphore with exclusive write access without
    /// blocking. If the semaphore is already held by another thread, this
    /// won't lock the semaphore.
    ///
    /// Returns an RAII guard which will drop the write access of this semaphore
    /// when dropped if the lock was successful. Otherwise, returns None.
    ///
    /// ```
    /// let mylock = kernel::Semaphore::new(0);
    /// mylock.init();
    /// {
    ///     let mut data = mylock.write();
    ///     // The lock is now locked and the data can be written
    ///     *data += 1;
    ///     // The lock is dropped
    /// }
    /// ```
    #[inline]
    pub fn try_write(&self) -> Option<SemaphoreWriteGuard<T>> {
        let write_ret = unsafe { down_write_trylock(&*self.lock.get()) };
        if write_ret == Ok(1) {
            return Some(SemaphoreWriteGuard {
                lock: unsafe { NonNull::new_unchecked(self.lock.get()) },
                data: unsafe { NonNull::new_unchecked(self.data.get()) },
                _invariant: PhantomData,
            });
        } else {
            return None;
        }
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// Since this call borrows the `Semaphore` mutably, no actual locking needs to
    /// take place -- the mutable borrow statically guarantees no locks exist.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut lock = spin::Semaphore::new(0);
    /// *lock.get_mut() = 10;
    /// assert_eq!(*lock.read(), 10);
    /// ```
    pub fn get_mut(&mut self) -> &mut T {
        // We know statically that there are no other references to `self`, so
        // there's no need to lock the inner lock.
        unsafe { &mut *self.data.get() }
    }
}

impl<T: ?Sized> Deref for SemaphoreReadGuard<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.data.as_ref() }
    }
}

impl<'sem, T: ?Sized> Deref for SemaphoreWriteGuard<'sem, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.data.as_ref() }
    }
}

impl<'sem, T: ?Sized> DerefMut for SemaphoreWriteGuard<'sem, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.data.as_mut() }
    }
}

impl<T: ?Sized> Drop for SemaphoreReadGuard<T> {
    fn drop(&mut self) {
        unsafe {
            let _ = up_read(self.lock.as_ref());
        }
    }
}

impl<'sem, T: ?Sized> Drop for SemaphoreWriteGuard<'sem, T> {
    fn drop(&mut self) {
        unsafe {
            let _ = up_write(self.lock.as_ref());
        }
    }
}
