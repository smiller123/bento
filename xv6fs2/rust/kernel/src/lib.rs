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
extern crate serde;

use bento::fuse;
use bento::libc;
use bento::println;
use bento::std;
use bento::time;
use bento::bento_utils;

mod xv6fs_log;
mod xv6fs_file;
mod xv6fs_fs;
mod xv6fs_ll;
mod xv6fs_utils;

use bento_utils::BentoFilesystem;
use xv6fs_ll::Xv6FileSystem;

pub static FS_NAME: &'static str = "xv6fs_ll\0";

pub static XV6FS: Xv6FileSystem = Xv6FileSystem {
    log: None,
    sb: None,
    disk: None,
    ilock_cache: None,
    icache_map: None,
    ialloc_lock: None,
    balloc_lock: None,
    diskname: None,
};


#[no_mangle]
pub fn rust_main() {
    println!("Hello from Rust");
    XV6FS.register();
}

#[no_mangle]
pub fn rust_exit() {
    XV6FS.unregister();
}
