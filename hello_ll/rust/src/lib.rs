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
use bento::bentofs::*;
use bento::c_str;
use bento::println;

pub mod hello_ll;

//use hello_ll::HELLO_LL_OPS;
use hello_ll::HelloFS;

pub static HELLO_OPS: fs_ops<HelloFS> = get_fs_ops(&HelloFS);

pub static FS_NAME: &'static str = c_str!("hello_ll");

#[no_mangle]
pub fn rust_main() {
    println!("Hello from Rust");
    //register_bento_fs_rs(FS_NAME, &HELLO_LL_OPS);
    //register_bento_fs_rs(FS_NAME, &HELLO_OPS);
    HelloFS.register(&HELLO_OPS);
}

#[no_mangle]
pub fn rust_exit() {
    unregister_bento_fs_rs(FS_NAME);
}
