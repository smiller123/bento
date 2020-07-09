use crate::bento_utils;
use crate::fuse;
use crate::libc;
use crate::std;
use crate::time;

use alloc::vec::Vec;

use bento_utils::*;

use core::sync::atomic;
use core::str;

use fuse::*;

//use crate::println;

use std::ffi::OsStr;
use std::sync::RwLock;

use time::Timespec;

pub const PAGE_SIZE: usize = 4096;

static LEN: atomic::AtomicUsize = atomic::AtomicUsize::new(13);
static HELLO_NAME: &str = "hello";

pub static HELLO_FS: HelloFS = HelloFS {
    disk: None
};

pub struct HelloFS {
    disk: Option<RwLock<Disk>>
}

impl HelloFS {
    const NAME: &'static str = "hello_ll\0";

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
}

impl BentoFilesystem for HelloFS {
    fn get_name(&self) -> &'static str {
        Self::NAME
    }

    fn bento_init(
        &mut self,
        _req: &Request,
        devname: &OsStr,
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

        let devname_str = devname.to_str().unwrap();
        let disk = RwLock::new(Disk::new(devname_str, 4096));
        self.disk = Some(disk);

        return Ok(());
    }

    fn bento_statfs(&self, _req: &Request, _ino: u64, reply: ReplyStatfs) {
        reply.statfs(0, 0, 0, 0, 0, 512, 255, 0);
    }

    fn bento_open(
        &self,
        _req: &Request,
        nodeid: u64,
        _flags: u32,
        reply: ReplyOpen,
    ) {
        if nodeid != 2 {
            reply.error(libc::EISDIR);
        } else {
            reply.opened(0, 0);
        }
    }

    fn bento_opendir(
        &self,
        _req: &Request,
        nodeid: u64,
        _flags: u32,
        reply: ReplyOpen,
    ) {
        if nodeid != 1 {
            reply.error(libc::EISDIR);
        } else {
            reply.opened(0, 0);
        }
    }

    fn bento_getattr(&self, _req: &Request, nodeid: u64, reply: ReplyAttr) {
        let attr_valid = Timespec::new(1, 999999999);
        match HelloFS::hello_stat(nodeid) {
            Ok(attr) => reply.attr(&attr_valid, &attr),
            Err(_) => reply.error(libc::ENOENT),
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
        if nodeid != 1 || name_str != HELLO_NAME {
            reply.error(libc::ENOENT);
        } else {
            let out_nodeid = 2;
            let generation = 0;
            let entry_valid = Timespec::new(1, 999999999);
            match HelloFS::hello_stat(out_nodeid) {
                Ok(attr) => reply.entry(&entry_valid, &attr, generation),
                Err(_) => reply.error(libc::ENOENT),
            }
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
        if nodeid != 2 {
            reply.error(libc::ENOENT);
            return;
        }
        let copy_len = LEN.load(atomic::Ordering::SeqCst) - offset as usize;

        let disk = self.disk.as_ref().unwrap().read().unwrap();
        let mut bh = match disk.bread(0) {
            Ok(x) => x,
            Err(x)=> {
                reply.error(x);
                return;
            }
        };

        let mut buf_vec: Vec<u8> = vec![0; copy_len];
        let buf_slice = buf_vec.as_mut_slice();

        let b_slice = bh.data_mut();
        let offset = offset as usize;
        let data_region = &b_slice[offset..offset + copy_len];
        buf_slice.copy_from_slice(data_region);
        reply.data(&buf_slice);
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
        let total_len = data.len() + offset as usize;

        if nodeid != 2 {
            reply.error(libc::ENOENT);
            return;
        }

        let disk = self.disk.as_ref().unwrap().read().unwrap();
        let mut bh = match disk.bread(0) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x);
                return;
            }
        };
        {
            let b_slice = bh.data_mut();
            let offset = offset as usize;
            let copy_size = data.len();
            let write_region = &mut b_slice[offset..offset + copy_size];
            let data_region = &data[..copy_size];
            write_region.copy_from_slice(data_region);
            LEN.store(total_len, atomic::Ordering::SeqCst);
        }

        bh.mark_buffer_dirty();
        bh.sync_dirty_buffer();
        reply.written(data.len() as u32);
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
        if nodeid != 1 {
            reply.error(libc::ENOTDIR);
            return;
        }
        let mut buf_off = 1;
        let mut inarg_offset = offset;
        if inarg_offset < 1 {
            if reply.add(1 as u64, buf_off, FileType::Directory, ".") {
                reply.ok();
                return;
            };
        }
        inarg_offset -= 1;
        buf_off += 1;
        if inarg_offset < 1 {
            if reply.add(2 as u64, buf_off, FileType::RegularFile, HELLO_NAME) {
                reply.ok();
                return;
            };
        }
        inarg_offset -= 1;
        buf_off += 1;
        if inarg_offset < 1 {
            if reply.add(1 as u64, buf_off, FileType::Directory, "..") {
                reply.ok();
                return;
            };
        }
        reply.ok();
    }

    fn bento_fsync(
        &self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        let disk = self.disk.as_ref().unwrap().read().unwrap();
        if let Err(x) = disk.sync_all() {
            reply.error(x);
        } else {
            reply.ok();
        }
    }
}
