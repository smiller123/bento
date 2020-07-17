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

#[macro_use]
extern crate alloc;
extern crate capnp;

use bento::bento_utils;
use bento::fuse;
use bento::libc;
use bento::println;
use bento::std;
use bento::time;

pub mod hello_ll;

use hello_ll::HelloFS;
use bento_utils::BentoFilesystem;

pub mod hello_capnp {
    include!(concat!(env!("OUT_DIR"), "/src/hello_capnp.rs"));
}

pub static mut HELLO_FS: HelloFS = HelloFS {
    socket: None,
};

#[no_mangle]
pub fn rust_main() {
    println!("Hello from Rust");
    unsafe {
        HELLO_FS.register();
    }
}

#[no_mangle]
pub fn rust_exit() {
    unsafe {
        HELLO_FS.unregister();
    }
}
