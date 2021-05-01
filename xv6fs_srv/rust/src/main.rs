#![feature(lang_items)]
#![feature(concat_idents)]
#![feature(const_fn)]
#![feature(get_mut_unchecked)]

extern crate alloc;
extern crate bento_utils;
extern crate datablock;
extern crate fuse;
extern crate capnp;
extern crate core;
extern crate libc;
extern crate time;

#[macro_use]
pub mod xv6fs_ll;
pub mod xv6fs_file;
pub mod xv6fs_fs;
pub mod xv6fs_htree;
pub mod xv6fs_log;
pub mod xv6fs_utils;


//pub mod xv6fs_ll;

use std::env;

use xv6fs_ll::xv6fs_srv_runner;

pub mod hello_capnp {
    include!(concat!(env!("OUT_DIR"), "/src/hello_capnp.rs"));
}

fn main() {
    env_logger::init();
    let disk_name = env::args_os().nth(1).unwrap();
    let srv_role = env::args_os().nth(2).unwrap();
    let is_primary : bool = match srv_role.to_str().unwrap().to_lowercase().as_ref() {
        "primary" => true,
        "backup" => false,
        _ => {
            println!("server role must be : 'primary' or 'backup'");
            return;
        }
    };
    xv6fs_srv_runner(disk_name.to_str().unwrap(), is_primary);
}
