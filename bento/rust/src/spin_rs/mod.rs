//use spin::RwLock;

use kernel::ffi;
use bindings as c;

pub struct RwLock<T: ?Sized> {
    id: u64,
    lock: spin::RwLock<T>,
    //lock: UnsafeCell<Option<RsRwSemaphore>>,
    //data: UnsafeCell<T>,
}

impl<T> RwLock<T> {
    pub fn new(user_data: T) -> Self {
        let id = unsafe {
            ffi::rs_ktime_get_ns()
        };
        let lock = RwLock {
            id: id,
            lock: spin::RwLock::new(user_data),
        };
        #[cfg(feature = "record")]
        unsafe {
            //let pid = ffi::current_pid();
            let curr = ffi::rs_current();
            //let curr_cpu = ffi::rs_smp_processor_id();
            //let mut write_str = alloc::format!("lock_new: {} {:?}\n\0", pid, &lock.lock as *const spin::RwLock<T> as *const u8 as u64);
            let mut write_str = alloc::format!("lock_new: {:?} {:?}\n\0", curr, lock.id);
            //c::printk_deferred(write_str.as_ptr() as *const i8);
            c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
        }
        lock
    }
}

impl<T: ?Sized> RwLock<T> {
    #[inline]
    pub fn read(&self) -> spin::RwLockReadGuard<T> {
        let guard = self.lock.read();
        #[cfg(feature = "record")]
        unsafe {
            //let pid = ffi::current_pid();
            let curr = ffi::rs_current();
            //let curr_cpu = ffi::rs_smp_processor_id();
            //let mut write_str = alloc::format!("read_lock: {} {:?}\n\0", pid, &self.lock as *const spin::RwLock<T> as *const u8 as u64);
            let mut write_str = alloc::format!("read_lock: {:?} {}\n\0", curr, self.id);
            //c::printk_deferred(write_str.as_ptr() as *const i8);
            c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
        }
        guard
    }

    #[inline]
    pub fn write(&self) -> spin::RwLockWriteGuard<T> {
        let guard = self.lock.write();
        #[cfg(feature = "record")]
        unsafe {
            //let pid = ffi::current_pid();
            let curr = ffi::rs_current();
            //let curr_cpu = ffi::rs_smp_processor_id();
            //let mut write_str = alloc::format!("write_lock: {} {:?}\n\0", pid, &self.lock as *const spin::RwLock<T> as *const u8 as u64);
            let mut write_str = alloc::format!("write_lock: {:?} {}\n\0", curr, self.id);
            //c::printk_deferred(write_str.as_ptr() as *const i8);
            c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
        }
        guard
    }
}
