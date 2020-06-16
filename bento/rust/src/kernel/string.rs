/*
 * SPDX-License-Identifier: GPL-2.0
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 *
 */

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
