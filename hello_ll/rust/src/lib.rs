/*
 * SPDX-License-Identifier: GPL-2.0 OR MIT
 *
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 */

#![feature(lang_items)]
#![feature(concat_idents)]
#![feature(const_fn)]
#![feature(allocator_api)]
#![feature(alloc_error_handler)]
#![feature(alloc_layout_extra)]
#![feature(panic_info_message)]
#![no_std]

use bento;
use bento::bentofs::*;
use bento::c_str;
use bento::println;

pub mod hello_ll;

use hello_ll::HELLO_LL_OPS;

pub static FS_NAME: &'static str = c_str!("hello_ll");

#[no_mangle]
pub fn rust_main() {
    println!("Hello from Rust");
    register_bento_fs_rs(FS_NAME, &HELLO_LL_OPS);
}

#[no_mangle]
pub fn rust_exit() {
    unregister_bento_fs_rs(FS_NAME);
}
