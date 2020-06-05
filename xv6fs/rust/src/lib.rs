#![feature(lang_items)]
#![feature(concat_idents)]
#![feature(const_fn)]
#![feature(allocator_api)]
#![feature(alloc_error_handler)]
#![feature(alloc_layout_extra)]
#![feature(panic_info_message)]
#![no_std]

extern crate bento;

use bento::bentofs::*;
use bento::c_str;
use bento::println;

extern crate arr_macro;
extern crate rlibc;

pub mod log;
pub mod xv6fs_file;
pub mod xv6fs_fs;
pub mod xv6fs_ll;
pub mod xv6fs_utils;

//use xv6fs_ll::XV6FS_LL_OPS;
use xv6fs_ll::Xv6FileSystem;
use xv6fs_ll::XV6FS;

pub static XV6FS_OPS: fs_ops<Xv6FileSystem> = get_fs_ops(&XV6FS);

pub static FS_NAME: &'static str = c_str!("xv6fs_ll");

#[no_mangle]
pub fn rust_main() {
    println!("Hello from Rust");
    //register_bento_fs_rs(FS_NAME, &XV6FS_LL_OPS);
    //register_bento_fs_rs(FS_NAME, &XV6FS_OPS);
    XV6FS.register(&XV6FS_OPS);
}

#[no_mangle]
pub fn rust_exit() {
    unregister_bento_fs_rs(FS_NAME);
}
