use crate::kernel::time::*;
use crate::kernel::ffi::current_kernel_time_rs;

pub const UNIX_EPOCH: SystemTime = SystemTime {
    secs: 0,
    nanos: 0,
};

pub struct SystemTime {
    pub secs: u64,
    pub nanos: u32,
}

impl SystemTime {
    pub fn now() -> SystemTime {
        let now = unsafe {
            current_kernel_time_rs()
        };
        SystemTime {
            secs: now.tv_sec as u64,
            nanos: now.tv_nsec as u32,
        }
    }

    pub fn duration_since(&self, earlier: SystemTime)
        -> Result<Duration, SystemTimeError> {
        if let Some(mut secs) = self.secs.checked_sub(earlier.secs) {
            let nanos = if self.nanos >= earlier.nanos {
                self.nanos - earlier.nanos
            } else {
                if let Some(sub_secs) = secs.checked_sub(1) {
                    secs = sub_secs;
                    self.nanos + 1000000000 - earlier.nanos
                } else {
                    return Err(SystemTimeError{});
                }
            };
            Ok(Duration { secs, nanos })
        } else {
            Err(SystemTimeError{})
        }
    }
}

pub struct SystemTimeError{}

pub struct Duration {
    pub secs: u64,
    pub nanos: u32,
}

impl Duration {
    pub fn as_secs(&self) -> u64 {
        self.secs
    }

    pub fn subsec_nanos(&self) -> u32 {
        self.nanos
    }
}
