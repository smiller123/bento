use kernel::ffi;
use kernel::raw;

use core::mem;

fn spawn_thread_helper<T>(data: *mut raw::c_void) -> T
where
    T: Send + 'static
{
    unsafe {
        // let function run
        let func: fn() -> T = mem::transmute(data);
        let ret_val = func();
        // When function finishes, wait on stop
        loop {
            if ffi::kthread_should_stop() {
                return ret_val;
            }
            ffi::wait_for_interrupt();
        }
    }
}

pub fn spawn<F, T>(_f: F) -> JoinHandle
where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    unsafe {
        let c: extern "rust-call" fn(F, ()) -> T = <F as FnOnce<()>>::call_once;
        let data = mem::transmute(c);
        let tstruct = ffi::kthread_run_helper(
            spawn_thread_helper::<T> as *const raw::c_void,
            data,
            "".as_ptr() as *const raw::c_void
        );
        return JoinHandle { inner: tstruct as *mut raw::c_void };
    }
}

pub struct JoinHandle {
    inner: *mut raw::c_void
}

impl JoinHandle {
    pub fn join(&mut self) -> Result<i32, i32> {
        unsafe {
            return Ok(ffi::kthread_stop(self.inner as *mut raw::c_void));
        }
    }
}

unsafe impl Send for JoinHandle {}
unsafe impl Sync for JoinHandle {}
