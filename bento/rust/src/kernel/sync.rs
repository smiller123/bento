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

pub fn up_write(semaphore: &Option<RsRwSemaphore>) -> Result<(), i32> {
    if let Some(sem) = semaphore {
        sem.up_write();
    }
    Ok(())
}
