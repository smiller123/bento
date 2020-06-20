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
extern crate bento;

use bento::c_str;
use bento::fuse::*;
use bento::println;
use bento::kernel::fs::*;

extern crate arr_macro;
extern crate rlibc;

pub mod log;
pub mod xv6fs_file;
pub mod xv6fs_fs;
pub mod xv6fs_ll;
pub mod xv6fs_utils;

use xv6fs_ll::XV6FS;
use xv6fs_fs::DISK;
use xv6fs_utils::BSIZE;

pub static FS_NAME: &'static str = c_str!("xv6fs_ll");

#[no_mangle]
pub fn rust_main() {
    println!("Hello from Rust");
    XV6FS.register();
    let mut mut_disk = DISK.write();
    *mut_disk = Some(Disk::new(c_str!("/dev/nvme0n1"), BSIZE as u32));
}

#[no_mangle]
pub fn rust_exit() {
    XV6FS.unregister();
}
