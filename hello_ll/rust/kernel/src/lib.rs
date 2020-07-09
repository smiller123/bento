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

use bento::bento_utils;
use bento::fuse;
use bento::libc;
use bento::println;
use bento::std;
use bento::time;

pub mod hello_ll;

use hello_ll::HelloFS;
use bento_utils::BentoFilesystem;

pub static HELLO_FS: HelloFS = HelloFS {
    disk: None
};

#[no_mangle]
pub fn rust_main() {
    println!("Hello from Rust");
    HELLO_FS.register();
}

#[no_mangle]
pub fn rust_exit() {
    HELLO_FS.unregister();
}
