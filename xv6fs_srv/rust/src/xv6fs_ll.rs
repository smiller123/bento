/*
* SPDX-License-Identifier: GPL-2.0 OR MIT
*
* Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
     Anderson, Ang Chen, University of Washington
* Copyright (C) 2006-2018 Frans Kaashoek, Robert Morris, Russ Cox,
*                      Massachusetts Institute of Technology
*/

#[cfg(not(feature = "user"))]
use crate::bento_utils;
#[cfg(not(feature = "user"))]
use crate::fuse;
#[cfg(not(feature = "user"))]
use crate::libc;
#[cfg(not(feature = "user"))]
use crate::std;
#[cfg(not(feature = "user"))]
use crate::time;

use alloc::collections::btree_map::BTreeMap;

use alloc::string::String;
//use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;

use core::mem;
use core::str;


use datablock::DataBlock;

//use bento_utils::consts::*;
use bento_utils::*;
use fuse::consts::*;

use fuse::*;

#[cfg(not(feature = "user"))]
use bento::kernel::journal::*;
#[cfg(feature = "user")]
use crate::xv6fs_log::*;

use std::ffi::OsStr;
use std::sync::RwLock;
use std::thread;
use std::time::Duration;
use std::sync::Mutex;

use time::*;

use crate::xv6fs_file::*;
use crate::xv6fs_htree::*;
use crate::xv6fs_utils::*;

pub struct Xv6FileSystem {
    pub log: Option<Journal>,
    pub sb: Option<Xv6fsSB>,
    pub disk: Option<Arc<Disk>>,
    pub ilock_cache: Option<Vec<RwLock<Inode>>>,
    pub icache_map: Option<RwLock<BTreeMap<u64, usize>>>,
    pub ialloc_lock: Option<RwLock<usize>>,
    pub balloc_lock: Option<RwLock<usize>>,
    pub diskname: Option<String>,
}

// Xv6fs_srv impl starts here
use std::net::*;
use std::fs::File;
use std::io::{Read, Write};

use crate::hello_capnp::foo;
use capnp::serialize;

const PRIMARY_PORT: u16 = 1234;
const BACKUP_PORT: u16 = 8888; 

const VIEW_PORT: u16 = 1234;
const DEBUG: bool = false;
const HB_PORT: u16 = 8888;
//static mut hb_backup_stream: Option<TcpStream> = None;

fn send_rcv_from_backup(backup_stream: &Option<TcpStream>, client_stream: &mut TcpStream,msg_bytes: &[u8], resp_vec_len: u32) -> Result<(), ()> {
    // send op to backup
    let mut backup_stream_ref = backup_stream.as_ref().unwrap();
    backup_stream_ref.write(msg_bytes);

    // get result from backup
    let mut backup_resp = [0 as u8; 4096];
    let backup_resp_size = match backup_stream_ref.read(&mut backup_resp) {
        Ok(x) => x,
        Err(_) => {
            println!("statfs backup_stream read err");
            return Err(());
        }
    };
    let op_msg = str::from_utf8(&backup_resp[0..backup_resp_size]).unwrap();
    let op_vec: Vec<&str> = op_msg.split(' ').collect();
    match *op_vec.get(0).unwrap() {
        "Ok" => {
            return Ok(());
        },
        "Add" => {
            return Ok(());
        },
        "Err" => {
            println!("backup op - Err 1");
            let _ = client_stream.write(&backup_resp[0..backup_resp_size]);
            return Err(());
        },
        _ => {
            println!("backup op_msg: {}", op_msg);
            let _ = client_stream.write(&backup_resp[0..backup_resp_size]);
            return Err(());
        },

    };
}

fn connect_to_client(port: u16) -> Result<TcpStream, ()> {

    let srv_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);

    let listener = match TcpListener::bind(SocketAddr::V4(srv_addr)) {
        Ok(x) => x,
        Err(_) => {
            println!("connect_to_client - binding error");
            return Err(());
        },
    };

    match listener.accept() {
        Ok((stream, _)) => return Ok(stream),
        Err(_) => {

            println!("connect_to_client - listener accept error");
            return Err(());
        }
    };
}

pub fn xv6fs_srv_runner(devname: &str) {
    // initialize xv6fs
    println!("initializing xv6fs..");
    let mut XV6FS = Xv6FileSystem {
        log: None,
        sb: None,
        disk: None,
        ilock_cache: None,
        icache_map: None,
        ialloc_lock: None,
        balloc_lock: None,
        diskname: None,
    };
    XV6FS.xv6fs_init(devname);
    println!("OK");

    let view_srv_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, VIEW_PORT);

    println!("connecting to view server..");
    let mut stream = match TcpStream::connect(SocketAddr::V4(view_srv_addr)) {
        Ok(x) => x,
        Err(_) => {

            println!("..FAILED");
            return ;
        },
    };
    println!("..OK");

    if DEBUG {
        let msg = format!("hello from a server");
        stream.write(msg.as_bytes());
    }

    println!("connecting to hb port..");
    let hb_srv_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, HB_PORT);
    thread::spawn(move || {
        let hb_backup_stream = match TcpStream::connect(SocketAddr::V4(hb_srv_addr)) {
            Ok(x) => Some(x),
            Err(_) => {
                println!("..FAILED");
                return;
            }
        };
        
        let hb_msg = "tick";
        loop {
            hb_backup_stream.as_ref().unwrap().write(hb_msg.as_bytes());
            thread::sleep(Duration::from_micros(500));
        }
    }); 
    println!("..OK");

    println!("ready for ops..");
    loop {}
    loop {
    let mut buf = [0; 4096];
       //connection = match 
        let size = match stream.read(&mut buf) {
            Ok(x) if x == 0 => {
                break;
            },
            Ok(x) => {
                x
            },
            Err(_) => {
                let _ = stream.shutdown(Shutdown::Both);
                break;
            },
        };
        let buf_str = str::from_utf8(&buf[0..size]).unwrap();
        let buf_vec: Vec<&str> = buf_str.split(' ').collect();
        let buf_op = buf_vec.get(0).unwrap();
        match *buf_op {
            "exit" => break,
            "statfs" => {
                let statfs_res = XV6FS.statfs();
                match statfs_res {
                    Ok((a, b, c, d, e, f, g, h)) => {
                        let msg = format!("Ok {} {} {} {} {} {} {} {}",
                                          a, b, c, d, e, f, g, h);
                        let _ = stream.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = stream.write(msg.as_bytes());
                    },
                }
            },
            "open" => {
                if buf_vec.len() < 3 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = stream.write(msg.as_bytes());
                    continue;
                }

                let open_fh: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let open_flags: u32 = buf_vec.get(2).unwrap().parse().unwrap();
                let open_res = XV6FS.open(open_fh, open_flags);
                match open_res {
                    Ok((a, b)) => {
                        let msg = format!("Ok {} {}", a, b);
                        let _ = stream.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = stream.write(msg.as_bytes());
                    },
                }
            },
            "opendir" => {
                if buf_vec.len() < 2 {
                    //println!("server - opendir 1");
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = stream.write(msg.as_bytes());
                    continue;
                }
                let open_fh: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let open_res = XV6FS.opendir(open_fh);
                match open_res {
                    Ok((a, b)) => {
                        let msg = format!("Ok {} {}", a, b);
                        let _ = stream.write(msg.as_bytes());
                    },
                    Err(x) => {
                        //println!("server - opendir 2");
                        let msg = format!("Err {}", x);
                        let _ = stream.write(msg.as_bytes());
                    },
                }
            },
            "getattr" => {
                if buf_vec.len() < 2 {
                    // Send error back

                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = stream.write(msg.as_bytes());
                    continue;
                }

                let getattr_fh: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let getattr_res = XV6FS.getattr(getattr_fh);
                match getattr_res {
                    Ok((a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t)) => {

                        //println!("server - getattr OK");
                        let msg = format!("Ok {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
                                          a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t);
                        let _ = stream.write(msg.as_bytes());
                    },
                    Err(x) => {
                        //println!("server - getattr 2");
                        let msg = format!("Err {}", x);
                        let _ = stream.write(msg.as_bytes());
                    },
                }
            },
            "setattr" => { // TODO: change to match function
                if buf_vec.len() < 3 {
                    // Send error back

                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = stream.write(msg.as_bytes());
                    continue;
                }
                let setattr_fh: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                
                let setattr_size: Option<u64> = match buf_vec.get(2).unwrap().parse() {
                    Ok(size) => Some(size),
                    Err(_) => None,
                };
                let setattr_res = XV6FS.setattr(setattr_fh, setattr_size);
                match setattr_res {

                    Ok((a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t)) => {
                        let msg = format!("Ok {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
                                          a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t);
                        let _ = stream.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = stream.write(msg.as_bytes());
                    },
                }
            },
            "create" => {
                if buf_vec.len() < 3 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = stream.write(msg.as_bytes());
                    continue;
                }
                let create_parent: u64= buf_vec.get(1).unwrap().parse().unwrap();
                let create_name: &str= buf_vec.get(2).unwrap();
                let osstr_name = OsStr::new(create_name);
                let create_res = XV6FS.create(create_parent, &osstr_name);

                match create_res {
                    Ok((a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w)) => {
                        let msg = format!("Ok {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
                                          a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w);
                        let _ = stream.write(msg.as_bytes());
                    },
                    Err(x) => {

                        //println!("server - create ERR");
                        let msg = format!("Err {}", x);
                        let _ = stream.write(msg.as_bytes());
                    },
                }

            },
            "mkdir" => {
                if buf_vec.len() < 3 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = stream.write(msg.as_bytes());
                    continue;
                }
                let mkdir_parent: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let mkdir_name: &str = buf_vec.get(2).unwrap();
                let osstr_name = OsStr::new(mkdir_name);
                let mkdir_res = XV6FS.mkdir(mkdir_parent, osstr_name);
                match mkdir_res {
                    Ok((a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u)) => {
                        let msg = format!("Ok {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
                                          a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u);
                        let _ = stream.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = stream.write(msg.as_bytes());
                    },
                }
            },
            "lookup" => {
                if buf_vec.len() < 3 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = stream.write(msg.as_bytes());
                    continue;
                }

                let lookup_id: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let lookup_name: &str = buf_vec.get(2).unwrap();
                let osstr_name = OsStr::new(lookup_name);
                let lookup_res = XV6FS.lookup(lookup_id, &osstr_name);
                match lookup_res {
                    Ok((a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u)) => {
                        let msg = format!("Ok {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
                                          a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u);
                        let _ = stream.write(msg.as_bytes());
                    },
                    Err(x) => {
                        //println!("lookup err - err(x): {}", x);
                        let msg = format!("Err {}", x);
                        let _ = stream.write(msg.as_bytes());
                    },
                }
            },
            "read" => {
                if buf_vec.len() < 4 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = stream.write(msg.as_bytes());
                    continue;
                }

                let read_id: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let read_off: i64 = buf_vec.get(2).unwrap().parse().unwrap();
                let read_size: u32 = buf_vec.get(3).unwrap().parse().unwrap();
                let read_res = XV6FS.read(read_id, read_off, read_size);
                match read_res {
                    Ok(s) => {
                        let msg = format!("Ok {}", str::from_utf8(s.as_slice()).unwrap());
                        let _ = stream.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = stream.write(msg.as_bytes());
                    },
                }
            },
            "write" => {
                if buf_vec.len() < 3 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);

                    //println!("write - buf_vec.len() < 3");
                    let _ = stream.write(msg.as_bytes());
                    continue;
                }

                let write_id: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let write_off: i64 = buf_vec.get(2).unwrap().parse().unwrap();
                if buf_vec.len() == 3 {
                    let msg = "Ok 0";
                    let _ = stream.write(msg.as_bytes());
                }
                let write_data_off = buf_vec.get(0).unwrap().len() + buf_vec.get(1).unwrap().len() +
                    buf_vec.get(2).unwrap().len() + 3;
                let write_data = &buf[write_data_off..size];

                let write_res = XV6FS.write(write_id, write_off, write_data);
                match write_res {
                    Ok(a) => {
                        let msg = format!("Ok {}", a);
                        let _ = stream.write(msg.as_bytes());
                    },
                    Err(x) => {
                        //println!("write err(x): {}", x);
                        let msg = format!("Err {}", x);
                        let _ = stream.write(msg.as_bytes());
                    },
                }
            },
            "readdir" => {
                if buf_vec.len() < 3 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = stream.write(msg.as_bytes());
                    continue;
                }

                let readdir_id: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let readdir_off: i64 = buf_vec.get(2).unwrap().parse().unwrap();

                let readdir_res = XV6FS.readdir(readdir_id, readdir_off);
                let mut msg_vec: Vec<String> = Vec::new();
                match readdir_res {
                    Ok(s) => {
                        for (a, b, c, d) in s.iter() {
                            let msg = format!("Add {} {} {} {}", a, b, c, d);
                            msg_vec.push(msg);
                        }
                        let msg = format!("Ok");
                        msg_vec.push(msg);
                        let full_msg = msg_vec.join(" ");
                        let _ = stream.write(full_msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = stream.write(msg.as_bytes());
                    },
                }
            },
            "rmdir" => {
                if buf_vec.len() < 3 {
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = stream.write(msg.as_bytes());
                    continue;
                }

                let rmdir_parent: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let rmdir_name: &str = buf_vec.get(2).unwrap();
                let osstr_name = OsStr::new(rmdir_name);
                let rmdir_res = XV6FS.rmdir(rmdir_parent, &osstr_name);
                match rmdir_res {
                    Ok(()) => {
                        let msg = "Ok";
                        let _ = stream.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = stream.write(msg.as_bytes());
                    },
                }
 
            },
            "unlink" => {
                if buf_vec.len() < 3 {
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = stream.write(msg.as_bytes());
                    continue;
                }

                let unlink_parent: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let unlink_name: &str = buf_vec.get(2).unwrap();
                let osstr_name = OsStr::new(unlink_name);
                let unlink_res = XV6FS.unlink(unlink_parent, &osstr_name);
                match unlink_res {
                    Ok(()) => {
                        let msg = "Ok";
                        let _ = stream.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = stream.write(msg.as_bytes());
                    },
                }
            },
            "fsync" => {
                if buf_vec.len() < 2 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = stream.write(msg.as_bytes());
                    continue;
                }

                let fsync_res = XV6FS.fsync();
                match fsync_res {
                    Ok(()) => {
                        let msg = "Ok";
                        let _ = stream.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = stream.write(msg.as_bytes());
                    },
                }
            },
            "fsyncdir" => {
                if buf_vec.len() < 2 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = stream.write(msg.as_bytes());
                    continue;
                }

                let fsyncdir_res = XV6FS.fsyncdir();
                match fsyncdir_res {
                    Ok(()) => {
                        let msg = "Ok";
                        let _ = stream.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = stream.write(msg.as_bytes());
                    },
                }
            },
            "symlink" => {
                if buf_vec.len() < 3 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = stream.write(msg.as_bytes());
                    continue;
                }

                let symlink_nodeid: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let symlink_name: &str = buf_vec.get(2).unwrap();
                let symlink_linkname_str = buf_vec.get(3).unwrap();
                let osstr_name = OsStr::new(symlink_name);
                let symlink_res = XV6FS.symlink(symlink_nodeid, &osstr_name, symlink_linkname_str);
                match symlink_res {
                    Ok((a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u)) => {
                        let msg = format!("Ok {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
                                          a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u);
                        let _ = stream.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = stream.write(msg.as_bytes());
                    },
                }
            },
            "readlink" => {
                if buf_vec.len() < 2 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = stream.write(msg.as_bytes());
                    continue;
                }

                let readlink_nodeid: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let readlink_res = XV6FS.readlink(readlink_nodeid);
                match readlink_res {
                    Ok(s) => {
                        let msg = format!("Ok {}", str::from_utf8(s.as_slice()).unwrap());
                        let _ = stream.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = stream.write(msg.as_bytes());
                    },
                }
            },
            "rename" => {
                if buf_vec.len() < 6 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = stream.write(msg.as_bytes());
                    continue;
                }

                let rename_parent_ino: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let rename_name: &str = buf_vec.get(2).unwrap();
                let rename_newparent_ino: u64 = buf_vec.get(3).unwrap().parse().unwrap();
                let rename_newname: &str= buf_vec.get(4).unwrap();
                let rename_flags: u32 = buf_vec.get(5).unwrap().parse().unwrap();
                let osstr_name = OsStr::new(rename_name);
                let osstr_newname = OsStr::new(rename_newname);

                let rename_res = XV6FS.rename(rename_parent_ino, &osstr_name, rename_newparent_ino, osstr_newname, rename_flags);
                match rename_res {
                    Ok(()) => {
                        let msg = "Ok";
                        let _ = stream.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = stream.write(msg.as_bytes());
                    },
                }
            },
            s => println!("got buf {}", s),
        }
    }
    println!("shutting down..");
    let _ = stream.shutdown(Shutdown::Both);
}

/*pub fn xv6fs_srv_runner(devname: &str, is_primary: bool) {
    // initialize xv6fs
    let mut XV6FS = Xv6FileSystem {
        log: None,
        sb: None,
        disk: None,
        ilock_cache: None,
        icache_map: None,
        ialloc_lock: None,
        balloc_lock: None,
        diskname: None,
    };
    XV6FS.xv6fs_init(devname);

    println!("xv6fs_srv init - ok");
    println!("setting sockaddr");
    let srv_addr = match is_primary {
        true => SocketAddrV4::new(Ipv4Addr::LOCALHOST, PRIMARY_PORT),
        false => SocketAddrV4::new(Ipv4Addr::LOCALHOST, BACKUP_PORT),
    };
    let mut primary_alive: bool = true;
    let missed_hb = Arc::new(Mutex::new(0));

    // if is primary, connect to back up first
    let mut backup_stream: Option<TcpStream> = None;
    if is_primary {
        // connect to backup
        let backup_srv_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, BACKUP_PORT);
        backup_stream = match TcpStream::connect(SocketAddr::V4(backup_srv_addr)) {
            Ok(x) => Some(x),
            Err(_) => {
                println!("failed to connect do backup");
                return;
            }
        };

        let mut backup_msg = capnp::message::Builder::new_default();
        let mut backup_foo_msg = backup_msg.init_root::<foo::Builder>();
        backup_foo_msg.set_msg("hello from xv6fs_primary");
        serialize::write_message(&mut backup_stream.as_ref().unwrap(), &backup_msg);
        println!("primary connected to backup");

        // spawn a thread, connect to backup and send heart beats
        // TODO: add a listener to client for heart beats
        thread::spawn(move || {
            let hb_backup_stream = match TcpStream::connect(SocketAddr::V4(backup_srv_addr)) {
                Ok(x) => Some(x),
                Err(_) => {
                    println!("PRIMARY - Failed to setup hb tcpstream");
                    return;
                }
            };
           
            let hb_msg = "tick";
            loop {
                hb_backup_stream.as_ref().unwrap().write(hb_msg.as_bytes());
                thread::sleep(Duration::from_micros(500));
            }
        });
        // connect to backup stream for heartbeats
    }
    // TODO: REMOVE IF CONDITION
    println!("binding");
    let listener = match TcpListener::bind(SocketAddr::V4(srv_addr)) {
        Ok(x) => x,
        Err(_) => {
            return;
        },
    };

    println!("waiting for connection");
    let mut main_connection = match listener.accept() {
        Ok((stream, _)) => stream,
        Err(_) => {

            println!("listener accept error");
            return;
        }
    };


    println!("accepted connection");
    let message_reader = serialize::read_message(&mut main_connection, capnp::message::ReaderOptions::new()).unwrap();
    let foo_msg = message_reader.get_root::<foo::Reader>().unwrap();
    let text = foo_msg.get_msg().unwrap();
    println!("got text {}", text);

    // backup accept hb connection from primary
    let mut hb_primary_connection: Option<TcpStream> = None;
    if !is_primary {
        hb_primary_connection = match listener.accept() {
            Ok((stream, _)) => Some(stream),
            Err(_) => {
                println!("backup to primary accept error");
                return;
            }
        };
        let missed_hb_clone = missed_hb.clone();
        thread::spawn(move || {
            let backup_hb_connection = Some(hb_primary_connection.unwrap());
            let hb_missed_count = missed_hb_clone;
            println!("backup hb thread running..");
            loop {
                let mut hb_buf = [0; 4096];
                let hb_read_size = match backup_hb_connection.as_ref().unwrap().read(&mut hb_buf) {
                    Ok(x) if x == 0 => 0,
                    Ok(x) => {
                        //println!("connect.read {} bytes", x);
                    x
                    },
                    Err(_) => {
                        let _ = backup_hb_connection.unwrap().shutdown(Shutdown::Both);
                        println!("read from primary hb failed");
                        return;
                    },
                };
                if hb_read_size == 0 {
                    // increase missed heart beat
                    *hb_missed_count.lock().unwrap() += 1;

                } else {

                    *hb_missed_count.lock().unwrap() = 0;
                }
                thread::sleep(Duration::from_micros(1000));
            }

        });

    }

    let mut btc_stream: Option<TcpStream>;
    if !is_primary {
        btc_stream = match connect_to_client(5555) {
            Ok(stream) => Some(stream),
            Err(_) => {
                println!("failed to connect to client");
                return;
            }
        };
    }
    

    if is_primary {
        println!("primary main thread");
    } else {
        println!("backup main thread");
    }

    let mut connection = &main_connection;
    loop {
        if !is_primary & primary_alive {
            let count_missed = *missed_hb.lock().unwrap();
            if count_missed > 5 {
                    println!("primary died");
                primary_alive = false;
                connection = &btc_stream.as_ref().unwrap();
            }
        }
        if primary_alive {
            thread::sleep(Duration::from_millis(5));
        }

        let mut buf = [0; 4096];
       //connection = match 
        let size = match connection.read(&mut buf) {
            Ok(x) if x == 0 => {
                if !is_primary {
                    println!("backup read 0 from primary");
                    println!("primary crashed..");

                    // terminate tcp stream with primary

                    let _ = connection.shutdown(Shutdown::Both);
                    if primary_alive {
                        connection =  &btc_stream.as_ref().unwrap();
                        primary_alive = false;
                        continue;
                    }
                }
                break;
            },
            Ok(x) => {
                //println!("connect.read {} bytes", x);
                x
            },
            Err(_) => {
                let _ = connection.shutdown(Shutdown::Both);
                if is_primary {
                    return;
                } else { // this is backup
                    println!("primary crashed..");
                    
                    let _ = connection.shutdown(Shutdown::Both);
                    // terminate tcp stream with primary
                    if primary_alive {
                        connection = &btc_stream.as_ref().unwrap();
                        primary_alive = false;
                        continue;
                    }

                    break;
                }
            },
        };
        let buf_str = str::from_utf8(&buf[0..size]).unwrap();
        //println!("buf: {}", buf_str);
        let buf_vec: Vec<&str> = buf_str.split(' ').collect();
        let buf_op = buf_vec.get(0).unwrap();
        match *buf_op {
            "exit" => break,
            "statfs" => {
                // if primary server, send op to backup and get ack
                if is_primary {
                    match send_rcv_from_backup(&backup_stream, &mut connection, &buf[0..size], 9) {
                        Ok(_)=> (),
                        Err(_) => {
                            println!("statfs - send_rcv_from_backup Err");

                            continue;
                        },
                    };
                }

                let statfs_res = XV6FS.statfs();
                match statfs_res {
                    Ok((a, b, c, d, e, f, g, h)) => {
                        let msg = format!("Ok {} {} {} {} {} {} {} {}",
                                          a, b, c, d, e, f, g, h);
                        let _ = connection.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = connection.write(msg.as_bytes());
                    },
                }
            },
            "open" => {
                if buf_vec.len() < 3 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
                // if primary server, send op to backup and get ack
                if is_primary {
                    match send_rcv_from_backup(&backup_stream, &mut connection, &buf[0..size], 3) {
                        Ok(_)=> (),
                        Err(_) => {
                            println!("open - send_rcv_from_backup Err");
                            continue;
                        },
                    };
                }

                let open_fh: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let open_flags: u32 = buf_vec.get(2).unwrap().parse().unwrap();
                let open_res = XV6FS.open(open_fh, open_flags);
                match open_res {
                    Ok((a, b)) => {
                        let msg = format!("Ok {} {}", a, b);
                        let _ = connection.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = connection.write(msg.as_bytes());
                    },
                }
            },
            "opendir" => {
                if buf_vec.len() < 2 {
                    //println!("server - opendir 1");
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
                // if primary server, send op to backup and get ack
                if is_primary {
                    match send_rcv_from_backup(&backup_stream, &mut connection, &buf[0..size], 3) {
                        Ok(_)=> (),
                        Err(_) => {
                            println!("opendir - send_rcv_from_backup Err");
                            continue;
                        },
                    };
                }

                //println!("server - opendir");
                let open_fh: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let open_res = XV6FS.opendir(open_fh);
                match open_res {
                    Ok((a, b)) => {
                        //println!("server - opendir OK");
                        let msg = format!("Ok {} {}", a, b);
                        let _ = connection.write(msg.as_bytes());
                    },
                    Err(x) => {
                        //println!("server - opendir 2");
                        let msg = format!("Err {}", x);
                        let _ = connection.write(msg.as_bytes());
                    },
                }
            },
            "getattr" => {
                if buf_vec.len() < 2 {
                    // Send error back

                    //println!("server - getattr 1");
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
                // if primary server, send op to backup and get ack
                if is_primary {
                    match send_rcv_from_backup(&backup_stream, &mut connection, &buf[0..size], 21) {
                        Ok(_)=> (),
                        Err(_) => {
                            println!("getattr - send_rcv_from_backup Err");
                            continue;
                        },
                    };
                }

                //println!("server - getattr");
                let getattr_fh: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let getattr_res = XV6FS.getattr(getattr_fh);
                match getattr_res {
                    Ok((a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t)) => {

                        //println!("server - getattr OK");
                        let msg = format!("Ok {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
                                          a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t);
                        let _ = connection.write(msg.as_bytes());
                    },
                    Err(x) => {
                        //println!("server - getattr 2");
                        let msg = format!("Err {}", x);
                        let _ = connection.write(msg.as_bytes());
                    },
                }
            },
            "setattr" => { // TODO: change to match function
                if buf_vec.len() < 3 {
                    // Send error back

                    //println!("server - settattr 1");
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
                // if primary server, send op to backup and get ack
                if is_primary {
                    match send_rcv_from_backup(&backup_stream, &mut connection, &buf[0..size], 21) {
                        Ok(_)=> (),
                        Err(_) => {
                            println!("setattr - send_rcv_from_backup Err");
                            continue;
                        },
                    };
                }

                //println!("server - settattr ");
                let setattr_fh: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                
                let setattr_size: Option<u64> = match buf_vec.get(2).unwrap().parse() {
                    Ok(size) => Some(size),
                    Err(_) => None,
                };
                let setattr_res = XV6FS.setattr(setattr_fh, setattr_size);
                match setattr_res {

                    Ok((a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t)) => {
                        //println!("server - settattr OK");
                        let msg = format!("Ok {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
                                          a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t);
                        let _ = connection.write(msg.as_bytes());
                    },
                    Err(x) => {

                        //println!("server - settattr 2");
                        let msg = format!("Err {}", x);
                        let _ = connection.write(msg.as_bytes());
                    },
                }
            },
            "create" => {
                if buf_vec.len() < 3 {
                    //println!("server - create ERR");
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
                // if primary server, send op to backup and get ack
                if is_primary {
                    match send_rcv_from_backup(&backup_stream, &mut connection,&buf[0..size], 24) {
                        Ok(_)=> (),
                        Err(_) => {
                            println!("create - send_rcv_from_backup Err");
                            continue;
                        },
                    };
                }

                //println!("server - create");
                let create_parent: u64= buf_vec.get(1).unwrap().parse().unwrap();
                let create_name: &str= buf_vec.get(2).unwrap();
                let osstr_name = OsStr::new(create_name);
                let create_res = XV6FS.create(create_parent, &osstr_name);

                match create_res {
                    Ok((a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w)) => {

                        //println!("server - create OK");
                        let msg = format!("Ok {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
                                          a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w);
                        let _ = connection.write(msg.as_bytes());
                    },
                    Err(x) => {

                        //println!("server - create ERR");
                        let msg = format!("Err {}", x);
                        let _ = connection.write(msg.as_bytes());
                    },
                }

            },
            "mkdir" => {
                if buf_vec.len() < 3 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
                // if primary server, send op to backup and get ack
                if is_primary {
                    match send_rcv_from_backup(&backup_stream, &mut connection, &buf[0..size], 22) {
                        Ok(_)=> (),
                        Err(_) => {
                            println!("mkdir - send_rcv_from_backup Err");
                            continue;
                        },
                    };
                }

                let mkdir_parent: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let mkdir_name: &str = buf_vec.get(2).unwrap();
                let osstr_name = OsStr::new(mkdir_name);
                let mkdir_res = XV6FS.mkdir(mkdir_parent, osstr_name);
                match mkdir_res {
                    Ok((a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u)) => {
                        let msg = format!("Ok {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
                                          a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u);
                        let _ = connection.write(msg.as_bytes());
                    },
                    Err(x) => {
                        //println!("mkdir failed err: {} - parent_inode: {}, dir_name: {} ", x, mkdir_parent, mkdir_name);
                        let msg = format!("Err {}", x);
                        let _ = connection.write(msg.as_bytes());
                    },
                }
            },
            "lookup" => {
                if buf_vec.len() < 3 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
                // if primary server, send op to backup and get ack
                if is_primary {
                    match send_rcv_from_backup(&backup_stream, &mut connection, &buf[0..size], 22) {
                        Ok(_)=> (),
                        Err(_) => {
                            println!("lookup - send_rcv_from_backup Err");
                            continue;
                        },
                    };
                }

                let lookup_id: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let lookup_name: &str = buf_vec.get(2).unwrap();
                let osstr_name = OsStr::new(lookup_name);
                let lookup_res = XV6FS.lookup(lookup_id, &osstr_name);
                match lookup_res {
                    Ok((a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u)) => {
                        let msg = format!("Ok {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
                                          a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u);
                        let _ = connection.write(msg.as_bytes());
                    },
                    Err(x) => {
                        //println!("lookup err - err(x): {}", x);
                        let msg = format!("Err {}", x);
                        let _ = connection.write(msg.as_bytes());
                    },
                }
            },
            "read" => {
                if buf_vec.len() < 4 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
//                // if primary server, send op to backup and get ack
                //if is_primary {
                    //match send_rcv_from_backup(&backup_stream, &buf[0..size], 9) {
                        //Ok(_)=> (),
                        //Err(_) => {
                            //println!("read - send_rcv_from_backup Err");
                            //return;
                        //},
                    //};
                //}

                let read_id: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let read_off: i64 = buf_vec.get(2).unwrap().parse().unwrap();
                let read_size: u32 = buf_vec.get(3).unwrap().parse().unwrap();
                let read_res = XV6FS.read(read_id, read_off, read_size);
                match read_res {
                    Ok(s) => {
                        let msg = format!("Ok {}", str::from_utf8(s.as_slice()).unwrap());
                        let _ = connection.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = connection.write(msg.as_bytes());
                    },
                }
            },
            "write" => {
                if buf_vec.len() < 3 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);

                    //println!("write - buf_vec.len() < 3");
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
                // if primary server, send op to backup and get ack
                if is_primary {
                    match send_rcv_from_backup(&backup_stream, &mut connection, &buf[0..size], 2) {
                        Ok(_)=> (),
                        Err(_) => {
                            println!("write - send_rcv_from_backup Err");
                            continue;
                        },
                    };
                }

                let write_id: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let write_off: i64 = buf_vec.get(2).unwrap().parse().unwrap();
                if buf_vec.len() == 3 {
                    let msg = "Ok 0";
                    let _ = connection.write(msg.as_bytes());
                }
                let write_data_off = buf_vec.get(0).unwrap().len() + buf_vec.get(1).unwrap().len() +
                    buf_vec.get(2).unwrap().len() + 3;
                let write_data = &buf[write_data_off..size];

                let write_res = XV6FS.write(write_id, write_off, write_data);
                match write_res {
                    Ok(a) => {
                        let msg = format!("Ok {}", a);
                        let _ = connection.write(msg.as_bytes());
                    },
                    Err(x) => {
                        //println!("write err(x): {}", x);
                        let msg = format!("Err {}", x);
                        let _ = connection.write(msg.as_bytes());
                    },
                }
            },
            "readdir" => {
                if buf_vec.len() < 3 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
                // if primary server, send op to backup and get ack
                if is_primary {
                    match send_rcv_from_backup(&backup_stream, &mut connection, &buf[0..size], 5) {
                        Ok(_)=> (),
                        Err(_) => {
                            println!("readdir - send_rcv_from_backup Err");
                            continue;
                        },
                    };
                }

                let readdir_id: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let readdir_off: i64 = buf_vec.get(2).unwrap().parse().unwrap();

                let readdir_res = XV6FS.readdir(readdir_id, readdir_off);
                let mut msg_vec: Vec<String> = Vec::new();
                match readdir_res {
                    Ok(s) => {
                        for (a, b, c, d) in s.iter() {
                            let msg = format!("Add {} {} {} {}", a, b, c, d);
                            msg_vec.push(msg);
                        }
                        let msg = format!("Ok");
                        msg_vec.push(msg);
                        let full_msg = msg_vec.join(" ");
                        let _ = connection.write(full_msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = connection.write(msg.as_bytes());
                    },
                }
            },
            "rmdir" => {
                if buf_vec.len() < 3 {
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
                // if primary server, send op to backup and get ack
                if is_primary {
                    match send_rcv_from_backup(&backup_stream, &mut connection, &buf[0..size], 1) {
                        Ok(_)=> (),
                        Err(_) => {
                            println!("rmdir - send_rcv_from_backup Err");
                            continue;
                        },
                    };
                }

                let rmdir_parent: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let rmdir_name: &str = buf_vec.get(2).unwrap();
                let osstr_name = OsStr::new(rmdir_name);
                let rmdir_res = XV6FS.rmdir(rmdir_parent, &osstr_name);
                match rmdir_res {
                    Ok(()) => {
                        let msg = "Ok";
                        let _ = connection.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = connection.write(msg.as_bytes());
                    },
                }
 
            },
            "unlink" => {
                if buf_vec.len() < 3 {
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
                // if primary server, send op to backup and get ack
                if is_primary {
                    match send_rcv_from_backup(&backup_stream, &mut connection, &buf[0..size], 1) {
                        Ok(_)=> (),
                        Err(_) => {
                            println!("unlink - send_rcv_from_backup Err");
                            continue;
                        },
                    };
                }

                let unlink_parent: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let unlink_name: &str = buf_vec.get(2).unwrap();
                let osstr_name = OsStr::new(unlink_name);
                let unlink_res = XV6FS.unlink(unlink_parent, &osstr_name);
                match unlink_res {
                    Ok(()) => {
                        let msg = "Ok";
                        let _ = connection.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = connection.write(msg.as_bytes());
                    },
                }
            },
            "fsync" => {
                if buf_vec.len() < 2 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
                // if primary server, send op to backup and get ack
                if is_primary {
                    match send_rcv_from_backup(&backup_stream, &mut connection, &buf[0..size], 1) {
                        Ok(_)=> (),
                        Err(_) => {
                            println!("fsync - send_rcv_from_backup Err");
                            continue;
                        },
                    };
                }

                let fsync_res = XV6FS.fsync();
                match fsync_res {
                    Ok(()) => {
                        let msg = "Ok";
                        let _ = connection.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = connection.write(msg.as_bytes());
                    },
                }
            },
            "fsyncdir" => {
                if buf_vec.len() < 2 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
                // if primary server, send op to backup and get ack
                if is_primary {
                    match send_rcv_from_backup(&backup_stream, &mut connection, &buf[0..size], 1) {
                        Ok(_)=> (),
                        Err(_) => {
                            println!("fsyncdir - send_rcv_from_backup Err");
                            continue;
                        },
                    };
                }

                let fsyncdir_res = XV6FS.fsyncdir();
                match fsyncdir_res {
                    Ok(()) => {
                        let msg = "Ok";
                        let _ = connection.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = connection.write(msg.as_bytes());
                    },
                }
            },
            "symlink" => {
                if buf_vec.len() < 3 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
                // if primary server, send op to backup and get ack
                if is_primary {
                    match send_rcv_from_backup(&backup_stream, &mut connection, &buf[0..size], 22) {
                        Ok(_)=> (),
                        Err(_) => {
                            println!("symlink - send_rcv_from_backup Err");
                            continue;
                        },
                    };
                }

                let symlink_nodeid: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let symlink_name: &str = buf_vec.get(2).unwrap();
                let symlink_linkname_str = buf_vec.get(3).unwrap();
                let osstr_name = OsStr::new(symlink_name);
                let symlink_res = XV6FS.symlink(symlink_nodeid, &osstr_name, symlink_linkname_str);
                match symlink_res {
                    Ok((a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u)) => {
                        let msg = format!("Ok {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
                                          a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u);
                        let _ = connection.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = connection.write(msg.as_bytes());
                    },
                }
            },
            "readlink" => {
                if buf_vec.len() < 2 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
//                // if primary server, send op to backup and get ack
                //if is_primary {
                    //match send_rcv_from_backup(&backup_stream, &buf[0..size], 1) {
                        //Ok(_)=> (),
                        //Err(_) => {
                            //println!("readlink - send_rcv_from_backup Err");
                            //return;
                        //},
                    //};
                //}

                let readlink_nodeid: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let readlink_res = XV6FS.readlink(readlink_nodeid);
                match readlink_res {
                    Ok(s) => {
                        let msg = format!("Ok {}", str::from_utf8(s.as_slice()).unwrap());
                        let _ = connection.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = connection.write(msg.as_bytes());
                    },
                }
            },
            "rename" => {
                if buf_vec.len() < 6 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
                // if primary server, send op to backup and get ack
                if is_primary {
                    match send_rcv_from_backup(&backup_stream, &mut connection, &buf[0..size], 1) {
                        Ok(_)=> (),
                        Err(_) => {
                            println!("rename - send_rcv_from_backup Err");
                            continue;
                        },
                    };
                }

                let rename_parent_ino: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let rename_name: &str = buf_vec.get(2).unwrap();
                let rename_newparent_ino: u64 = buf_vec.get(3).unwrap().parse().unwrap();
                let rename_newname: &str= buf_vec.get(4).unwrap();
                let rename_flags: u32 = buf_vec.get(5).unwrap().parse().unwrap();
                let osstr_name = OsStr::new(rename_name);
                let osstr_newname = OsStr::new(rename_newname);

                let rename_res = XV6FS.rename(rename_parent_ino, &osstr_name, rename_newparent_ino, osstr_newname, rename_flags);
                match rename_res {
                    Ok(()) => {
                        let msg = "Ok";
                        let _ = connection.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = connection.write(msg.as_bytes());
                    },
                }
            },
            s => println!("got buf {}", s),
        }
    }
    let _ = connection.shutdown(Shutdown::Both);
    
}
*/

impl Xv6FileSystem {

    fn xv6fs_init(&mut self, devname: &str) -> Result<(), i32> {
        if self.disk.is_none() {
            let disk = Disk::new(devname, BSIZE as u64);
            //let mut disk_string = devname_str.to_string();
            let mut disk_string = devname.to_string();
            disk_string.push('\0');
            self.diskname = Some(disk_string);
            self.disk = Some(Arc::new(disk));
        }

        let sb_lock = Xv6fsSB {
            size: 0,
            nblocks: 0,
            ninodes: 0,
            nlog: 0,
            logstart: 0,
            inodestart: 0,
            bmapstart: 0,
        };
        self.sb = Some(sb_lock);

        self.iinit();

        return Ok(());
    }

    fn statfs(&self) -> Result<(u64, u64, u64, u64, u64, u32, u32, u32), i32> {
        let sb_lock = self.sb.as_ref().unwrap();
        let fs_size = sb_lock.size;
        return Ok((fs_size as u64, 0, 0, 0, 0, BSIZE as u32, DIRSIZ as u32, 0))
    }

    fn open(&self, nodeid: u64, flags: u32) -> Result<(u64, u32), i32> {
        let log = self.log.as_ref().unwrap();
        let inode = match self.iget(nodeid) {
            Ok(x) => x,
            Err(x) => {
                return Err(x);
            }
        };

        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                return Err(x);
            }
        };
        let mut internals = match inode_guard.internals.write() {
            Ok(x) => x,
            Err(_) => {
                return Err(libc::EIO);
            }
        };

        // Check if inode is a file
        if internals.inode_type != T_FILE {
            return Err(libc::EISDIR);
        }

        if flags & libc::O_TRUNC as u32 != 0 {
            let handle = log.begin_op(2);
            internals.size = 0;
            if let Err(x) = self.iupdate(&internals, inode.inum, &handle) {
                return Err(x);
            }
        }

        let fh = 0;
        let open_flags = FOPEN_KEEP_CACHE;
        return Ok((fh, open_flags));
    }


    fn opendir(&self, nodeid: u64) -> Result<(u64, u32), i32> {
        let inode = match self.iget(nodeid) {
            Ok(x) => x,
            Err(x) => {
                return Err(x);
            }
        };

        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                return Err(x);
            }
        };
        let internals = match inode_guard.internals.write() {
            Ok(x) => x,
            Err(_) => {
                return Err(libc::EIO);
            }
        };

        if internals.inode_type != T_DIR {
            return Err(libc::ENOTDIR);
        } else {
            let fh = 0;
            let open_flags = 0;
            return Ok((fh, open_flags));
        }
    }

    fn getattr(&self, nodeid: u64) -> Result<
        (i64, i32, u64, u64, u64, i64, i32, i64, i32, i64, i32, i64, i32, u32, u16, u32, u32, u32, u32, u32), i32> {

        let inode = match self.iget(nodeid) {
            Ok(x) => x,
            Err(x) => {
                return Err(x);
            }
        };

        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                return Err(x);
            }
        };
        let internals = match inode_guard.internals.read() {
            Ok(x) => x,
            Err(_) => {
                return Err(libc::EIO);
            }
        };
        let attr_valid = Timespec::new(1, 999999999);
        match self.stati(nodeid, &internals) {
            Ok(attr) => {
                let kind = match attr.kind {
                    FileType::Directory => 1,
                    FileType::RegularFile => 2,
                    _ => 3,
                };
                return  Ok((
                    attr_valid.sec,
                    attr_valid.nsec,
                    attr.ino,
                    attr.size,
                    attr.blocks,
                    attr.atime.sec,
                    attr.atime.nsec,
                    attr.mtime.sec,
                    attr.mtime.nsec,
                    attr.ctime.sec,
                    attr.ctime.nsec,
                    attr.crtime.sec,
                    attr.crtime.nsec,
                    kind,
                    attr.perm,
                    attr.nlink,
                    attr.uid,
                    attr.gid,
                    attr.rdev,
                    attr.flags
                ));
            },
            Err(x) => {
                return Err(x);
            }
        };
    }

    fn setattr(
        &self,
        ino: u64,
        size: Option<u64>,
    ) -> Result<
        (i64, i32, u64, u64, u64, i64, i32, i64, i32, i64, i32, i64, i32, u32, u16, u32, u32, u32, u32, u32), i32> {
        //println!("server - setattr");
        let inode = match self.iget(ino) {
            Ok(x) => x,
            Err(x) => {
                return Err(x);
            }
        };

        //println!("server - setattr 1");
        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                return Err(x);
            }
        };
        //println!("server - setattr 2");
        let mut internals = match inode_guard.internals.write() {
            Ok(x) => x,
            Err(_) => {
                return Err(libc::EIO);
            }
        };
        //println!("server - setattr 3");


        if let Some(fsize) = size {
            let log = self.log.as_ref().unwrap();
            let handle = log.begin_op(2);
            internals.size = fsize;
            if let Err(x) = self.iupdate(&internals, inode.inum, &handle) {
                return Err(x);
            }
        }

        //println!("server - setattr 4");
        let attr_valid = Timespec::new(1, 999999999);
        match self.stati(ino, &internals) {
            Ok(attr) => {
                let kind = match attr.kind {
                    FileType::Directory => 1,
                    FileType::RegularFile => 2,
                    _ => 3,
                };

                //println!("server - setattr OK");
                return  Ok((
                    attr_valid.sec,
                    attr_valid.nsec,
                    attr.ino,
                    attr.size,
                    attr.blocks,
                    attr.atime.sec,
                    attr.atime.nsec,
                    attr.mtime.sec,
                    attr.mtime.nsec,
                    attr.ctime.sec,
                    attr.ctime.nsec,
                    attr.crtime.sec,
                    attr.crtime.nsec,
                    kind,
                    attr.perm,
                    attr.nlink,
                    attr.uid,
                    attr.gid,
                    attr.rdev,
                    attr.flags
                ));
            },
            Err(x) =>  {

                //println!("server - setattr 5");
                return Err(x);
            },
        }
    }

    fn create(
        &self,
        parent: u64,
        name: &OsStr,
    ) -> Result<
        (i64, i32, u64, u64, u64, i64, i32, i64, i32, i64, i32, i64, i32, u32, u16, u32, u32, u32, u32, u32, u64, u32, u32), i32>  {
        // Check if the file already exists
        let log = self.log.as_ref().unwrap();
        let handle = log.begin_op(16);
        let child = match self.create_internal(parent, T_FILE, name, &handle) {
            Ok(x) => x,
            Err(x) => {
                return Err(x);
            }
        };

        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(child.idx, &icache, child.inum) {
            Ok(x) => x,
            Err(x) => {
                return Err(x);
            }
        };
        let internals = match inode_guard.internals.read() {
            Ok(x) => x,
            Err(_) => {
                return Err(libc::EIO);
            }
        };

        let fh = 0;
        let open_flags = FOPEN_KEEP_CACHE;
        let nodeid = child.inum as u64;
        let generation = 0;
        let attr_valid = Timespec::new(1, 999999999);
        match self.stati(nodeid, &internals) {
            Ok(attr) => {
                let kind = match attr.kind {
                    FileType::Directory => 1,
                    FileType::RegularFile => 2,
                    _ => 3,
                };
                return Ok((
                    attr_valid.sec,
                    attr_valid.nsec,
                    attr.ino,
                    attr.size,
                    attr.blocks,
                    attr.atime.sec,
                    attr.atime.nsec,
                    attr.mtime.sec,
                    attr.mtime.nsec,
                    attr.ctime.sec,
                    attr.ctime.nsec,
                    attr.crtime.sec,
                    attr.crtime.nsec,
                    kind,
                    attr.perm,
                    attr.nlink,
                    attr.uid,
                    attr.gid,
                    attr.rdev,
                    attr.flags,
                    generation,
                    fh,
                    open_flags,
                ));
            }
            Err(x) => {
                return Err(x);
            }
        }
    }    

    fn mkdir(
        &self,
        parent: u64,
        name: &OsStr,
    ) -> Result<
        (i64, i32, u64, u64, u64, i64, i32, i64, i32, i64, i32, i64, i32, u32, u16, u32, u32, u32, u32, u32, u64), i32> {

        let log = self.log.as_ref().unwrap();
        let handle = log.begin_op(MAXOPBLOCKS as u32);
        let child = match self.create_internal(parent, T_DIR, &name, &handle) {
            Ok(x) => x,
            Err(x) => {
                //println!("fn mkdir - 1");
                return Err(x);
            }
        };

        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(child.idx, &icache, child.inum) {
            Ok(x) => x,
            Err(x) => {
                //println!("fn mkdir - 2");
                return Err(x);
            }
        };

        let internals = match inode_guard.internals.read() {
            Ok(x) => x,
            Err(_) => {
                //println!("fn mkdir - 3");
                return Err(libc::EIO);
            }
        };

        let out_nodeid = child.inum as u64;
        let generation = 0;
        let attr_valid = Timespec::new(1, 999999999);
        match self.stati(out_nodeid, &internals) {
            Ok(attr) => {
                let kind = match attr.kind {
                    FileType::Directory => 1,
                    FileType::RegularFile => 2,
                    _ => 3,
                };
                return Ok((
                    attr_valid.sec,
                    attr_valid.nsec,
                    attr.ino,
                    attr.size,
                    attr.blocks,
                    attr.atime.sec,
                    attr.atime.nsec,
                    attr.mtime.sec,
                    attr.mtime.nsec,
                    attr.ctime.sec,
                    attr.ctime.nsec,
                    attr.crtime.sec,
                    attr.crtime.nsec,
                    kind,
                    attr.perm,
                    attr.nlink,
                    attr.uid,
                    attr.gid,
                    attr.rdev,
                    attr.flags,
                    generation,
                ));
            }
            Err(x) => {
                //println!("fn mkdir - 4");
                return Err(x);
            }
        }
    }

    fn lookup(&self, nodeid: u64, name: &OsStr) -> Result<
        (i64, i32, u64, u64, u64, i64, i32, i64, i32, i64, i32, i64, i32, u32, u16, u32, u32, u32, u32, u32, u64), i32> {
        // Get inode number from nodeid
        let inode = match self.iget(nodeid) {
            Ok(x) => x,
            Err(x) => {
                //println!("lookup 1");
                return Err(x);
            }
        };
        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                //println!("lookup 2");
                return Err(x);
            }
        };
        let mut internals = match inode_guard.internals.write() {
            Ok(x) => x,
            Err(_) => {
                //println!("lookup 3");
                return Err(libc::EIO);
            }
        };
        let mut poff = 0;
        let child = match self.dirlookup(&mut internals, name, &mut poff) {
            Ok(x) => x,
            Err(x) => {
                //println!("lookup 4");
                return Err(x);
            }
        };

        let outarg_nodeid = child.inum as u64;
        let outarg_generation = 0;
        let attr_valid = Timespec::new(1, 999999999);

        let child_inode_guard = match self.ilock(child.idx, &icache, child.inum) {
            Ok(x) => x,
            Err(x) => {
                //println!("lookup 5");
                return Err(x);
            }
        };
        let child_internals = match child_inode_guard.internals.read() {
            Ok(x) => x,
            Err(_) => {
                //println!("lookup 6");
                return Err(libc::EIO);
            }
        };
        match self.stati(outarg_nodeid, &child_internals) {
            Ok(attr) => {
                let kind = match attr.kind {
                    FileType::Directory => 1,
                    FileType::RegularFile => 2,
                    _ => 3,
                };
                return Ok((
                    attr_valid.sec,
                    attr_valid.nsec,
                    attr.ino,
                    attr.size,
                    attr.blocks,
                    attr.atime.sec,
                    attr.atime.nsec,
                    attr.mtime.sec,
                    attr.mtime.nsec,
                    attr.ctime.sec,
                    attr.ctime.nsec,
                    attr.crtime.sec,
                    attr.crtime.nsec,
                    kind,
                    attr.perm,
                    attr.nlink,
                    attr.uid,
                    attr.gid,
                    attr.rdev,
                    attr.flags,
                    outarg_generation
                ));
            }
            Err(x) => {
                //println!("lookup 7");
                return Err(x);
            }
        };
    }


    fn read(
        &self,
        nodeid: u64,
        offset: i64,
        size: u32,
    ) -> Result<Vec<u8>, i32> {
        // Get inode number nodeid
        let inode = match self.iget(nodeid) {
            Ok(x) => x,
            Err(x) => {
                return Err(x);
            }
        };

        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                return Err(x);
            }
        };
        let mut internals = match inode_guard.internals.write() {
            Ok(x) => x,
            Err(_) => {
                return Err(libc::EIO);
            }
        };

        // Check if inode is a file
        if internals.inode_type != T_FILE {
            return Err(libc::EISDIR);
        }

        let off = offset as usize;
        let n = size as usize;

        let mut buf_vec: Vec<u8> = vec![0; n as usize];
        let buf_slice = buf_vec.as_mut_slice();

        let read_rs = match self.readi(buf_slice, off, n, &mut internals) {
            Ok(x) => x,
            Err(x) => {
                return Err(x);
            }
        };
        return Ok(buf_vec);
    }


    fn write(
        &self,
        nodeid: u64,
        offset: i64,
        data: &[u8],
    ) -> Result<u32, i32> {
        // Get the inode at nodeid
        let max = ((MAXOPBLOCKS - 1 - 1 - 2) / 2) * BSIZE;
        let mut i = 0;
        let n = data.len();
        let mut off = offset as usize;
        let mut file_off = 0;
        while i < n {
            let log = self.log.as_ref().unwrap();
            let handle = log.begin_op(MAXOPBLOCKS as u32);
            let inode = match self.iget(nodeid) {
                Ok(x) => x,
                Err(x) => {
                    //println!("write 1");
                    return Err(x);
                }
            };

            let icache = self.ilock_cache.as_ref().unwrap();
            let inode_guard = match self.ilock(inode.idx, &icache, inode.inum) {
                Ok(x) => x,
                Err(x) => {
                    //println!("write 2");
                    return Err(x);
                }
            };
            let mut internals = match inode_guard.internals.write() {
                Ok(x) => x,
                Err(_) => {
                    //println!("write 3");
                    return Err(libc::EIO);
                }
            };

            // Check if inode is a file
            if internals.inode_type != T_FILE {
                //println!("write 4");
                return Err(libc::EISDIR);
            }

            let mut n1 = n - i;
            if n1 > max {
                n1 = max;
            }
            let data_region = &data[file_off..];
            let r = match self.writei(data_region, off, n1, &mut internals, inode.inum, &handle) {
                Ok(x) => x,
                Err(x) => {
                    //println!("write 5");
                    return Err(x);
                }
            };

            off += r;
            file_off += r;
            i += r;
        }
        return Ok(n as u32);
    }

    #[allow(unused_mut)]
    fn readdir(
        &self,
        nodeid: u64,
        offset: i64,
    ) -> Result<Vec<(u64, i64, u64, String)>, i32> {
        // Get inode number nodeid
        let inode = match self.iget(nodeid) {
            Ok(x) => x,
            Err(x) => {
                return Err(x);
            }
        };

        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                return Err(x);
            }
        };
        let mut internals = match inode_guard.internals.write() {
            Ok(x) => x,
            Err(_) => {
                return Err(libc::EIO);
            }
        };

        // Check if inode is directory
        if internals.inode_type != T_DIR {
            return Err(libc::ENOTDIR);
        }
        
        let mut readdir_vec: Vec<(u64, i64, u64, String)> = Vec::new();
        let hroot_len = mem::size_of::<Htree_root>();
        let hindex_len = mem::size_of::<Htree_index>();
        let hentry_len = mem::size_of::<Htree_entry>();
        let de_len = mem::size_of::<Xv6fsDirent>();
        let mut hroot_vec: Vec<u8> = vec![0; hroot_len];
        let mut buf_off = 1;
        let mut inarg_offset = offset as usize;
        let hroot_slice = hroot_vec.as_mut_slice();

        // try reading directory root
        let mut root = Htree_root::new();
        match self.readi(hroot_slice, 0, hroot_len, &mut internals) {
            Ok(x) if x != hroot_len => {
                return Err(1);
            }
            Err(x) => {
                return Err(x);
            }
            _ => {}
        };
        if root.extract_from(hroot_slice).is_err() {
            return Err(libc::EIO);
        }

        let num_indeces = root.ind_entries;
        if num_indeces == 0 {
            return Ok(readdir_vec);
        }

        let mut hie_vec: Vec<u8> = vec![0; hentry_len];
        let hie_slice = hie_vec.as_mut_slice();

        // check the index pointers stored in the root node
        for off in (hroot_len..(num_indeces as usize * hentry_len) + hroot_len).step_by(hentry_len)
        {
            if off >= BSIZE {
                break;
            }
            let mut hie = Htree_entry::new();
            match self.readi(hie_slice, off as usize, hentry_len, &mut internals) {
                Ok(x) if x != hentry_len => {
                    return Err(1);
                }
                Err(x) => {
                    return Err(x);
                }
                _ => {}
            }
            if hie.extract_from(hie_slice).is_err() {
                return Err(libc::EIO);
            }

            // check the index block for entries
            let mut ind_arr_vec: Vec<u8> = vec![0; BSIZE];
            let ind_arr_slice = ind_arr_vec.as_mut_slice();
            match self.readi(
                ind_arr_slice,
                BSIZE * hie.block as usize,
                BSIZE,
                &mut internals,
            ) {
                Ok(x) if x != BSIZE => {
                    return Err(1);
                }

                Err(x) => {
                    return Err(x);
                }
                _ => {}
            }

            let ind_header_slice = &mut ind_arr_slice[0..hindex_len];
            let mut index = Htree_index::new();
            if index.extract_from(ind_header_slice).is_err() {
                return Err(libc::EIO);
            }

            let num_entries = index.entries;

            if num_entries == 0 {
                continue;
            }

            // check entries in index node
            for ine_idx in
                (hindex_len..hindex_len + (hentry_len * index.entries as usize)).step_by(hentry_len)
            {
                let ine_slice = &mut ind_arr_slice[ine_idx..ine_idx + hentry_len];
                let mut ine = Htree_entry::new();
                if ine.extract_from(ine_slice).is_err() {
                    return Err(libc::EIO);
                }
                let dblock_off = ine.block;
                if dblock_off == 0 {
                    continue;
                }
                let mut de_block_vec: Vec<u8> = vec![0; BSIZE];
                let de_block_slice = de_block_vec.as_mut_slice();

                match self.readi(
                    de_block_slice,
                    BSIZE * dblock_off as usize,
                    BSIZE,
                    &mut internals,
                ) {
                    Err(x) => {
                        return Err(x);
                    }
                    _ => {}
                }

                // check dirents in leaf node
                for de_off in (0..BSIZE).step_by(de_len) {
                    let de_slice = &mut de_block_slice[de_off..de_off + de_len];
                    let mut de = Xv6fsDirent::new();
                    if de.extract_from(de_slice).is_err() {
                        return Err(libc::EIO);
                    }

                    if de.inum == 0 {
                        continue;
                    }
                    if inarg_offset >= 1 {
                        inarg_offset -= 1;
                        buf_off += 1;
                        continue;
                    }

                    let i_type;
                    if de.inum as u64 == nodeid {
                        i_type = FileType::Directory;
                    } else {
                        let entry = match self.iget(de.inum as u64) {
                            Ok(x) => x,
                            Err(x) => {
                                return Err(x);
                            }
                        };

                        let entry_inode_guard = match self.ilock(entry.idx, &icache, de.inum) {
                            Ok(x) => x,
                            Err(x) => {
                                return Err(x);
                            }
                        };
                        let entry_internals = match entry_inode_guard.internals.read() {
                            Ok(x) => x,
                            Err(_) => {
                                return Err(libc::EIO);
                            }
                        };

                        i_type = match entry_internals.inode_type {
                            T_DIR => FileType::Directory,
                            T_LNK => FileType::Symlink,
                            _ => FileType::RegularFile,
                        };
                    }

                    let name_str = match str::from_utf8(&de.name) {
                        Ok(x) => x,
                        Err(_) => "",
                    };
                    if de.inum != 0 {
                        // TODO: Match all types
                        let inode_type: u64 = match i_type {
//                            FileType::NamedPipe => stat::S_IFIFO,
                            //FileType::CharDevice => stat::S_IFCHR,
                            //FileType::BlockDevice => stat::S_IFBLK,
                            //FileType::Directory => stat::S_IFDIR,
                            //FileType::RegularFile => stat::S_IFREG,
                            //FileType::Symlink => stat::S_IFLNK,
                            //FileType::Socket => stat::S_IFSOCK,
                            FileType::Directory => 1,
                            FileType::Symlink => 2,
                            FileType::RegularFile => 3,
                            _ => 8888, 
                        };
                        readdir_vec.push((de.inum as u64, buf_off, inode_type, name_str.to_string()));
                        // TODO: might not want to return here
                        return Ok(readdir_vec);
                    }
                    buf_off += 1;
                }
            }
        }
        return Ok(readdir_vec);
    }
    
    fn rmdir(&self, parent: u64, name: &OsStr) -> Result<(), i32> {
        let log = self.log.as_ref().unwrap();
        let handle = log.begin_op(MAXOPBLOCKS as u32);
        match self.dounlink(parent, name, &handle) {
            Ok(_) => return Ok(()),
            Err(x) => return Err(x),
        };
    }

    fn unlink(&self, parent: u64, name: &OsStr) -> Result<(), i32> {
        let log = self.log.as_ref().unwrap();
        let handle = log.begin_op(MAXOPBLOCKS as u32);
        match self.dounlink(parent, name, &handle) {
            Ok(_) => return Ok(()),
            Err(x) => return Err(x),
        };
    }

    fn fsync(&self) -> Result<(), i32> {
        let log = self.log.as_ref().unwrap();
        log.force_commit();
        return Ok(());
    }

    fn fsyncdir(&self) -> Result<(), i32> {
        let log = self.log.as_ref().unwrap();
        log.force_commit();
        return Ok(());
    }

    fn symlink(
        &self,
        nodeid: u64,
        name: &OsStr,
        linkname: &str,
    ) -> Result<
        (i64, i32, u64, u64, u64, i64, i32, i64, i32, i64, i32, i64, i32, u32, u16, u32, u32, u32, u32, u32, u64), i32> {
        let log = self.log.as_ref().unwrap();
        let handle = log.begin_op(MAXOPBLOCKS as u32);
        // Create new file
        let child = match self.create_internal(nodeid, T_LNK, name, &handle) {
            Ok(x) => x,
            Err(x) => {
                return Err(x);
            }
        };

        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(child.idx, &icache, child.inum) {
            Ok(x) => x,
            Err(x) => {
                return Err(x);
            }
        };
        let mut internals = match inode_guard.internals.write() {
            Ok(x) => x,
            Err(_) => {
                return Err(libc::EIO);
            }
        };

        let mut len_slice = [0; mem::size_of::<u32>()];
        let str_length: u32 = linkname.len() as u32 + 1;
        let strlen_slice = str_length.to_ne_bytes();
        len_slice.copy_from_slice(&strlen_slice);
        if let Err(x) = self.writei(
            &len_slice,
            0,
            mem::size_of::<u32>(),
            &mut internals,
            child.inum,
            &handle,
        ) {
            return Err(x);
        };

        if let Err(x) = self.writei(
            linkname.as_bytes(),
            mem::size_of::<u32>(),
            linkname.len(),
            &mut internals,
            child.inum,
            &handle,
        ) {
            return Err(x);
        };
        let out_nodeid = child.inum as u64;
        let generation = 0;
        let attr_valid = Timespec::new(1, 999999999);
        match self.stati(out_nodeid, &internals) {
            Ok(attr) => {
                let kind = match attr.kind {
                    FileType::Directory => 1,
                    FileType::RegularFile => 2,
                    _ => 3,
                };
                return Ok((
                    attr_valid.sec,
                    attr_valid.nsec,
                    attr.ino,
                    attr.size,
                    attr.blocks,
                    attr.atime.sec,
                    attr.atime.nsec,
                    attr.mtime.sec,
                    attr.mtime.nsec,
                    attr.ctime.sec,
                    attr.ctime.nsec,
                    attr.crtime.sec,
                    attr.crtime.nsec,
                    kind,
                    attr.perm,
                    attr.nlink,
                    attr.uid,
                    attr.gid,
                    attr.rdev,
                    attr.flags,
                    generation,
                ));
            } 
            Err(x) => {
                return Err(x);
            }
        }
    }

    fn readlink(&self, nodeid: u64) -> Result<Vec<u8>, i32>  {
        let inode = match self.iget(nodeid) {
            Ok(x) => x,
            Err(x) => {
                return Err(x);
            }
        };

        let icache = self.ilock_cache.as_ref().unwrap();
        let inode_guard = match self.ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                return Err(x);
            }
        };
        let mut internals = match inode_guard.internals.write() {
            Ok(x) => x,
            Err(_) => {
                return Err(libc::EIO);
            }
        };

        // Check if inode is a file
        if internals.inode_type != T_LNK {
            return Err(1);
        }

        let mut len_slice = [0; 4];

        match self.readi(&mut len_slice, 0, mem::size_of::<u32>(), &mut internals) {
            Ok(x) if x != mem::size_of::<u32>() => {
                return Err(libc::EIO);
            }
            Err(x) => {
                return Err(x);
            }
            _ => {}
        }
        let mut str_len_bytes = [0; 4];
        str_len_bytes.copy_from_slice(&len_slice);
        let str_len = u32::from_ne_bytes(str_len_bytes);

        let mut buf_vec: Vec<u8> = vec![0; str_len as usize];
        let buf_slice = buf_vec.as_mut_slice();

        match self.readi(
            buf_slice,
            mem::size_of::<u32>(),
            str_len as usize,
            &mut internals,
        ) {
            Ok(x) => x,
            Err(x) => {
                return Err(x);
            }
        };
        return Ok(buf_vec);
    }

    fn rename(
        &self,
        parent_ino: u64,
        name: &OsStr,
        newparent_ino: u64,
        newname: &OsStr,
        flags: u32,
    ) -> Result<(), i32> {
        let log = self.log.as_ref().unwrap();
        let handle = log.begin_op(MAXOPBLOCKS as u32);
        let no_replace = (flags & libc::RENAME_NOREPLACE as u32) > 0;
        let exchange = (flags & libc::RENAME_EXCHANGE as u32) > 0;
        // Get and lock old and new parent directories
        if parent_ino != newparent_ino {
            let old_parent = match self.iget(parent_ino) {
                Ok(x) => x,
                Err(_) => {
                    return Err(libc::EIO);
                },
            };
            let new_parent = match self.iget(newparent_ino) {
                Ok(x) => x,
                Err(_) => {
                    return Err(libc::EIO);
                },
            };
            let icache = self.ilock_cache.as_ref().unwrap();
            let old_parent_inode_guard = match self.ilock(old_parent.idx, &icache, old_parent.inum) {
                Ok(x) => x,
                Err(_) => {
                    return Err(libc::EIO);
                },
            };
            let new_parent_inode_guard = match self.ilock(new_parent.idx, &icache, new_parent.inum) {
                Ok(x) => x,
                Err(_) => {
                    return Err(libc::EIO);
                },
            };
            let mut old_parent_internals = match old_parent_inode_guard
                .internals
                .write() {
                Ok(x) => x,
                Err(_) => {
                    return Err(libc::EIO);
                },
            };
            let mut new_parent_internals = match new_parent_inode_guard
                .internals
                .write() {
                Ok(x) => x,
                Err(_) => {
                    return Err(libc::EIO);
                },
            };
            let mut old_poff = 0;
            let old_name_str = name.to_str().unwrap();
            if old_name_str == "." || old_name_str == ".." {
                return Err(libc::EIO);
            }
            let inode = match self.dirlookup(&mut old_parent_internals, name, &mut old_poff) {
                Ok(x) => x,
                Err(x) => {
                    return Err(x);
                },
            };

            let inode_guard = match self.ilock(inode.idx, &icache, inode.inum) {
                Ok(x) => x,
                Err(_) => {
                    return Err(libc::EIO);
                },
            };
            let mut inode_internals = match inode_guard.internals.write() {
                Ok(x) => x,
                Err(_) => {
                    return Err(libc::EIO);
                },
            };

            if inode_internals.nlink < 1 {
                return Err(libc::EIO);
            }

            let mut new_poff = 0;
            let new_name_str = newname.to_str().unwrap();
            if new_name_str == "." || new_name_str == ".." {
                return Err(libc::EIO);
            }
            let new_inode_res = self.dirlookup(&mut new_parent_internals, newname, &mut new_poff);
            if let Ok(new_inode) = new_inode_res {
                if no_replace {
                    return Err(libc::EEXIST);
                } else if exchange {
                    let de_arr = [0; mem::size_of::<Xv6fsDirent>()];
                    let buf_len = mem::size_of::<Xv6fsDirent>();
                    match self.writei(
                        &de_arr,
                        new_poff as usize,
                        buf_len,
                        &mut new_parent_internals,
                        new_parent.inum,
                        &handle
                    ) {
                        Ok(x) => x,
                        Err(_) => {
                            return Err(libc::EIO);
                        },
                    };
                    let new_inode_guard = match self.ilock(new_inode.idx, &icache, new_inode.inum) {
                        Ok(x) => x,
                        Err(_) => {
                            return Err(libc::EIO);
                        },
                    };
                    let mut new_inode_internals = match new_inode_guard.internals.write() {
                        Ok(x) => x,
                        Err(_) => {
                            return Err(libc::EIO);
                        },
                    };
                    if new_inode_internals.inode_type == T_DIR {
                        old_parent_internals.nlink += 1;
                        if self.iupdate(&old_parent_internals, old_parent.inum, &handle).is_err() {
                            return Err(libc::EIO);
                        }
                        let d = OsStr::new(".");
                        if self.dirlink(&mut new_inode_internals, &d, new_inode.inum, new_inode.inum, &handle).is_err() {
                            return Err(libc::EIO);
                        }
    
                        let dd = OsStr::new("..");
                        if self.dirlink(&mut new_inode_internals, &dd, parent_ino as u32, new_inode.inum, &handle).is_err() {
                            return Err(libc::EIO);
                        }
                    }
    
                    if self.dirlink(&mut old_parent_internals, name, new_inode.inum, old_parent.inum, &handle).is_err() {
                        return Err(libc::EIO);
                    }
                } else {
                    let new_inode_guard = match self.ilock(new_inode.idx, &icache, new_inode.inum) {
                        Ok(x) => x,
                        Err(_) => {
                            return Err(libc::EIO);
                        },
                    };
                    let mut new_inode_internals = match new_inode_guard.internals.write() {
                        Ok(x) => x,
                        Err(_) => {
                            return Err(libc::EIO);
                        },
                    };
                    if new_inode_internals.inode_type == T_DIR {
                        match self.isdirempty(&mut new_inode_internals) {
                            Ok(true) => {}
                            _ => {
                                return Err(libc::ENOTEMPTY);
                            }
                        }
                    }
                    let de_arr = [0; mem::size_of::<Xv6fsDirent>()];
                    let buf_len = mem::size_of::<Xv6fsDirent>();
                    match self.writei(
                        &de_arr,
                        new_poff as usize,
                        buf_len,
                        &mut new_parent_internals,
                        new_parent.inum,
                        &handle
                    ) {
                        Ok(x) => x,
                        Err(_) => {
                            return Err(libc::EIO);
                        },
                    };
                }
            }


            let de_arr = [0; mem::size_of::<Xv6fsDirent>()];
            let buf_len = mem::size_of::<Xv6fsDirent>();
            match self.writei(
                &de_arr,
                old_poff as usize,
                buf_len,
                &mut old_parent_internals,
                old_parent.inum,
                &handle
            ) {
                Ok(x) => x,
                Err(_) => {
                    return Err(libc::EIO);
                },
            };


            if inode_internals.inode_type == T_DIR {
                old_parent_internals.nlink -= 1;
                if self.iupdate(&old_parent_internals, old_parent.inum, &handle).is_err() {
                    return Err(libc::EIO);
                }
            }
            if inode_internals.inode_type == T_DIR {
                new_parent_internals.nlink += 1;
                if self.iupdate(&new_parent_internals, new_parent.inum, &handle).is_err() {
                    return Err(libc::EIO);
                }
                let d = OsStr::new(".");
                if self.dirlink(&mut inode_internals, &d, inode.inum, inode.inum, &handle).is_err() {
                    return Err(libc::EIO);
                }
    
                let dd = OsStr::new("..");
                if self.dirlink(&mut inode_internals, &dd, newparent_ino as u32, inode.inum, &handle).is_err() {
                    return Err(libc::EIO);
                }
            }
    
            if self.dirlink(&mut new_parent_internals, newname, inode.inum, new_parent.inum, &handle).is_err() {
                return Err(libc::EIO);
            }
        } else {
            let parent = match self.iget(parent_ino) {
                Ok(x) => x,
                Err(_) => {
                    return Err(libc::EIO);
                },
            };
            let icache = self.ilock_cache.as_ref().unwrap();
            let parent_inode_guard = match self.ilock(parent.idx, &icache, parent.inum) {
                Ok(x) => x,
                Err(_) => {
                    return Err(libc::EIO);
                },
            };
            let mut parent_internals = match parent_inode_guard
                .internals
                .write() {
                Ok(x) => x,
                Err(_) => {
                    return Err(libc::EIO);
                },
            };
            let mut old_poff = 0;
            let old_name_str = name.to_str().unwrap();
            if old_name_str == "." || old_name_str == ".." {
                return Err(libc::EIO);
            }
            let inode = match self.dirlookup(&mut parent_internals, name, &mut old_poff) {
                Ok(x) => x,
                Err(x) => {
                    return Err(x);
                },
            };

            let inode_guard = match self.ilock(inode.idx, &icache, inode.inum) {
                Ok(x) => x,
                Err(_) => {
                    return Err(libc::EIO);
                },
            };
            let inode_internals = match inode_guard.internals.write() {
                Ok(x) => x,
                Err(_) => {
                    return Err(libc::EIO);
                },
            };
            if inode_internals.nlink < 1 {
                return Err(libc::EIO);
            }

            let mut new_poff = 0;
            let new_name_str = newname.to_str().unwrap();
            if new_name_str == "." || new_name_str == ".." {
                return Err(libc::EIO);
            }
            let new_inode_res = self.dirlookup(&mut parent_internals, newname, &mut new_poff);
            if let Ok(new_inode) = new_inode_res {
                if no_replace {
                    return Err(libc::EEXIST);
                } else if exchange {
                    let de_arr = [0; mem::size_of::<Xv6fsDirent>()];
                    let buf_len = mem::size_of::<Xv6fsDirent>();
                    match self.writei(
                        &de_arr,
                        new_poff as usize,
                        buf_len,
                        &mut parent_internals,
                        parent.inum,
                        &handle
                    ) {
                        Ok(x) => x,
                        Err(_) => {
                            return Err(libc::EIO);
                        },
                    };
                    if self.dirlink(&mut parent_internals, name, new_inode.inum, parent.inum, &handle).is_err() {
                        return Err(libc::EIO);
                    }
                } else {
                    let new_inode_guard = match self.ilock(new_inode.idx, &icache, new_inode.inum) {
                        Ok(x) => x,
                        Err(_) => {
                            return Err(libc::EIO);
                        },
                    };
                    let mut new_inode_internals = match new_inode_guard.internals.write() {
                        Ok(x) => x,
                        Err(_) => {
                            return Err(libc::EIO);
                        },
                    };
                    if new_inode_internals.inode_type == T_DIR {
                        match self.isdirempty(&mut new_inode_internals) {
                            Ok(true) => {}
                            _ => {
                                return Err(libc::ENOTEMPTY);
                            }
                        }
                    }
                    let de_arr = [0; mem::size_of::<Xv6fsDirent>()];
                    let buf_len = mem::size_of::<Xv6fsDirent>();
                    match self.writei(
                        &de_arr,
                        new_poff as usize,
                        buf_len,
                        &mut parent_internals,
                        parent.inum,
                        &handle
                    ) {
                        Ok(x) => x,
                        Err(_) => {
                            return Err(libc::EIO);
                        },
                    };
                }
            }
            let de_arr = [0; mem::size_of::<Xv6fsDirent>()];
            let buf_len = mem::size_of::<Xv6fsDirent>();
            match self.writei(
                &de_arr,
                old_poff as usize,
                buf_len,
                &mut parent_internals,
                parent.inum,
                &handle
            ) {
                Ok(x) => x,
                Err(_) => {
                    return Err(libc::EIO);
                },
            };
            if self.dirlink(&mut parent_internals, newname, inode.inum, parent.inum, &handle).is_err() {
                return Err(libc::EIO);
            }
        }
        return Ok(());
    
    }
    // old part
    const NAME: &'static str = "xv6fs_ll\0";

    fn create_internal<'a>(
        &'a self,
        nodeid: u64,
        itype: u16,
        name: &OsStr,
        handle: &Handle
    ) -> Result<CachedInode<'a>, libc::c_int> {
        // Get inode for parent directory
        let parent = self.iget(nodeid)?;
        let icache = self.ilock_cache.as_ref().unwrap();
        // Get inode for new file
        let parent_inode_guard = self.ilock(parent.idx, &icache, parent.inum)?;
        let mut parent_internals = parent_inode_guard
            .internals
            .write()
            .map_err(|_| libc::EIO)?;

        let inode = self.ialloc(itype, handle)?;
        if (parent_internals.size as usize + mem::size_of::<Xv6fsDirent>())
            > (MAXFILE as usize * BSIZE)
        {
            return Err(libc::EIO);
        }

        let inode_guard = self.ilock(inode.idx, &icache, inode.inum)?;
        let mut internals = inode_guard.internals.write().map_err(|_| libc::EIO)?;

        internals.major = parent_internals.major;
        internals.minor = parent_internals.minor;
        internals.nlink = 1;
    
        self.iupdate(&internals, inode.inum, handle)?;
    
        if itype == T_DIR {
            parent_internals.nlink += 1;
            self.iupdate(&parent_internals, parent.inum, handle)?;
            let d = OsStr::new(".");
            self.dirlink(&mut internals, &d, inode.inum, inode.inum, handle)?;
    
            let dd = OsStr::new("..");
            self.dirlink(&mut internals, &dd, nodeid as u32, inode.inum, handle)?;
        }
    
        self.dirlink(&mut parent_internals, name, inode.inum, parent.inum, handle)?;
        return Ok(inode);
    }

    fn isdirempty(&self, internals: &mut InodeInternal) -> Result<bool, libc::c_int> {
        let hroot_len = mem::size_of::<Htree_root>();
        let hindex_len = mem::size_of::<Htree_index>();
        let hentry_len = mem::size_of::<Htree_entry>();
        let de_len = mem::size_of::<Xv6fsDirent>();
        let mut hroot_vec: Vec<u8> = vec![0; hroot_len];

        let hroot_slice = hroot_vec.as_mut_slice();

        // try reading directory root
        let mut root = Htree_root::new();
        match self.readi(hroot_slice, 0, hroot_len, internals) {
            Ok(x) if x != hroot_len => return Err(libc::EIO),
            Err(x) => {
                return Err(x);
            }
            _ => {}
        };

        root.extract_from(hroot_slice).map_err(|_| libc::EIO)?;

        let num_indeces = root.ind_entries;
        if num_indeces == 0 {
            return Ok(true);
        }

        // check the index pointers stored in the root
        for off in (hroot_len..(num_indeces as usize * hentry_len) + hroot_len).step_by(hentry_len)
        {
            if off >= BSIZE {
                break;
            }
            let mut rie = Htree_entry::new();
            let mut rie_vec: Vec<u8> = vec![0; hentry_len];
            let rie_slice = rie_vec.as_mut_slice();
            match self.readi(rie_slice, off as usize, hentry_len, internals) {
                Ok(x) if x != hentry_len => return Err(libc::EIO),
                Err(x) => {
                    return Err(x);
                }
                _ => {}
            }

            rie.extract_from(rie_slice).map_err(|_| libc::EIO)?;

            // check the index block for entries
            let mut ind_arr_vec: Vec<u8> = vec![0; BSIZE];
            let ind_arr_slice = ind_arr_vec.as_mut_slice();
            match self.readi(ind_arr_slice, BSIZE * rie.block as usize, BSIZE, internals) {
                Ok(x) if x != BSIZE => return Err(libc::EIO),
                Err(x) => {
                    return Err(x);
                }
                _ => {}
            }

            let ind_header_slice = &mut ind_arr_slice[0..hindex_len];
            let mut index = Htree_index::new();
            index
                .extract_from(ind_header_slice)
                .map_err(|_| libc::EIO)?;

            let num_entries = index.entries;
            if num_entries == 0 {
                break;
            }

            // check entries in index node
            for ine_idx in
                (hindex_len..hindex_len + (hentry_len * index.entries as usize)).step_by(hentry_len)
            {
                if ine_idx / hentry_len >= num_entries as usize || ine_idx >= BSIZE {
                    break;
                }

                let ine_slice = &mut ind_arr_slice[ine_idx..ine_idx + hentry_len];
                let mut ine = Htree_entry::new();

                ine.extract_from(ine_slice).map_err(|_| libc::EIO)?;

                let dblock_off = ine.block;
                if dblock_off == 0 {
                    break;
                }
                let mut de_block_vec: Vec<u8> = vec![0; BSIZE];
                let de_block_slice = de_block_vec.as_mut_slice();

                match self.readi(
                    de_block_slice,
                    BSIZE * dblock_off as usize,
                    BSIZE,
                    internals,
                ) {
                    Ok(x) if x != BSIZE => return Err(libc::EIO),
                    Err(x) => {
                        return Err(x);
                    }
                    _ => {}
                }

                // check dirents in leaf node
                for de_off in (0..BSIZE).step_by(de_len) {
                    let de_slice = &mut de_block_slice[de_off..de_off + de_len];
                    let mut de = Xv6fsDirent::new();

                    de.extract_from(de_slice).map_err(|_| libc::EIO)?;

                    if de.inum != 0 {
                        return Ok(false);
                    }
                }
            }
        }

        return Ok(true);
    }
    
    fn dounlink(&self, nodeid: u64, name: &OsStr, handle: &Handle) -> Result<usize, libc::c_int> {
        let parent = self.iget(nodeid)?;
        let icache = self.ilock_cache.as_ref().unwrap();
        let parent_inode_guard = self.ilock(parent.idx, &icache, parent.inum)?;
        let mut parent_internals = parent_inode_guard
            .internals
            .write()
            .map_err(|_| libc::EIO)?;
        let mut poff = 0;
        let name_str = name.to_str().unwrap();
        if name_str == "." || name_str == ".." {
            return Err(libc::EIO);
        }
        let inode = self.dirlookup(&mut parent_internals, name, &mut poff)?;

        let inode_guard = self.ilock(inode.idx, &icache, inode.inum)?;
        let mut inode_internals = inode_guard.internals.write().map_err(|_| libc::EIO)?;

        if inode_internals.nlink < 1 {
            return Err(libc::EIO);
        }

        if inode_internals.inode_type == T_DIR {
            match self.isdirempty(&mut inode_internals) {
                Ok(true) => {}
                _ => {
                    return Err(libc::ENOTEMPTY);
                }
            }
        }

        let de_arr = [0; mem::size_of::<Xv6fsDirent>()];
        let buf_len = mem::size_of::<Xv6fsDirent>();
        let r = self.writei(
            &de_arr,
            poff as usize,
            buf_len,
            &mut parent_internals,
            parent.inum,
            handle
        )?;

        if r != buf_len {
            return Err(libc::EIO);
        }

        if inode_internals.inode_type == T_DIR {
            parent_internals.nlink -= 1;
            self.iupdate(&parent_internals, parent.inum, handle)?;
        }

        inode_internals.nlink -= 1;
        self.iupdate(&inode_internals, inode.inum, handle)?;
    
        return Ok(0);
    }


}
// xv6fs_srv impl ends here
    /*fn bento_update_prepare(&mut self) -> Option<Xv6State> {
        let mut state = Xv6State {
            diskname: self.diskname.as_ref().unwrap().clone(),
            log: None,
        };
        mem::swap(&mut self.log, &mut state.log);
        Some(state)
    }

    fn bento_update_transfer(&mut self, state_opt: Option<Xv6State>) {
        if let Some(mut state) = state_opt {
            let disk = Arc::new(Disk::new(state.diskname.as_str(), 4096));
            self.disk = Some(disk);
            self.diskname = Some(state.diskname);
            let sb_lock = Xv6fsSB {
                size: 0,
                nblocks: 0,
                ninodes: 0,
                nlog: 0,
                logstart: 0,
                inodestart: 0,
                bmapstart: 0,
            };
            self.sb = Some(sb_lock);
            mem::swap(&mut self.log, &mut state.log);

            self.iinit();
        }
    }
}

*/