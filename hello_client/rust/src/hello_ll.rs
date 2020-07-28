/*
 * SPDX-License-Identifier: GPL-2.0 OR MIT
 *
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 */

#[cfg(not(feature="user"))]
use crate::bento_utils;
#[cfg(not(feature="user"))]
use crate::fuse;
#[cfg(not(feature="user"))]
use crate::libc;
#[cfg(not(feature="user"))]
use crate::std;
#[cfg(not(feature="user"))]
use crate::time;
//#[cfg(not(feature="user"))]
//use crate::println;

use alloc::vec::Vec;

use bento_utils::*;

use core::str;

use fuse::*;

use std::ffi::OsStr;

use time::Timespec;

use std::net::*;
use std::io::{Read, Write};

use crate::hello_capnp::foo;
use capnp::serialize;

pub const PAGE_SIZE: usize = 4096;

pub struct HelloFS {
    pub socket: Option<TcpStream>,
}

impl HelloFS {
    const NAME: &'static str = "hello_client\0";
}

impl BentoFilesystem<'_> for HelloFS {
    fn get_name(&self) -> &'static str {
        Self::NAME
    }

    fn bento_init(
        &mut self,
        _req: &Request,
        _devname: &OsStr,
        outarg: &mut FuseConnInfo,
    ) -> Result<(), i32> {
        outarg.proto_major = BENTO_KERNEL_VERSION;
        outarg.proto_minor = BENTO_KERNEL_MINOR_VERSION;

        let mut max_readahead = u32::MAX;
        if outarg.max_readahead < max_readahead {
            max_readahead = outarg.max_readahead;
        }

        outarg.max_readahead = max_readahead;
        outarg.max_background = 0;
        outarg.congestion_threshold = 0;
        outarg.time_gran = 1;

        let srv_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 1234);
        let mut stream = match TcpStream::connect(SocketAddr::V4(srv_addr)) {
            Ok(x) => x,
            Err(_) => return Err(-1),
        };
        let mut message = capnp::message::Builder::new_default();
        let mut foo_msg = message.init_root::<foo::Builder>();
        foo_msg.set_msg("hello");
        serialize::write_message(&mut stream, &message);
        self.socket = Some(stream);

        return Ok(());
    }

    fn bento_destroy(&mut self, _req: &Request) {
        let msg = "exit";
        let _size = match self.socket.as_ref().unwrap().write(msg.as_bytes()) {
            Ok(x) => x,
            Err(_) => return,
        };
        let _ = self.socket.as_ref().unwrap().shutdown(Shutdown::Both);
        self.socket = None;
    }

    fn bento_statfs(&self, _req: &Request, _ino: u64, reply: ReplyStatfs) {
        let msg = format!("statfs");
        let _ = self.socket.as_ref().unwrap().write(msg.as_bytes());
        let mut msg_resp = [0 as u8; 4096];
        let size = match self.socket.as_ref().unwrap().read(&mut msg_resp) {
            Ok(x) => x,
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };
        let statfs_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
        let statfs_vec: Vec<&str> = statfs_msg.split(' ').collect();
        match *statfs_vec.get(0).unwrap() {
            "Ok" => {
                if statfs_vec.len() < 9 {
                    reply.error(libc::EINVAL);
                } else {
                    let blocks: u64 = statfs_vec.get(1).unwrap().parse().unwrap();
                    let bfree: u64 = statfs_vec.get(2).unwrap().parse().unwrap();
                    let bavail: u64 = statfs_vec.get(3).unwrap().parse().unwrap();
                    let files: u64 = statfs_vec.get(4).unwrap().parse().unwrap();
                    let ffree: u64 = statfs_vec.get(5).unwrap().parse().unwrap();
                    let bsize: u32 = statfs_vec.get(6).unwrap().parse().unwrap();
                    let namelen: u32 = statfs_vec.get(7).unwrap().parse().unwrap();
                    let frsize: u32 = statfs_vec.get(8).unwrap().parse().unwrap();
                    reply.statfs(
                        blocks,
                        bfree,
                        bavail,
                        files,
                        ffree,
                        bsize,
                        namelen,
                        frsize
                    );
                }
            }
            "Err" => {
                let err_val: i32 = statfs_vec.get(1).unwrap().parse().unwrap();
                reply.error(err_val);
            },
            _ => reply.error(libc::EINVAL),
        }
    }

    fn bento_open(
        &self,
        _req: &Request,
        nodeid: u64,
        _flags: u32,
        reply: ReplyOpen,
    ) {
        let msg = format!("open {}", nodeid);
        let _ = self.socket.as_ref().unwrap().write(msg.as_bytes());
        let mut msg_resp = [0 as u8; 4096];
        let size = match self.socket.as_ref().unwrap().read(&mut msg_resp) {
            Ok(x) => x,
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };
        let open_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
        let open_vec: Vec<&str> = open_msg.split(' ').collect();
        match *open_vec.get(0).unwrap() {
            "Ok" => {
                if open_vec.len() < 3 {
                    reply.error(libc::EINVAL);
                } else {
                    let fh: u64 = open_vec.get(1).unwrap().parse().unwrap();
                    let flags: u32 = open_vec.get(2).unwrap().parse().unwrap();
                    reply.opened(fh, flags);
                }
            }
            "Err" => {
                let err_val: i32 = open_vec.get(1).unwrap().parse().unwrap();
                reply.error(err_val);
            },
            _ => reply.error(libc::EINVAL),
        }
    }

    fn bento_opendir(
        &self,
        _req: &Request,
        nodeid: u64,
        _flags: u32,
        reply: ReplyOpen,
    ) {
        let msg = format!("opendir {}", nodeid);
        let _ = self.socket.as_ref().unwrap().write(msg.as_bytes());
        let mut msg_resp = [0 as u8; 4096];
        let size = match self.socket.as_ref().unwrap().read(&mut msg_resp) {
            Ok(x) => x,
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };
        let open_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
        let open_vec: Vec<&str> = open_msg.split(' ').collect();
        match *open_vec.get(0).unwrap() {
            "Ok" => {
                if open_vec.len() < 3 {
                    reply.error(libc::EINVAL);
                } else {
                    let fh: u64 = open_vec.get(1).unwrap().parse().unwrap();
                    let flags: u32 = open_vec.get(2).unwrap().parse().unwrap();
                    reply.opened(fh, flags);
                }
            }
            "Err" => {
                let err_val: i32 = open_vec.get(1).unwrap().parse().unwrap();
                reply.error(err_val);
            },
            _ => reply.error(libc::EINVAL),
        }
    }

    fn bento_getattr(&self, _req: &Request, nodeid: u64, reply: ReplyAttr) {
        let msg = format!("getattr {}", nodeid);
        let _ = self.socket.as_ref().unwrap().write(msg.as_bytes());
        let mut msg_resp = [0 as u8; 4096];
        let size = match self.socket.as_ref().unwrap().read(&mut msg_resp) {
            Ok(x) => x,
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };
        let attr_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
        let attr_vec: Vec<&str> = attr_msg.split(' ').collect();
        match *attr_vec.get(0).unwrap() {
            "Ok" => {
                if attr_vec.len() < 21 {
                    reply.error(libc::EINVAL);
                } else {
                    let ts_sec: i64 = attr_vec.get(1).unwrap().parse().unwrap();
                    let ts_nsec: i32 = attr_vec.get(2).unwrap().parse().unwrap();
                    let attr_valid = Timespec::new(ts_sec, ts_nsec);

                    let ino: u64 = attr_vec.get(3).unwrap().parse().unwrap();
                    let size: u64 = attr_vec.get(4).unwrap().parse().unwrap();
                    let blocks: u64 = attr_vec.get(5).unwrap().parse().unwrap();

                    let atime_sec: i64 = attr_vec.get(6).unwrap().parse().unwrap();
                    let atime_nsec: i32 = attr_vec.get(7).unwrap().parse().unwrap();

                    let mtime_sec: i64 = attr_vec.get(8).unwrap().parse().unwrap();
                    let mtime_nsec: i32 = attr_vec.get(9).unwrap().parse().unwrap();

                    let ctime_sec: i64 = attr_vec.get(10).unwrap().parse().unwrap();
                    let ctime_nsec: i32 = attr_vec.get(11).unwrap().parse().unwrap();

                    let crtime_sec: i64 = attr_vec.get(12).unwrap().parse().unwrap();
                    let crtime_nsec: i32 = attr_vec.get(13).unwrap().parse().unwrap();

                    let kind: FileType = match attr_vec.get(14).unwrap().parse().unwrap() {
                        1 => FileType::Directory,
                        _ => FileType::RegularFile,
                    };

                    let perm: u16 = attr_vec.get(15).unwrap().parse().unwrap();
                    let nlink: u32 = attr_vec.get(16).unwrap().parse().unwrap();
                    let uid: u32 = attr_vec.get(17).unwrap().parse().unwrap();
                    let gid: u32 = attr_vec.get(18).unwrap().parse().unwrap();
                    let rdev: u32 = attr_vec.get(19).unwrap().parse().unwrap();
                    let flags: u32 = attr_vec.get(20).unwrap().parse().unwrap();
                    let attr = FileAttr {
                        ino: ino,
                        size: size,
                        blocks: blocks,
                        atime: Timespec::new(atime_sec, atime_nsec),
                        mtime: Timespec::new(mtime_sec, mtime_nsec),
                        ctime: Timespec::new(ctime_sec, ctime_nsec),
                        crtime: Timespec::new(crtime_sec, crtime_nsec),
                        kind: kind,
                        perm: perm,
                        nlink: nlink,
                        uid: uid,
                        gid: gid,
                        rdev: rdev,
                        flags: flags,
                    };
                    reply.attr(&attr_valid, &attr);
                }
            }
            "Err" => {
                let err_val: i32 = attr_vec.get(1).unwrap().parse().unwrap();
                reply.error(err_val);
            },
            _ => reply.error(libc::EINVAL),
        }
    }

    fn bento_lookup(
        &self,
        _req: &Request,
        nodeid: u64,
        name: &OsStr,
        reply: ReplyEntry,
    ) {
        let name_str = name.to_str().unwrap();
        let msg = format!("lookup {} {}", nodeid, name_str);
        let _ = self.socket.as_ref().unwrap().write(msg.as_bytes());
        let mut msg_resp = [0 as u8; 4096];
        let size = match self.socket.as_ref().unwrap().read(&mut msg_resp) {
            Ok(x) => x,
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };
        let attr_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
        let attr_vec: Vec<&str> = attr_msg.split(' ').collect();
        match *attr_vec.get(0).unwrap() {
            "Ok" => {
                if attr_vec.len() < 22 {
                    reply.error(libc::EINVAL);
                } else {
                    let ts_sec: i64 = attr_vec.get(1).unwrap().parse().unwrap();
                    let ts_nsec: i32 = attr_vec.get(2).unwrap().parse().unwrap();
                    let attr_valid = Timespec::new(ts_sec, ts_nsec);

                    let ino: u64 = attr_vec.get(3).unwrap().parse().unwrap();
                    let size: u64 = attr_vec.get(4).unwrap().parse().unwrap();
                    let blocks: u64 = attr_vec.get(5).unwrap().parse().unwrap();

                    let atime_sec: i64 = attr_vec.get(6).unwrap().parse().unwrap();
                    let atime_nsec: i32 = attr_vec.get(7).unwrap().parse().unwrap();

                    let mtime_sec: i64 = attr_vec.get(8).unwrap().parse().unwrap();
                    let mtime_nsec: i32 = attr_vec.get(9).unwrap().parse().unwrap();

                    let ctime_sec: i64 = attr_vec.get(10).unwrap().parse().unwrap();
                    let ctime_nsec: i32 = attr_vec.get(11).unwrap().parse().unwrap();

                    let crtime_sec: i64 = attr_vec.get(12).unwrap().parse().unwrap();
                    let crtime_nsec: i32 = attr_vec.get(13).unwrap().parse().unwrap();

                    let kind: FileType = match attr_vec.get(14).unwrap().parse().unwrap() {
                        1 => FileType::Directory,
                        _ => FileType::RegularFile,
                    };

                    let perm: u16 = attr_vec.get(15).unwrap().parse().unwrap();
                    let nlink: u32 = attr_vec.get(16).unwrap().parse().unwrap();
                    let uid: u32 = attr_vec.get(17).unwrap().parse().unwrap();
                    let gid: u32 = attr_vec.get(18).unwrap().parse().unwrap();
                    let rdev: u32 = attr_vec.get(19).unwrap().parse().unwrap();
                    let flags: u32 = attr_vec.get(20).unwrap().parse().unwrap();
                    let generation: u64 = attr_vec.get(21).unwrap().parse().unwrap();
                    let attr = FileAttr {
                        ino: ino,
                        size: size,
                        blocks: blocks,
                        atime: Timespec::new(atime_sec, atime_nsec),
                        mtime: Timespec::new(mtime_sec, mtime_nsec),
                        ctime: Timespec::new(ctime_sec, ctime_nsec),
                        crtime: Timespec::new(crtime_sec, crtime_nsec),
                        kind: kind,
                        perm: perm,
                        nlink: nlink,
                        uid: uid,
                        gid: gid,
                        rdev: rdev,
                        flags: flags,
                    };
                    reply.entry(&attr_valid, &attr, generation);
                }
            }
            "Err" => {
                let err_val: i32 = attr_vec.get(1).unwrap().parse().unwrap();
                reply.error(err_val);
            },
            _ => reply.error(libc::EINVAL),
        }
    }

    fn bento_read(
        &self,
        _req: &Request,
        nodeid: u64,
        _fh: u64,
        offset: i64,
        _size: u32,
        reply: ReplyData,
    ) {
        let msg = format!("read {} {}", nodeid, offset);
        let _ = self.socket.as_ref().unwrap().write(msg.as_bytes());
        let mut msg_resp = [0 as u8; 4096];
        let size = match self.socket.as_ref().unwrap().read(&mut msg_resp) {
            Ok(x) => x,
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };
        let read_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
        let read_vec: Vec<&str> = read_msg.split(' ').collect();
        match *read_vec.get(0).unwrap() {
            "Ok" => {
                let data = &msg_resp[3..size];
                reply.data(data);
            }
            "Err" => {
                let err_val: i32 = read_vec.get(1).unwrap().parse().unwrap();
                reply.error(err_val);
            },
            _ => reply.error(libc::EINVAL),
        }
    }

    fn bento_write(
        &self,
        _req: &Request,
        nodeid: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _flags: u32,
        reply: ReplyWrite,
    ) {
        let msg = format!("write {} {} {}", nodeid, offset, str::from_utf8(data).unwrap());
        let _ = self.socket.as_ref().unwrap().write(msg.as_bytes());
        let mut msg_resp = [0 as u8; 4096];
        let size = match self.socket.as_ref().unwrap().read(&mut msg_resp) {
            Ok(x) => x,
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };
        let write_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
        let write_vec: Vec<&str> = write_msg.split(' ').collect();
        match *write_vec.get(0).unwrap() {
            "Ok" => {
                if write_vec.len() < 2 {
                    reply.error(libc::EINVAL);
                } else {
                    let size: u32 = write_vec.get(1).unwrap().parse().unwrap();
                    reply.written(size);
                }
            }
            "Err" => {
                let err_val: i32 = write_vec.get(1).unwrap().parse().unwrap();
                reply.error(err_val);
            },
            _ => reply.error(libc::EINVAL),
        }
    }

    #[allow(unused_mut)]
    fn bento_readdir(
        &self,
        _req: &Request,
        nodeid: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let msg = format!("readdir {} {}", nodeid, offset);
        let _ = self.socket.as_ref().unwrap().write(msg.as_bytes());
        let mut msg_resp = [0 as u8; 4096];
        let size = match self.socket.as_ref().unwrap().read(&mut msg_resp) {
            Ok(x) => x,
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };
        let readdir_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
        let mut readdir_vec: Vec<&str> = readdir_msg.split(' ').collect();
        while readdir_vec.len() > 0 {
            match *readdir_vec.get(0).unwrap() {
                "Add" => {
                    if readdir_vec.len() < 5 {
                        reply.error(libc::EINVAL);
                        return;
                    } else {
                        let ino: u64 = readdir_vec.get(1).unwrap().parse().unwrap();
                        let offset: i64 = readdir_vec.get(2).unwrap().parse().unwrap();
                        let kind: FileType = match readdir_vec.get(3).unwrap().parse().unwrap() {
                            1 => FileType::Directory,
                            _ => FileType::RegularFile,
                        };
                        let name: &str = readdir_vec.get(4).unwrap();
                        reply.add(ino, offset, kind, name);
                        readdir_vec = readdir_vec.drain(5..).collect();
                    }
                }
                "Ok" => {
                    reply.ok();
                    return;
                },
                "Err" => {
                    let err_val: i32 = readdir_vec.get(1).unwrap().parse().unwrap();
                    reply.error(err_val);
                    return;
                },
                _ => {
                    reply.error(libc::EINVAL);
                    return;
                },
            }
        }
    }

    fn bento_fsync(
        &self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        let msg = format!("fsync");
        let _ = self.socket.as_ref().unwrap().write(msg.as_bytes());
        let mut msg_resp = [0 as u8; 4096];
        let size = match self.socket.as_ref().unwrap().read(&mut msg_resp) {
            Ok(x) => x,
            Err(_) => {
                reply.error(libc::EIO);
                return;
            }
        };
        let open_msg = str::from_utf8(&msg_resp[0..size]).unwrap();
        let open_vec: Vec<&str> = open_msg.split(' ').collect();
        match *open_vec.get(0).unwrap() {
            "Ok" => {
                reply.ok();
            }
            "Err" => {
                let err_val: i32 = open_vec.get(1).unwrap().parse().unwrap();
                reply.error(err_val);
            },
            _ => reply.error(libc::EINVAL),
        }
    }
}
