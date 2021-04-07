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
//use crate::bindings::PAGE_SIZE;

pub struct KernelAllocator;

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() <= 8*4096 {
            __kmalloc(layout.size(), 0x90) as *mut u8
        } else {
            vmalloc(layout.size() as raw::c_size_t) as *mut u8
        }

    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if layout.size() <= 8*4096 {
            kfree(ptr as *const raw::c_void);
        } else {
            vfree(ptr as *const raw::c_void);
        }
    }
}

#[alloc_error_handler]
fn oom(_layout: Layout) -> ! {
    loop {}
}
