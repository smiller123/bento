#![feature(lang_items)]
#![feature(concat_idents)]
#![feature(const_fn)]
#![feature(get_mut_unchecked)]

extern crate alloc;
extern crate fuse;
extern crate capnp;
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
use time::Timespec;

use hello_ll::HelloFS;

use fuse::*;
use bento_utils::BentoFilesystem;
use bento_utils::FuseConnInfo;

impl_filesystem!(HelloFS);

pub mod hello_capnp {
    include!(concat!(env!("OUT_DIR"), "/src/hello_capnp.rs"));
}

fn main() {
    env_logger::init();
    let fs = HelloFS {
        socket: None,
    };

    let mountpoint = env::args_os().nth(1).unwrap();
    let options: Vec::<&OsStr> = Vec::<&OsStr>::new();

    fuse::mount(fs, &mountpoint, &options).unwrap();
}
