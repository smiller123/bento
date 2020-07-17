/*
 * SPDX-License-Identifier: GPL-2.0 OR MIT
 *
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 */

use alloc::vec::Vec;

use core::sync::atomic;
use core::str;

use fuse::*;

use time::Timespec;

use std::net::*;

use std::io::{Read, Write, Seek, SeekFrom};

use std::fs::File;
use std::fs::OpenOptions;

use crate::hello_capnp::foo;
use capnp::serialize;

pub const PAGE_SIZE: usize = 4096;

static LEN: atomic::AtomicUsize = atomic::AtomicUsize::new(13);
static HELLO_NAME: &str = "hello";

pub fn hello_srv_runner(devname: &str) {
    let mut disk = OpenOptions::new().read(true).write(true).open(devname).unwrap();
    let srv_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 1234);
    let listener = match TcpListener::bind(SocketAddr::V4(srv_addr)) {
        Ok(x) => x,
        Err(_) => {
            return;
        },
    };
    let mut connection = match listener.accept() {
        Ok((stream, _)) => stream,
        Err(_) => {
            return;
        }
    };
    let message_reader = serialize::read_message(&mut connection, capnp::message::ReaderOptions::new()).unwrap();
    let foo_msg = message_reader.get_root::<foo::Reader>().unwrap();
    let text = foo_msg.get_msg().unwrap();
    println!("got text {}", text);
    loop {
        let mut buf = [0; 4096];
        let size = match connection.read(&mut buf) {
            Ok(x) if x == 0 => break,
            Ok(x) => x,
            Err(_) => {
                let _ = connection.shutdown(Shutdown::Both);
                return;
            },
        };
        let buf_str = str::from_utf8(&buf[0..size]).unwrap();
        let buf_vec: Vec<&str> = buf_str.split(' ').collect();
        let buf_op = buf_vec.get(0).unwrap();
        match *buf_op {
            "open" => {
                if buf_vec.len() < 2 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
                let open_fh: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let open_res = open(open_fh);
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
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
                let open_fh: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let open_res = opendir(open_fh);
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
            "getattr" => {
                if buf_vec.len() < 2 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
                let getattr_fh: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let getattr_res = getattr(getattr_fh);
                match getattr_res {
                    Ok((a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t)) => {
                        let msg = format!("Ok {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
                                          a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t);
                        let _ = connection.write(msg.as_bytes());
                    },
                    Err(x) => {
                        let msg = format!("Err {}", x);
                        let _ = connection.write(msg.as_bytes());
                    },
                }
            },
            "statfs" => {
                let statfs_res = statfs();
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
            "lookup" => {
                if buf_vec.len() < 3 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
                let lookup_id: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let lookup_name: &str = buf_vec.get(2).unwrap();
                let lookup_res = lookup(lookup_id, lookup_name);
                match lookup_res {
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
            "fsync" => {
                if buf_vec.len() < 2 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
                let fsync_res = fsync(&mut disk);
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
            "read" => {
                if buf_vec.len() < 3 {
                    // Send error back
                    let msg = format!("Err {}", libc::EINVAL);
                    let _ = connection.write(msg.as_bytes());
                    continue;
                }
                let read_id: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let read_off: i64 = buf_vec.get(2).unwrap().parse().unwrap();
                let read_res = read(&mut disk, read_id, read_off);
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
                    let _ = connection.write(msg.as_bytes());
                    continue;
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

                let write_res = write(&mut disk, write_id, write_off, write_data);
                match write_res {
                    Ok(a) => {
                        let msg = format!("Ok {}", a);
                        let _ = connection.write(msg.as_bytes());
                    },
                    Err(x) => {
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
                let readdir_id: u64 = buf_vec.get(1).unwrap().parse().unwrap();
                let readdir_off: i64 = buf_vec.get(2).unwrap().parse().unwrap();
                let readdir_res = readdir(readdir_id, readdir_off);
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
            "exit" => break,
            s => println!("got buf {}", s),
        }
    }
    let _ = connection.shutdown(Shutdown::Both);
}

fn hello_stat(ino: u64) -> Result<FileAttr, i32> {
    if ino != 1 && ino != 2 {
        return Err(-1);
    }
    let nlink = match ino {
        1 => 2,
        2 => 1,
        _ => 0,
    };
    let file_type = match ino {
        1 => FileType::Directory,
        2 => FileType::RegularFile,
        _ => FileType::RegularFile,
    };
    let size = match ino {
        1 => 0,
        2 => LEN.load(atomic::Ordering::SeqCst) as u64,
        _ => 0,
    };
    Ok(FileAttr {
        ino: ino,
        size: size,
        blocks: 0,
        atime: Timespec::new(0, 0),
        mtime: Timespec::new(0, 0),
        ctime: Timespec::new(0, 0),
        crtime: Timespec::new(0, 0),
        kind: file_type,
        perm: 0o077,
        nlink: nlink,
        uid: 0,
        gid: 0,
        rdev: 0,
        flags: 0,
    })
}

fn statfs() -> Result<(u64, u64, u64, u64, u64, u32, u32, u32), i32> {
    return Ok((0, 0, 0, 0, 0, 512, 255, 0));
}

fn open(
    nodeid: u64,
) -> Result<(u64, u32), i32> {
    if nodeid != 2 {
        return Err(libc::EISDIR);
    } else {
        return Ok((0,0));
    }
}

fn opendir(
    nodeid: u64,
) -> Result<(u64, u32), i32> {
    if nodeid != 1 {
        return Err(libc::ENOTDIR);
    } else {
        return Ok((0,0));
    }
}

fn getattr(
    nodeid: u64,
) -> Result<
    (i64, i32, u64, u64, u64, i64, i32, i64, i32, i64, i32, i64, i32, u32, u16, u32, u32, u32, u32, u32), i32>{
    let attr_valid = Timespec::new(1, 999999999);
    match hello_stat(nodeid) {
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
                attr.flags
            ));
        },
        Err(_) => return Err(libc::ENOENT),
    }
}

fn lookup(
    nodeid: u64,
    name_str: &str,
) -> Result<
    (i64, i32, u64, u64, u64, i64, i32, i64, i32, i64, i32, i64, i32, u32, u16, u32, u32, u32, u32, u32, u64), i32>{
    if nodeid != 1 || name_str != HELLO_NAME {
        return Err(libc::ENOENT);
    } else {
        let out_nodeid = 2;
        let generation = 0;
        let entry_valid = Timespec::new(1, 999999999);
        match hello_stat(out_nodeid) {
            Ok(attr) => {
                let kind = match attr.kind {
                    FileType::Directory => 1,
                    FileType::RegularFile => 2,
                    _ => 3,
                };
                return Ok((
                    entry_valid.sec,
                    entry_valid.nsec,
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
                    generation
                ));
            },
            Err(_) => return Err(libc::ENOENT),
        }
    }
}

fn read(
    disk: &mut File,
    nodeid: u64,
    offset: i64,
) -> Result<Vec<u8>, i32> {
    if nodeid != 2 {
        return Err(libc::ENOENT);
    }
    let copy_len = LEN.load(atomic::Ordering::SeqCst) - offset as usize;

    let mut buf_vec: Vec<u8> = vec![0; copy_len];
    let buf_slice = buf_vec.as_mut_slice();

    if let Err(_) = disk.seek(SeekFrom::Start(offset as u64)) {
        return Err(libc::EIO); 
    }

    let _size = match disk.read(buf_slice) {
        Ok(x) => x,
        Err(_) => return Err(libc::EIO),
    };
    return Ok(buf_vec);
}

fn write(
    disk: &mut File,
    nodeid: u64,
    offset: i64,
    data: &[u8],
) -> Result<u32, i32>{
    if nodeid != 2 {
        return Err(libc::ENOENT);
    }
    if let Err(_) = disk.seek(SeekFrom::Start(offset as u64)) {
        return Err(libc::EIO); 
    }
    match disk.write(data) {
        Ok(x) => {
            let curr_len = LEN.load(atomic::Ordering::SeqCst);
            if x + offset as usize > curr_len {
                LEN.store(x + offset as usize, atomic::Ordering::SeqCst);
            }
            return Ok(x as u32);
        },
        Err(_) => return Err(libc::EIO),
    }
}

fn readdir(
    nodeid: u64,
    offset: i64,
) -> Result<Vec<(u64, i64, u64, &'static str)>, i32> {
    let mut readdir_vec: Vec<(u64, i64, u64, &str)> = Vec::new();

    if nodeid != 1 {
        return Err(libc::ENOTDIR);
    }
    let mut buf_off = 1;
    let mut inarg_offset = offset;
    if inarg_offset < 1 {
        readdir_vec.push((1 as u64, buf_off, 1, "."));
    }
    inarg_offset -= 1;
    buf_off += 1;
    if inarg_offset < 1 {
        readdir_vec.push((2 as u64, buf_off, 2, HELLO_NAME));
    }
    inarg_offset -= 1;
    buf_off += 1;
    if inarg_offset < 1 {
        readdir_vec.push((1 as u64, buf_off, 1, ".."));
    }
    return Ok(readdir_vec);
}

fn fsync(disk: &mut File) -> Result<(), i32> {
    if let Err(_) = disk.sync_all() {
        return Err(libc::EIO);
    } else {
        return Ok(());
    }
}
