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

use bento::kernel::fs::*;

pub mod hello_ll;

use hello_ll::HelloFS;
use hello_ll::DISK;

#[no_mangle]
pub fn rust_main() {
    println!("Hello from Rust");
    HelloFS.register();
    let mut mut_disk = DISK.write();
    *mut_disk = Some(Disk::new(c_str!("/dev/nvme0n1"), 4096));
}

#[no_mangle]
pub fn rust_exit() {
    HelloFS.unregister();
}
