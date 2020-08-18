#![feature(lang_items)]
#![feature(concat_idents)]
#![feature(const_fn)]
#![feature(get_mut_unchecked)]

extern crate alloc;
extern crate fuse;
extern crate core;
extern crate libc;
extern crate serde;
extern crate time;

#[macro_use]
pub mod bento_utils;

pub mod hello_ll;

use std::env;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::RwLock;
use time::Timespec;

use hello_ll::HelloFS;
use bento_utils::Disk;

use fuse::*;
use bento_utils::BentoFilesystem;
use bento_utils::FuseConnInfo;

impl_filesystem!(HelloFS);

fn main() {
    env_logger::init();
    let disk_name = env::args_os().nth(1).unwrap();
    let fsname_arg_str = format!("fsname={}", disk_name.to_str().unwrap());
    let fsname_arg = fsname_arg_str.as_str();
    let disk = Disk::new(disk_name.to_str().unwrap(), 4096);
    let fs = HelloFS {
        disk: Some(RwLock::new(disk)),
        diskname: Some(disk_name.to_str().unwrap().to_string()),
    };

    let mountpoint = env::args_os().nth(2).unwrap();
    let mut opts_arr = vec!["-o", fsname_arg];
    if let Some(arg) = env::args_os().nth(3) {
        if arg.to_str().unwrap() == "blkdev" {
            opts_arr.append(&mut vec!["-o", "blkdev"]);
        }
    }
    let options = opts_arr.iter().map(OsStr::new).collect::<Vec<&OsStr>>();

    fuse::mount(fs, &mountpoint, &options).unwrap();
}
