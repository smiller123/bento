use core::str;

use kernel::ffi::*;
use kernel::raw::*;

pub fn strcmp_rs(s1: *const c_char, s2: *const c_char) -> i32 {
    if s1.is_null() || s2.is_null() {
        return -1;
    }
    unsafe {
        return strcmp(s1, s2);
    }
}

pub fn str_from_utf8(s: &[u8]) -> &str {
    unsafe {
        return str::from_utf8_unchecked(s);
    }
}
