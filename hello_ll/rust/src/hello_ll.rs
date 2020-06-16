/*
 * SPDX-License-Identifier: GPL-2.0 OR MIT
 *
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 */

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

//use bento::println;

use bento::bindings::*;

pub const PAGE_SIZE: usize = 4096;

static LEN: atomic::AtomicUsize = atomic::AtomicUsize::new(13);
static HELLO_NAME: &str = "hello\0";

pub const HELLO_LL_OPS: fs_ops = fs_ops {
    init: hello_init,
    destroy: hello_destroy,
    getattr: hello_getattr,
    lookup: hello_lookup,
    readdir: hello_readdir,
    open: hello_open,
    opendir: hello_opendir,
    read: hello_read,
    statfs: hello_statfs,
    write: hello_write,
    create: hello_create,
    setattr: hello_setattr,
    mkdir: hello_mkdir,
    rmdir: hello_rmdir,
    unlink: hello_unlink,
    lseek: hello_lseek,
    symlink: hello_symlink,
    readlink: hello_readlink,

    ioctl: hello_ioctl,
    fsync: hello_fsync,
    fsyncdir: hello_fsyncdir,

    flush: hello_flush,
    getxattr: hello_getxattr,
    listxattr: hello_listxattr,
    access: hello_access,
    mknod: hello_mknod,
    forget: hello_forget,
    getlk: hello_getlk,
    setlk: hello_setlk,
    bmap: hello_bmap,
    poll: hello_poll,
    fallocate: hello_fallocate,
    setxattr: hello_setxattr,
    removexattr: hello_removexattr,
    rename: hello_rename,
    release: hello_release,
    releasedir: hello_releasedir,
};

#[no_mangle]
pub fn hello_flush(_sb: RsSuperBlock, _nodeid: u64, _inarg: &fuse_flush_in) -> i32 {
    return 0;
}

#[no_mangle]
pub fn hello_getxattr(
    _sb: RsSuperBlock,
    _nodeid: u64,
    _inarg: &fuse_getxattr_in,
    _name: CStr,
    _size: raw::c_size_t,
    _outarg: &mut fuse_getxattr_out,
    _cont: &mut kmem::MemContainer<u8>,
) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn hello_listxattr(
    _sb: RsSuperBlock,
    _nodeid: u64,
    _inarg: &fuse_getxattr_in,
    _size: raw::c_size_t,
    _outarg: &mut fuse_getxattr_out,
    _cont: &mut kmem::MemContainer<u8>,
) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn hello_statfs(_sb: RsSuperBlock, _nodeid: u64, outarg: &mut fuse_statfs_out) -> i32 {
    outarg.st.namelen = 255;
    outarg.st.bsize = 512;
    return 0;
}

#[no_mangle]
pub fn hello_open(
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

#[no_mangle]
pub fn hello_opendir(
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

#[no_mangle]
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

#[no_mangle]
pub fn hello_getattr(
    _sb: RsSuperBlock,
    nodeid: u64,
    _inarg: &fuse_getattr_in,
    outarg: &mut fuse_attr_out,
) -> i32 {
    outarg.attr_valid = 1;
    outarg.attr_valid_nsec = 999999999;
    if hello_stat(nodeid, &mut outarg.attr) == -1 {
        return -(ENOENT as i32);
    } else {
        return 0;
    }
}

#[no_mangle]
pub fn hello_lookup(
    _sb: RsSuperBlock,
    nodeid: u64,
    name: CStr,
    outarg: &mut fuse_entry_out,
) -> i32 {
    let c_name = HELLO_NAME.as_ptr() as *const raw::c_char;
    if nodeid != 1 || strcmp_rs(name.to_raw(), c_name) != 0 {
        return -(ENOENT as i32);
    } else {
        outarg.nodeid = 2;
        outarg.generation = 0;
        outarg.attr_valid = 1;
        outarg.entry_valid = 1;
        outarg.attr_valid_nsec = 999999999;
        outarg.entry_valid_nsec = 999999999;
        if hello_stat(outarg.nodeid, &mut outarg.attr) == -1 {
            return -(ENOENT as i32);
        } else {
            return 0;
        }
    }
}

#[no_mangle]
pub fn hello_read(
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

#[no_mangle]
pub fn hello_write(
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

#[no_mangle]
pub fn hello_readdir(
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

pub fn hello_init(_sb: RsSuperBlock, inarg: &fuse_init_in, outarg: &mut fuse_init_out) -> i32 {
    outarg.major = BENTO_KERNEL_VERSION;
    outarg.minor = BENTO_KERNEL_MINOR_VERSION;

    let mut bufsize = FUSE_MAX_MAX_PAGES * PAGE_SIZE as u32 + FUSE_BUFFER_HEADER_SIZE;
    let mut max_write = u32::MAX;
    let mut max_readahead = u32::MAX;

    if bufsize < FUSE_MIN_READ_BUFFER {
        bufsize = FUSE_MIN_READ_BUFFER;
    }

    if max_write > bufsize - FUSE_BUFFER_HEADER_SIZE {
        max_write = bufsize - FUSE_BUFFER_HEADER_SIZE;
    }

    if inarg.max_readahead < max_readahead {
        max_readahead = inarg.max_readahead;
    }

    outarg.flags |= FUSE_WRITEBACK_CACHE;

    outarg.max_readahead = max_readahead;
    outarg.max_write = max_write;
    outarg.max_background = 0;
    outarg.congestion_threshold = 0;
    outarg.time_gran = 1;

    return 0;
}

pub fn hello_create(
    _sb: RsSuperBlock,
    _nodeid: u64,
    _inarg: &fuse_create_in,
    _name: CStr,
    _outentry: &mut fuse_entry_out,
    _outopen: &mut fuse_open_out,
) -> i32 {
    return -(ENOSYS as i32);
}

pub fn hello_setattr(
    _sb: RsSuperBlock,
    _nodeid: u64,
    _inarg: &fuse_setattr_in,
    _outarg: &mut fuse_attr_out,
) -> i32 {
    return -(ENOSYS as i32);
}

pub fn hello_mkdir(
    _sb: RsSuperBlock,
    _nodeid: u64,
    _inarg: &fuse_mkdir_in,
    _name: CStr,
    _outarg: &mut fuse_entry_out,
) -> i32 {
    return -(ENOSYS as i32);
}

pub fn hello_rmdir(_sb: RsSuperBlock, _nodeid: u64, _name: CStr) -> i32 {
    return -(ENOSYS as i32);
}

pub fn hello_unlink(_sb: RsSuperBlock, _nodeid: u64, _name: CStr) -> i32 {
    return -(ENOSYS as i32);
}

pub fn hello_readlink(_sb: RsSuperBlock, _nodeid: u64, _buf: &mut kmem::MemContainer<u8>) -> i32 {
    return -(ENOSYS as i32);
}

pub fn hello_symlink(
    _sb: RsSuperBlock,
    _nodeid: u64,
    _name: CStr,
    _linkname: CStr,
    _outarg: &mut fuse_entry_out,
) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn hello_lseek(
    _sb: RsSuperBlock,
    _nodeid: u64,
    inarg: &fuse_lseek_in,
    outarg: &mut fuse_lseek_out,
) -> i32 {
    outarg.offset = inarg.offset;
    return 0;
}

#[no_mangle]
pub fn hello_ioctl(
    _sb: RsSuperBlock,
    _nodied: u64,
    _inarg: &fuse_ioctl_in,
    _outarg: &mut fuse_ioctl_out,
    _buf: &mut kmem::MemContainer<u8>,
) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn hello_fsync(sb: RsSuperBlock, _nodied: u64, _inarg: &fuse_fsync_in) -> i32 {
    let mut error_sector = 0;
    return blkdev_issue_flush_rust(&sb.s_bdev(), GFP_KERNEL as usize, &mut error_sector) as i32;
}

#[no_mangle]
pub fn hello_fsyncdir(_sb: RsSuperBlock, _nodied: u64, _inarg: &fuse_fsync_in) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn hello_destroy(_sb: RsSuperBlock) -> i32 {
    return 0;
}

#[no_mangle]
pub fn hello_access(_sb: RsSuperBlock, _nodeid: u64, _inarg: &fuse_access_in) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn hello_mknod(
    _sb: RsSuperBlock,
    _nodeid: u64,
    _inarg: &fuse_mknod_in,
    _name: CStr,
    _outarg: &mut fuse_entry_out,
) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn hello_forget(_sb: RsSuperBlock, _nodeid: u64, _nlookup: u64) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn hello_getlk(
    _sb: RsSuperBlock,
    _nodeid: u64,
    _inarg: &fuse_lk_in,
    _outarg: &mut fuse_lk_out,
) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn hello_setlk(_sb: RsSuperBlock, _nodeid: u64, _inarg: &fuse_lk_in, _sleep: bool) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn hello_bmap(
    _sb: RsSuperBlock,
    _nodeid: u64,
    _inarg: &fuse_bmap_in,
    _outarg: &mut fuse_bmap_out,
) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn hello_poll(
    _sb: RsSuperBlock,
    _nodeid: u64,
    _inarg: &fuse_poll_in,
    _outarg: &mut fuse_poll_out,
) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn hello_fallocate(_sb: RsSuperBlock, _nodeid: u64, _inarg: &fuse_fallocate_in) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn hello_setxattr(
    _sb: RsSuperBlock,
    _nodeid: u64,
    _inarg: &fuse_setxattr_in,
    _name: CStr,
    _value: &kmem::MemContainer<raw::c_uchar>,
) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn hello_removexattr(_sb: RsSuperBlock, _nodeid: u64, _name: CStr) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn hello_rename(
    _sb: RsSuperBlock,
    _nodeid: u64,
    _inarg: &fuse_rename2_in,
    _oldname: CStr,
    _newname: CStr,
) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn hello_release(_sb: RsSuperBlock, _nodeid: u64, _inarg: &fuse_release_in) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn hello_releasedir(_sb: RsSuperBlock, _nodeid: u64, _inarg: &fuse_release_in) -> i32 {
    return -(ENOSYS as i32);
}
