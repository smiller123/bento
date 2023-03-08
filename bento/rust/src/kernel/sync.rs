/*
 * SPDX-License-Identifier: GPL-2.0
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 *
 */

use kernel::ffi::*;
use kernel::kobj::*;
use kernel::raw::*;

pub fn get_semaphore() -> Option<RsRwSemaphore> {
    let sem;
    unsafe {
        sem = rs_get_semaphore();
    }
    if sem.is_null() {
        return None;
    } else {
        unsafe {
            return Some(RsRwSemaphore::from_raw(sem as *const c_void));
        }
    }
}

pub fn put_semaphore(semaphore: Option<RsRwSemaphore>) -> Result<(), i32> {
    if let Some(sem) = semaphore {
        unsafe {
            rs_put_semaphore(sem.get_raw());
        }
    }
    Ok(())
}

pub fn down_read(semaphore: &Option<RsRwSemaphore>) -> Result<(), i32> {
    if let Some(sem) = semaphore {
        sem.down_read();
    }
    Ok(())
}

pub fn up_read(semaphore: &Option<RsRwSemaphore>) -> Result<(), i32> {
    if let Some(sem) = semaphore {
        sem.up_read();
    }
    Ok(())
}

pub fn down_write(semaphore: &Option<RsRwSemaphore>) -> Result<(), i32> {
    if let Some(sem) = semaphore {
        sem.down_write();
    }
    Ok(())
}

pub fn down_write_trylock(semaphore: &Option<RsRwSemaphore>) -> Result<i32, i32> {
    if let Some(sem) = semaphore {
        let ret = sem.down_write_trylock();
        return Ok(ret);
    }
    Ok(0)
}

pub fn down_read_trylock(semaphore: &Option<RsRwSemaphore>) -> Result<i32, i32> {
    if let Some(sem) = semaphore {
        let ret = sem.down_read_trylock();
        return Ok(ret);
    }
    Ok(0)
}

pub fn up_write(semaphore: &Option<RsRwSemaphore>) -> Result<(), i32> {
    if let Some(sem) = semaphore {
        sem.up_write();
    }
    Ok(())
}

pub fn get_rwlock() -> Option<RsRwLock> {
    let lock;
    unsafe {
        lock = rs_get_rwlock();
    }
    if lock.is_null() {
        return None;
    } else {
        unsafe {
            return Some(RsRwLock::from_raw(lock as *const c_void));
        }
    }
}

pub fn put_rwlock(rwlock: Option<RsRwLock>) -> Result<(), i32> {
    if let Some(lock) = rwlock {
        unsafe {
            rs_put_rwlock(lock.get_raw());
        }
    }
    Ok(())
}

pub fn read_lock(rwlock: &Option<RsRwLock>) -> Result<(), i32> {
    if let Some(lock) = rwlock {
        lock.read_lock();
    }
    Ok(())
}

pub fn read_unlock(rwlock: &Option<RsRwLock>) -> Result<(), i32> {
    if let Some(lock) = rwlock {
        lock.read_unlock();
    }
    Ok(())
}

pub fn write_lock(rwlock: &Option<RsRwLock>) -> Result<(), i32> {
    if let Some(lock) = rwlock {
        lock.write_lock();
    }
    Ok(())
}

pub fn write_unlock(rwlock: &Option<RsRwLock>) -> Result<(), i32> {
    if let Some(lock) = rwlock {
        lock.write_unlock();
    }
    Ok(())
}
