#![feature(lang_items)]
#![feature(concat_idents)]
#![feature(const_fn)]
#![feature(get_mut_unchecked)]

extern crate alloc;
extern crate fuse;
extern crate capnp;
extern crate core;
extern crate libc;
extern crate time;

pub mod hello_ll;

use std::env;

use hello_ll::hello_srv_runner;

pub mod hello_capnp {
    include!(concat!(env!("OUT_DIR"), "/src/hello_capnp.rs"));
}

fn main() {
    env_logger::init();
    let disk_name = env::args_os().nth(1).unwrap();
    hello_srv_runner(disk_name.to_str().unwrap());
}
