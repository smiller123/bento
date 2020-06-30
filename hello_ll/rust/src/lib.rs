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

use bento;
use bento::c_str;
use bento::fuse::*;
use bento::println;

pub mod hello_ll;

use hello_ll::HELLO_FS;

#[no_mangle]
pub fn rust_main() {
    println!("Hello from Rust");
    HELLO_FS.register();
}

#[no_mangle]
pub fn rust_exit() {
    HELLO_FS.unregister();
}
