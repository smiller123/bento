extern crate alloc;
#[macro_use]
extern crate bento_utils;
extern crate fuse;
extern crate capnp;
extern crate core;
extern crate time;

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
