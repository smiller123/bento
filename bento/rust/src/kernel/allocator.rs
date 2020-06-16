/*
 * SPDX-License-Identifier: GPL-2.0
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 *
 * Based on code from fishinabarrel/linux-kernel-module-rust on Github
 */

use core::alloc::{GlobalAlloc, Layout};
use kernel::ffi::*;
use kernel::raw;

pub struct KernelAllocator;

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        __kmalloc(layout.size() as raw::c_size_t, 0x90) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        kfree(ptr as *const raw::c_void);
    }
}

#[alloc_error_handler]
fn oom(_layout: Layout) -> ! {
    loop {}
}
