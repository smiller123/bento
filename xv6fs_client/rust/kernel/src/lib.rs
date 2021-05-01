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

use bento::bento_utils;
use bento::fuse;
use bento::libc;
use bento::println;
use bento::std;
use bento::time;

pub mod xv6fs_ll;

use bento_utils::BentoFilesystem;
use xv6fs_ll::Xv6FileSystem;

pub static FS_NAME: &'static str = "xv6fs_client\0";

pub mod hello_capnp {
    include!(concat!(env!("OUT_DIR"), "/src/hello_capnp.rs"));
}

pub static mut XV6FS: Xv6FileSystem = Xv6FileSystem {
  //  socket: None,
    //srv_addr: None,
};

#[no_mangle]
pub fn rust_main() {
    println!("Hello from Rust");
    unsafe {
        XV6FS.register();
    }
}

#[no_mangle]
pub fn rust_exit() {
    unsafe {
        XV6FS.unregister();
    }
}
