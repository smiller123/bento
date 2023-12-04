use kernel::ffi::*;

pub fn num_online_cpus() -> i32 {
    return unsafe {
        rs_num_online_cpus() 
    };
}

