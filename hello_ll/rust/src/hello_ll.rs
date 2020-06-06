use core::sync::atomic;

use bento::bentofs::*;

use bento::kernel;
use kernel::errno;
use kernel::fs::*;
use kernel::fuse::*;
use kernel::kobj::*;
use kernel::mem as kmem;
use kernel::raw;
use kernel::stat;
use kernel::string::*;
use kernel::time::Timespec;

//use bento::println;

use bento::bindings::*;

pub const PAGE_SIZE: usize = 4096;

static LEN: atomic::AtomicUsize = atomic::AtomicUsize::new(13);
static HELLO_NAME: &str = "hello\0";

pub struct HelloFS;

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

impl FileSystem for HelloFS {
    fn get_name(&self) -> &str {
        Self::NAME
    }

    fn init(&mut self, _sb: RsSuperBlock, _req: &Request, outarg: &mut FuseConnInfo) -> Result<(), i32> {
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
    
        outarg.want |= FUSE_WRITEBACK_CACHE;
    
        outarg.max_readahead = max_readahead;
        outarg.max_write = max_write;
        outarg.max_background = 0;
        outarg.congestion_threshold = 0;
        outarg.time_gran = 1;
    
        return Ok(());
    }

    fn flush(&self, _sb: RsSuperBlock, _nodeid: u64, _inarg: &fuse_flush_in) -> i32 {
        return 0;
    }

    fn statfs(&self, _sb: RsSuperBlock, _nodeid: u64, outarg: &mut fuse_statfs_out) -> i32 {
        outarg.st.namelen = 255;
        outarg.st.bsize = 512;
        return 0;
    }

    fn open(&self, 
        _sb: RsSuperBlock,
        nodeid: u64,
        _inarg: &fuse_open_in,
        outarg: &mut fuse_open_out,
    ) -> i32 {
        if nodeid != 2 {
            return -(EISDIR as i32);
        } else {
            outarg.fh = 0;
            outarg.open_flags = 0;
            return 0;
        }
    }

    fn opendir(&self, 
        _sb: RsSuperBlock,
        nodeid: u64,
        _inarg: &fuse_open_in,
        outarg: &mut fuse_open_out,
    ) -> i32 {
        if nodeid != 1 {
            return -(ENOTDIR as i32);
        } else {
            outarg.fh = 0;
            outarg.open_flags = 0;
            return 0;
        }
    }

    fn getattr(&mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        nodeid: u64,
        reply: ReplyAttr,
    ) {
        let attr_valid = Timespec::new(1, 999999999);
        let mut attr = fuse_attr::new();
        if HelloFS::hello_stat(nodeid, &mut attr) == -1 {
            reply.error(-(ENOENT as i32));
        } else {
            reply.attr(&attr_valid, &attr);
        }
    }

    fn lookup(&mut self, 
        _sb: RsSuperBlock,
        _req: &Request,
        nodeid: u64,
        name: CStr,
        reply: ReplyEntry,
    ) {
        let c_name = HELLO_NAME.as_ptr() as *const raw::c_char;
        if nodeid != 1 || strcmp_rs(name.to_raw(), c_name) != 0 {
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

    fn read(&self, 
        sb: RsSuperBlock,
        nodeid: u64,
        inarg: &fuse_read_in,
        buf: &mut kmem::MemContainer<u8>,
    ) -> i32 {
        if nodeid != 2 {
            return -(ENOENT as i32);
        }
        let copy_len = LEN.load(atomic::Ordering::SeqCst) - inarg.offset as usize;
    
        let maybe_bh = sb_bread_rust(&sb, 0);
        let bh;
        match maybe_bh {
            None => return -(EIO as i32),
            Some(x) => bh = x,
        }
        let b_data = bh.get_buffer_data();
        let b_slice = b_data.to_slice();
        let offset = inarg.offset as usize;
        let data_region = &b_slice[offset..offset + copy_len];
        let buf_slice = buf.to_slice_mut();
        let buf_region = &mut buf_slice[..copy_len];
        buf_region.copy_from_slice(data_region);
        return 0;
    }

    fn write(&self, 
        sb: RsSuperBlock,
        nodeid: u64,
        inarg: &fuse_write_in,
        buf: &kmem::MemContainer<u8>,
        outarg: &mut fuse_write_out,
    ) -> i32 {
        let total_len = inarg.size as usize + inarg.offset as usize;
    
        if nodeid != 2 {
            return -(ENOENT as i32);
        }
    
        let maybe_bh = sb_bread_rust(&sb, 0);
        let mut bh;
        match maybe_bh {
            None => return -(EIO as i32),
            Some(x) => bh = x,
        }
        {
            let mut b_data = bh.get_buffer_data();
            let offset = inarg.offset as usize;
            let b_slice = b_data.to_slice_mut();
            let copy_size = inarg.size as usize;
            let write_region = &mut b_slice[offset..offset + copy_size];
            let buf_slice = buf.to_slice();
            let data_region = &buf_slice[offset..offset + copy_size];
            write_region.copy_from_slice(data_region);
            LEN.store(total_len, atomic::Ordering::SeqCst);
        }
    
        bh.mark_buffer_dirty();
        bh.sync_dirty_buffer();
        outarg.size = inarg.size;
        return 0;
    }

    fn readdir(&self, 
        _sb: RsSuperBlock,
        nodeid: u64,
        inarg: &fuse_read_in,
        buf: &mut kmem::MemContainer<u8>,
        size: &mut usize,
    ) -> i32 {
        if nodeid != 1 {
            return -(ENOTDIR as i32);
        }
        if let Err(x) = kmem::memset_rust(buf, 0, buf.len() as u64) {
            return x as i32;
        }
        let mut buf_off = 0;
        let mut inarg_offset = inarg.offset as usize;
        let buf_slice = buf.to_slice_mut();
        let curr_buf_slice = &mut buf_slice[buf_off..];
        let mut ent_len = match bento_add_direntry(
            curr_buf_slice,
            ".",
            1 as u64,
            0,
            buf_off as u64 + inarg.offset,
        ) {
            Ok(x) => x,
            Err(errno::Error::EOVERFLOW) => return 0,
            Err(x) => return x as i32,
        };
        if ent_len <= inarg_offset {
            inarg_offset -= ent_len;
        } else {
            buf_off += ent_len;
        }
        let curr_buf_slice = &mut buf_slice[buf_off..];
        ent_len = match bento_add_direntry(
            curr_buf_slice,
            HELLO_NAME,
            2 as u64,
            0,
            buf_off as u64 + inarg.offset,
        ) {
            Ok(x) => x,
            Err(errno::Error::EOVERFLOW) => return 0,
            Err(x) => return x as i32,
        };
        if ent_len <= inarg_offset {
            inarg_offset -= ent_len;
        } else {
            buf_off += ent_len;
        }
        let curr_buf_slice = &mut buf_slice[buf_off..];
        ent_len = match bento_add_direntry(
            curr_buf_slice,
            "..",
            1 as u64,
            0,
            buf_off as u64 + inarg.offset,
        ) {
            Ok(x) => x,
            Err(errno::Error::EOVERFLOW) => return 0,
            Err(x) => return x as i32,
        };
        if ent_len > inarg_offset {
            buf_off += ent_len;
        }
        *size = buf_off;
        return 0;
    }

    fn lseek(&self, 
        _sb: RsSuperBlock,
        _nodeid: u64,
        inarg: &fuse_lseek_in,
        outarg: &mut fuse_lseek_out,
    ) -> i32 {
        outarg.offset = inarg.offset;
        return 0;
    }

    fn fsync(&self, sb: RsSuperBlock, _nodied: u64, _inarg: &fuse_fsync_in) -> i32 {
        let mut error_sector = 0;
        return blkdev_issue_flush_rust(&sb.s_bdev(), GFP_KERNEL as usize, &mut error_sector) as i32;
    }
}
