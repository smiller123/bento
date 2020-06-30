use alloc::vec::Vec;
use core::sync::atomic;
use core::str;

use bento::fuse::reply::*;
use bento::fuse::request::*;
use bento::fuse::*;

use bento::kernel;
use kernel::fs::*;
use kernel::fuse::*;
use kernel::stat;
use kernel::time::Timespec;

//use bento::println;

use bento::bindings::*;

use bento::std::ffi::OsStr;
use bento::std::sync::RwLock;

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

    fn hello_stat(ino: u64, stbuf: &mut fuse_attr) -> i32 {
        stbuf.ino = ino;
        stbuf.size = 0;
        stbuf.blocks = 0;
        stbuf.atime = 0;
        stbuf.mtime = 0;
        stbuf.ctime = 0;
        stbuf.atimensec = 0;
        stbuf.mtimensec = 0;
        stbuf.ctimensec = 0;
        stbuf.uid = 0;
        stbuf.gid = 0;
        stbuf.rdev = 0;
        stbuf.blksize = 0;
        match ino {
            1 => {
                stbuf.mode = (stat::S_IFDIR | 0777) as u32;
                stbuf.nlink = 2;
            }
            2 => {
                stbuf.mode = (stat::S_IFREG | 0777) as u32;
                stbuf.nlink = 1;
                stbuf.size = LEN.load(atomic::Ordering::SeqCst) as u64;
            }
            _ => return -1,
        };
        return 0;
    }
}

impl Filesystem for HelloFS {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn init(
        &mut self,
        _req: &Request,
        devname: &OsStr,
        outarg: &mut FuseConnInfo,
    ) -> Result<(), i32> {
        outarg.proto_major = BENTO_KERNEL_VERSION;
        outarg.proto_minor = BENTO_KERNEL_MINOR_VERSION;

        let mut bufsize = FUSE_MAX_MAX_PAGES * PAGE_SIZE as u32 + FUSE_BUFFER_HEADER_SIZE;
        let mut max_write = u32::MAX;
        let mut max_readahead = u32::MAX;

        if bufsize < FUSE_MIN_READ_BUFFER {
            bufsize = FUSE_MIN_READ_BUFFER;
        }

        if max_write > bufsize - FUSE_BUFFER_HEADER_SIZE {
            max_write = bufsize - FUSE_BUFFER_HEADER_SIZE;
        }

        if outarg.max_readahead < max_readahead {
            max_readahead = outarg.max_readahead;
        }

        outarg.max_readahead = max_readahead;
        outarg.max_write = max_write;
        outarg.max_background = 0;
        outarg.congestion_threshold = 0;
        outarg.time_gran = 1;

        //let mut mut_disk = DISK.write();
        //let devname_str = str::from_utf8(devname.to_bytes_with_nul()).unwrap();
        let devname_str = devname.to_str().unwrap();
        let disk = RwLock::new(Disk::new(devname_str, 4096));
        self.disk = Some(disk);
        //*mut_disk = Some(Disk::new(devname_str, 4096));

        return Ok(());
    }

    fn statfs(&self, _req: &Request, _ino: u64, reply: ReplyStatfs) {
        reply.statfs(0, 0, 0, 0, 0, 512, 255, 0);
    }

    fn open(
        &self,
        _req: &Request,
        nodeid: u64,
        _flags: u32,
        reply: ReplyOpen,
    ) {
        if nodeid != 2 {
            reply.error(-(EISDIR as i32));
        } else {
            reply.opened(0, 0);
        }
    }

    fn opendir(
        &self,
        _req: &Request,
        nodeid: u64,
        _flags: u32,
        reply: ReplyOpen,
    ) {
        if nodeid != 1 {
            reply.error(-(EISDIR as i32));
        } else {
            reply.opened(0, 0);
        }
    }

    fn getattr(&self, _req: &Request, nodeid: u64, reply: ReplyAttr) {
        let attr_valid = Timespec::new(1, 999999999);
        let mut attr = fuse_attr::new();
        if HelloFS::hello_stat(nodeid, &mut attr) == -1 {
            reply.error(-(ENOENT as i32));
        } else {
            reply.attr(&attr_valid, &attr);
        }
    }

    fn lookup(
        &self,
        _req: &Request,
        nodeid: u64,
        name: &OsStr,
        reply: ReplyEntry,
    ) {
        let name_str = name.to_str().unwrap();
        if nodeid != 1 || name_str != HELLO_NAME {
            reply.error(-(ENOENT as i32));
        } else {
            let out_nodeid = 2;
            let generation = 0;
            let entry_valid = Timespec::new(1, 999999999);
            let mut attr = fuse_attr::new();
            if HelloFS::hello_stat(out_nodeid, &mut attr) == -1 {
                reply.error(-(ENOENT as i32));
            } else {
                reply.entry(&entry_valid, &attr, generation);
            }
        }
    }

    fn read(
        &self,
        _req: &Request,
        nodeid: u64,
        _fh: u64,
        offset: i64,
        _size: u32,
        reply: ReplyData,
    ) {
        if nodeid != 2 {
            reply.error(-(ENOENT as i32));
            return;
        }
        let copy_len = LEN.load(atomic::Ordering::SeqCst) - offset as usize;

        let disk = self.disk.as_ref().unwrap().read();
        let mut bh = match disk.bread(0) {
            Ok(x) => x,
            Err(x)=> {
                reply.error(x as i32);
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

    fn write(
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
            reply.error(-(ENOENT as i32));
            return;
        }

        let disk = self.disk.as_ref().unwrap().read();
        let mut bh = match disk.bread(0) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
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

    fn readdir(
        &self,
        _req: &Request,
        nodeid: u64,
        _fh: u64,
        offset: i64,
        reply: ReplyDirectory,
    ) {
        if nodeid != 1 {
            reply.error(-(ENOTDIR as i32));
            return;
        }
        let mut buf_off = 1;
        let mut inarg_offset = offset;
        if inarg_offset < 1 {
            if reply.add(1 as u64, buf_off, 0, ".") {
                reply.ok();
                return;
            };
        }
        inarg_offset -= 1;
        buf_off += 1;
        if inarg_offset < 1 {
            if reply.add(2 as u64, buf_off, 0, HELLO_NAME) {
                reply.ok();
                return;
            };
        }
        inarg_offset -= 1;
        buf_off += 1;
        if inarg_offset < 1 {
            if reply.add(1 as u64, buf_off, 0, "..") {
                reply.ok();
                return;
            };
        }
        reply.ok();
    }

    fn fsync(
        &self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        let disk = self.disk.as_ref().unwrap().read();
        if let Err(x) = disk.sync_all() {
            reply.error(x as i32);
        } else {
            reply.ok();
        }
    }
}
