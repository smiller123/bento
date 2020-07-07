use kernel::ffi::*;
use kernel::raw::c_void;

/// A wrapper around the kernel `timespec64`.
pub struct Timespec64 {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

impl Timespec64 {
    pub const fn new() -> Self {
        Timespec64 {
            tv_sec: 0,
            tv_nsec: 0,
        }
    }
}

/// Calculate the difference between two `Timespec64` in nanoseconds.
pub fn diff_ns(lhs: &Timespec64, rhs: &Timespec64) -> i64 {
    let secs = lhs.tv_sec - rhs.tv_sec;
    let nsecs = lhs.tv_nsec - rhs.tv_nsec;
    secs * 1000000000 + nsecs
}

/// Get the current time of day in nanoseconds.
pub fn getnstimeofday64_rs(ts: &mut Timespec64) {
    unsafe { getnstimeofday64(ts as *mut Timespec64 as *mut c_void) }
}
