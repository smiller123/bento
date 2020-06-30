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
extern crate arr_macro;
extern crate bento;
extern crate datablock;
extern crate rlibc;

use bento::c_str;
use bento::fuse::*;
use bento::println;
use bento::std as std;

mod log;
mod xv6fs_file;
mod xv6fs_fs;
mod xv6fs_ll;
mod xv6fs_utils;

use xv6fs_ll::XV6FS;

pub static FS_NAME: &'static str = c_str!("xv6fs_ll");

#[no_mangle]
pub fn rust_main() {
    println!("Hello from Rust");
    XV6FS.register();
}

#[no_mangle]
pub fn rust_exit() {
    XV6FS.unregister();
}
