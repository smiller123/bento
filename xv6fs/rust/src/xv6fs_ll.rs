/*
 * SPDX-License-Identifier: GPL-2.0 OR MIT
 *
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 * Copyright (C) 2006-2018 Frans Kaashoek, Robert Morris, Russ Cox,
 *                      Massachusetts Institute of Technology
 */

use core::mem;
use core::str;

use bento::kernel;
use kernel::errno;
use kernel::fs::*;
use kernel::fuse::*;
use kernel::kobj::*;
use kernel::mem as kmem;
use kernel::raw;
//use kernel::time::*;

use bento::bentofs::*;
use bento::bindings::*;
//use bento::println;
use bento::DataBlock;

use crate::log::*;
use crate::xv6fs_file::*;
use crate::xv6fs_fs::*;
use crate::xv6fs_utils::*;

pub const XV6FS_LL_OPS: fs_ops = fs_ops {
    init: xv6fs_ll_init_rs,
    destroy: xv6fs_ll_destroy,
    getattr: xv6fs_ll_getattr_rs,
    lookup: xv6fs_ll_lookup_rs,
    readdir: xv6fs_ll_readdir_rs,
    open: xv6fs_ll_open_rs,
    opendir: xv6fs_ll_opendir_rs,
    read: xv6fs_ll_read_rs,
    statfs: xv6fs_ll_statfs_rs,
    write: xv6fs_ll_write_rs,
    create: xv6fs_ll_create_rs,
    setattr: xv6fs_ll_setattr_rs,
    mkdir: xv6fs_ll_mkdir_rs,
    rmdir: xv6fs_ll_rmdir_rs,
    unlink: xv6fs_ll_unlink_rs,
    lseek: xv6fs_ll_lseek_rs,
    symlink: xv6fs_ll_symlink_rs,
    readlink: xv6fs_ll_readlink_rs,

    ioctl: xv6fs_ll_ioctl_rs,
    fsync: xv6fs_ll_fsync_rs,
    fsyncdir: xv6fs_ll_fsyncdir_rs,

    flush: xv6fs_ll_flush_rs,
    getxattr: xv6fs_ll_getxattr_rs,
    listxattr: xv6fs_ll_listxattr_rs,
    access: xv6fs_ll_access_rs,
    mknod: xv6fs_ll_mknod_rs,
    forget: xv6fs_ll_forget_rs,
    getlk: xv6fs_ll_getlk,
    setlk: xv6fs_ll_setlk,
    bmap: xv6fs_ll_bmap,
    poll: xv6fs_ll_poll,
    fallocate: xv6fs_ll_fallocate,
    setxattr: xv6fs_ll_setxattr,
    removexattr: xv6fs_ll_removexattr,
    rename: xv6fs_ll_rename,
    release: xv6fs_ll_release,
    releasedir: xv6fs_ll_releasedir,
};

#[no_mangle]
pub fn get_fs_ops_rs() -> &'static fs_ops {
    return &XV6FS_LL_OPS;
}

#[no_mangle]
pub fn xv6fs_ll_flush_rs(_sb: RsSuperBlock, _nodeid: u64, _inarg: &fuse_flush_in) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn xv6fs_ll_getxattr_rs(
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
pub fn xv6fs_ll_listxattr_rs(
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
pub fn xv6fs_ll_statfs_rs(_sb: RsSuperBlock, _nodeid: u64, outarg: &mut fuse_statfs_out) -> i32 {
    // Read super_block from disk
    let fs_size = SB.read().size;
    outarg.st.blocks = fs_size as u64;
    outarg.st.bsize = BSIZE as u32;
    outarg.st.namelen = DIRSIZ as u32;
    return 0;
}

#[no_mangle]
pub fn xv6fs_ll_open_rs(
    sb: RsSuperBlock,
    nodeid: u64,
    inarg: &fuse_open_in,
    outarg: &mut fuse_open_out,
) -> i32 {
    let inode = match iget(&sb, nodeid) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };

    let icache = ILOCK_CACHE.read();
    let inode_guard = match ilock(&sb, inode.idx, &icache, inode.inum) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };
    let mut internals = inode_guard.internals.write();

    // Check if inode is a file
    if internals.inode_type != T_FILE {
        return -(EISDIR as i32);
    }

    if inarg.flags & O_TRUNC != 0 {
        let _guard = begin_op(&sb);
        internals.size = 0;
        if let Err(x) = iupdate(&sb, &internals, inode.inum) {
            return x as i32;
        }
    }

    outarg.fh = 0;
    outarg.open_flags = FOPEN_KEEP_CACHE;
    return 0;
}

#[no_mangle]
pub fn xv6fs_ll_opendir_rs(
    sb: RsSuperBlock,
    nodeid: u64,
    _inarg: &fuse_open_in,
    outarg: &mut fuse_open_out,
) -> i32 {
    let inode = match iget(&sb, nodeid) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };

    let icache = ILOCK_CACHE.read();
    let inode_guard = match ilock(&sb, inode.idx, &icache, inode.inum) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };
    let internals = inode_guard.internals.write();

    if internals.inode_type != T_DIR {
        return -(ENOTDIR as i32);
    } else {
        outarg.fh = 0;
        outarg.open_flags = 0;
        return 0;
    }
}

#[no_mangle]
pub fn xv6fs_ll_getattr_rs(
    sb: RsSuperBlock,
    nodeid: u64,
    _inarg: &fuse_getattr_in,
    outarg: &mut fuse_attr_out,
) -> i32 {
    let inode = match iget(&sb, nodeid) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };

    let icache = ILOCK_CACHE.read();
    let inode_guard = match ilock(&sb, inode.idx, &icache, inode.inum) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };
    let internals = inode_guard.internals.read();
    outarg.attr_valid = 1;
    outarg.attr_valid_nsec = 999999999;
    return match stati(nodeid, &mut outarg.attr, &internals) {
        Ok(()) => 0,
        Err(x) => x as i32,
    };
}

#[no_mangle]
pub fn xv6fs_ll_setattr_rs(
    sb: RsSuperBlock,
    nodeid: u64,
    _inarg: &fuse_setattr_in,
    outarg: &mut fuse_attr_out,
) -> i32 {
    let _guard = begin_op(&sb);
    let inode = match iget(&sb, nodeid) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };

    let icache = ILOCK_CACHE.read();
    let inode_guard = match ilock(&sb, inode.idx, &icache, inode.inum) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };
    let internals = inode_guard.internals.read();
    outarg.attr_valid = 1;
    outarg.attr_valid_nsec = 999999999;
    match stati(nodeid, &mut outarg.attr, &internals) {
        Ok(()) => return 0,
        Err(x) => return x as i32,
    }
}

#[no_mangle]
pub fn xv6fs_ll_lookup_rs(
    sb: RsSuperBlock,
    nodeid: u64,
    name: CStr,
    outarg: &mut fuse_entry_out,
) -> i32 {
    // Get inode number from nodeid
    let inode = match iget(&sb, nodeid) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };
    let icache = ILOCK_CACHE.read();
    let inode_guard = match ilock(&sb, inode.idx, &icache, inode.inum) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };
    let mut internals = inode_guard.internals.write();
    let mut poff = 0;
    let child = match dirlookup(&sb, &mut internals, &name, &mut poff) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };

    outarg.nodeid = child.inum as u64;
    outarg.generation = 0;
    outarg.attr_valid = 1;
    outarg.entry_valid = 1;
    outarg.attr_valid_nsec = 999999999;
    outarg.entry_valid_nsec = 999999999;

    let child_inode_guard = match ilock(&sb, child.idx, &icache, child.inum) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };
    let child_internals = child_inode_guard.internals.read();
    return match stati(outarg.nodeid, &mut outarg.attr, &child_internals) {
        Ok(()) => 0,
        Err(x) => x as i32,
    };
}

#[no_mangle]
pub fn xv6fs_ll_read_rs(
    sb: RsSuperBlock,
    nodeid: u64,
    inarg: &fuse_read_in,
    buf: &mut kmem::MemContainer<u8>,
) -> i32 {
    // Get inode number nodeid
    let inode = match iget(&sb, nodeid) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };

    let icache = ILOCK_CACHE.read();
    let inode_guard = match ilock(&sb, inode.idx, &icache, inode.inum) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };
    let mut internals = inode_guard.internals.write();

    // Check if inode is a file
    if internals.inode_type != T_FILE {
        return -(EISDIR as i32);
    }

    let off = inarg.offset as usize;
    let n = buf.len();

    let buf_slice = buf.to_slice_mut();
    let read_rs = match readi(&sb, buf_slice, off, n, &mut internals) {
        Ok(x) => x as i32,
        Err(_) => -1,
    };
    return read_rs;
}

#[no_mangle]
pub fn xv6fs_ll_write_rs(
    sb: RsSuperBlock,
    nodeid: u64,
    inarg: &fuse_write_in,
    buf: &kmem::MemContainer<u8>,
    outarg: &mut fuse_write_out,
) -> i32 {
    // Get the inode at nodeid
    let max = ((MAXOPBLOCKS - 1 - 1 - 2) / 2) * BSIZE;
    let mut i = 0;
    let n = inarg.size as usize;
    let mut off = inarg.offset as usize;
    let mut file_off = 0;
    while i < n {
        let _guard = begin_op(&sb);
        let inode = match iget(&sb, nodeid) {
            Ok(x) => x,
            Err(x) => return x as i32,
        };

        let icache = ILOCK_CACHE.read();
        let inode_guard = match ilock(&sb, inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => return x as i32,
        };
        let mut internals = inode_guard.internals.write();

        // Check if inode is a file
        if internals.inode_type != T_FILE {
            return -(EISDIR as i32);
        }

        let mut n1 = n - i;
        if n1 > max {
            n1 = max;
        }
        let data_slice = buf.to_slice();
        let data_region = &data_slice[file_off..];
        let r = match writei(&sb, data_region, off, n1, &mut internals, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                return x as i32;
            }
        };

        off += r;
        file_off += r;
        i += r;
    }
    outarg.size = inarg.size;
    return 0;
}

#[no_mangle]
pub fn xv6fs_ll_readdir_rs(
    sb: RsSuperBlock,
    nodeid: u64,
    inarg: &fuse_read_in,
    buf: &mut kmem::MemContainer<u8>,
    size: &mut usize,
) -> i32 {
    // Get inode number nodeid
    let inode = match iget(&sb, nodeid) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };

    let icache = ILOCK_CACHE.read();
    let inode_guard = match ilock(&sb, inode.idx, &icache, inode.inum) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };
    let mut internals = inode_guard.internals.write();

    // Check if inode is directory
    if internals.inode_type != T_DIR {
        return -(ENOTDIR as i32);
    }

    if let Err(x) = kmem::memset_rust(buf, 0, buf.len() as u64) {
        return x as i32;
    }

    let mut de_cont = match kmem::MemContainer::<u8>::alloc(mem::size_of::<Xv6fsDirent>()) {
        Some(x) => x,
        None => return -1,
    };

    let mut buf_off = 0;
    let mut inarg_offset = inarg.offset as usize;
    for off in (0..internals.size).step_by(mem::size_of::<Xv6fsDirent>()) {
        let buf_len = de_cont.len();
        let de_slice = de_cont.to_slice_mut();
        match readi(&sb, de_slice, off as usize, buf_len, &mut internals) {
            Ok(x) if x != buf_len => return -1,
            Err(x) => return x as i32,
            _ => {}
        };
        let mut de = Xv6fsDirent::new();
        if de.extract_from(de_slice).is_err() {
            return -(EIO as i32);
        }



        let buf_slice = buf.to_slice_mut();
        let curr_buf_slice = &mut buf_slice[buf_off..];
        let name_str = match str::from_utf8(&de.name) {
            Ok(x) => x,
            Err(_) => "",
        };
        let ent_len = match bento_add_direntry(
            curr_buf_slice,
            name_str,
            de.inum as u64,
            0,
            buf_off as u64 + inarg.offset,
        ) {
            Ok(x) => x,
            Err(errno::Error::EOVERFLOW) => break,
            Err(x) => return x as i32,
        };
        if ent_len <= inarg_offset {
            inarg_offset -= ent_len;
        } else {
            buf_off += ent_len;
        }
    }
    *size = buf_off;
    return 0;
}

pub fn create<'a>(
    sb: &'a RsSuperBlock,
    nodeid: u64,
    itype: u16,
    name: &CStr,
) -> Result<CachedInode<'a>, errno::Error> {
    // Get inode for parent directory

    let parent = iget(sb, nodeid)?;
    let icache = ILOCK_CACHE.read();
    // Get inode for new file
    let parent_inode_guard = ilock(sb, parent.idx, &icache, parent.inum)?;
    let mut parent_internals = parent_inode_guard.internals.write();

    let inode = ialloc(sb, itype)?;
    if (parent_internals.size as usize + mem::size_of::<Xv6fsDirent>()) > (MAXFILE as usize * BSIZE)
    {
        return Err(errno::Error::EIO);
    }

    let inode_guard = ilock(sb, inode.idx, &icache, inode.inum)?;
    let mut internals = inode_guard.internals.write();

    internals.major = parent_internals.major;
    internals.minor = parent_internals.minor;
    internals.nlink = 1;

    iupdate(sb, &internals, inode.inum)?;

    if itype == T_DIR {
        parent_internals.nlink += 1;
        iupdate(sb, &parent_internals, parent.inum)?;
        let d_bytes = &['.' as u8, '\0' as u8];
        let d = CStr::from_bytes_with_nul(d_bytes)?;
        dirlink(sb, &mut internals, &d, inode.inum, inode.inum)?;

        let dd_bytes = &['.' as u8, '.' as u8, '\0' as u8];
        let dd = CStr::from_bytes_with_nul(dd_bytes)?;
        dirlink(sb, &mut internals, &dd, nodeid as u32, inode.inum)?;
    }

    dirlink(sb, &mut parent_internals, name, inode.inum, parent.inum)?;
    return Ok(inode);
}

#[no_mangle]
pub fn xv6fs_ll_create_rs(
    sb: RsSuperBlock,
    nodeid: u64,
    _inarg: &fuse_create_in,
    name: CStr,
    outentry: &mut fuse_entry_out,
    outopen: &mut fuse_open_out,
) -> i32 {
    // Check if the file already exists
    let _guard = begin_op(&sb);
    let child = match create(&sb, nodeid, T_FILE, &name) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };

    let icache = ILOCK_CACHE.read();
    let inode_guard = match ilock(&sb, child.idx, &icache, child.inum) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };
    let internals = inode_guard.internals.read();

    outopen.fh = 0;
    outopen.open_flags = FOPEN_KEEP_CACHE;
    outentry.nodeid = child.inum as u64;
    outentry.generation = 0;
    outentry.attr_valid = 1;
    outentry.entry_valid = 1;
    outentry.attr_valid_nsec = 999999999;
    outentry.entry_valid_nsec = 999999999;
    match stati(outentry.nodeid, &mut outentry.attr, &internals) {
        Ok(()) => return 0,
        Err(x) => return x as i32,
    }
}

#[no_mangle]
pub fn xv6fs_ll_init_rs(sb: RsSuperBlock, inarg: &fuse_init_in, outarg: &mut fuse_init_out) -> i32 {
    outarg.major = BENTO_KERNEL_VERSION;
    outarg.minor = BENTO_KERNEL_MINOR_VERSION;
    outarg.flags = 0;

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

    iinit(&sb);

    outarg.flags |= FUSE_BIG_WRITES;
    outarg.flags |= FUSE_ATOMIC_O_TRUNC;
    outarg.flags |= FUSE_WRITEBACK_CACHE;

    outarg.max_readahead = max_readahead;
    outarg.max_write = max_write;
    outarg.max_background = 0;
    outarg.congestion_threshold = 0;
    outarg.time_gran = 1;

    return 0;
}

#[no_mangle]
pub fn xv6fs_ll_destroy(_sb: RsSuperBlock) -> i32 {
    return 0;
}

#[no_mangle]
pub fn xv6fs_ll_access_rs(_sb: RsSuperBlock, _nodeid: u64, _inarg: &fuse_access_in) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn xv6fs_ll_mknod_rs(
    _sb: RsSuperBlock,
    _nodeid: u64,
    _inarg: &fuse_mknod_in,
    _name: CStr,
    _outarg: &mut fuse_entry_out,
) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn xv6fs_ll_mkdir_rs(
    sb: RsSuperBlock,
    nodeid: u64,
    _inarg: &fuse_mkdir_in,
    name: CStr,
    outarg: &mut fuse_entry_out,
) -> i32 {
    let _guard = begin_op(&sb);
    let child = match create(&sb, nodeid, T_DIR, &name) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };

    let icache = ILOCK_CACHE.read();
    let inode_guard = match ilock(&sb, child.idx, &icache, child.inum) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };
    let internals = inode_guard.internals.read();

    outarg.nodeid = child.inum as u64;
    outarg.generation = 0;
    outarg.attr_valid = 1;
    outarg.entry_valid = 1;
    outarg.attr_valid_nsec = 999999999;
    outarg.entry_valid_nsec = 999999999;
    match stati(outarg.nodeid, &mut outarg.attr, &internals) {
        Ok(()) => return 0,
        Err(x) => return x as i32,
    }
}

fn isdirempty(sb: &RsSuperBlock, internals: &mut InodeInternal) -> Result<bool, errno::Error> {
    let mut de_cont =
        kmem::MemContainer::<u8>::alloc(mem::size_of::<Xv6fsDirent>()).ok_or(errno::Error::EIO)?;

    let step_size = mem::size_of::<Xv6fsDirent>();
    for off in (2 * step_size..internals.size as usize).step_by(step_size) {
        let buf_len = de_cont.len();
        let de_slice = de_cont.to_slice_mut();
        match readi(sb, de_slice, off as usize, buf_len, internals) {
            Ok(x) if x != buf_len => return Err(errno::Error::EIO),
            Err(x) => return Err(x),
            _ => {}
        };
        let mut de = Xv6fsDirent::new();
        de.extract_from(de_slice).map_err(|_| errno::Error::EIO)?;

        if de.inum != 0 {
            return Ok(false);
        }
    }
    return Ok(true);
}

fn dounlink(sb: &RsSuperBlock, nodeid: u64, name: &CStr) -> Result<usize, errno::Error> {
    let parent = iget(sb, nodeid)?;
    let icache = ILOCK_CACHE.read();
    let parent_inode_guard = ilock(sb, parent.idx, &icache, parent.inum)?;
    let mut parent_internals = parent_inode_guard.internals.write();
    let mut poff = 0;
    if namecmp(name, ".") == 0 || namecmp(name, "..") == 0 {
        return Err(errno::Error::EIO);
    }
    let inode = dirlookup(sb, &mut parent_internals, name, &mut poff)?;

    let inode_guard = ilock(sb, inode.idx, &icache, inode.inum)?;
    let mut inode_internals = inode_guard.internals.write();

    if inode_internals.nlink < 1 {
        return Err(errno::Error::EIO);
    }

    if inode_internals.inode_type == T_DIR {
        match isdirempty(sb, &mut inode_internals) {
            Ok(true) => {}
            Ok(false) => {
                return Err(errno::Error::ENOTEMPTY);
            },
            _ => {
                return Err(errno::Error::EIO);
            }
        }
    }

    let de_arr = [0; mem::size_of::<Xv6fsDirent>()];
    let buf_len = mem::size_of::<Xv6fsDirent>();
    let r = writei(
        sb,
        &de_arr,
        poff as usize,
        buf_len,
        &mut parent_internals,
        parent.inum,
    )?;

    if r != buf_len {
        return Err(errno::Error::EIO);
    }

    if inode_internals.inode_type == T_DIR {
        parent_internals.nlink -= 1;
        iupdate(sb, &parent_internals, parent.inum)?;
    }

    inode_internals.nlink -= 1;
    iupdate(sb, &inode_internals, inode.inum)?;

    return Ok(0);
}

#[no_mangle]
pub fn xv6fs_ll_rmdir_rs(sb: RsSuperBlock, nodeid: u64, name: CStr) -> i32 {
    let _guard = begin_op(&sb);
    match dounlink(&sb, nodeid, &name) {
        Ok(x) => return x as i32,
        Err(x) => return x as i32,
    }
}

#[no_mangle]
pub fn xv6fs_ll_unlink_rs(sb: RsSuperBlock, nodeid: u64, name: CStr) -> i32 {
    let _guard = begin_op(&sb);
    match dounlink(&sb, nodeid, &name) {
        Ok(x) => return x as i32,
        Err(x) => return x as i32,
    }
}

#[no_mangle]
pub fn xv6fs_ll_lseek_rs(
    _sb: RsSuperBlock,
    _nodeid: u64,
    inarg: &fuse_lseek_in,
    outarg: &mut fuse_lseek_out,
) -> i32 {
    outarg.offset = inarg.offset;
    return 0;
}

#[no_mangle]
pub fn xv6fs_ll_ioctl_rs(
    _sb: RsSuperBlock,
    _nodied: u64,
    _inarg: &fuse_ioctl_in,
    _outarg: &mut fuse_ioctl_out,
    _buf: &mut kmem::MemContainer<u8>,
) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn xv6fs_ll_fsync_rs(sb: RsSuperBlock, _nodeid: u64, _inarg: &fuse_fsync_in) -> i32 {
    let mut error_sector = 0;
    return blkdev_issue_flush_rust(&sb.s_bdev(), GFP_KERNEL as usize, &mut error_sector) as i32;
}

#[no_mangle]
pub fn xv6fs_ll_fsyncdir_rs(_sb: RsSuperBlock, _nodied: u64, _inarg: &fuse_fsync_in) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn xv6fs_ll_symlink_rs(
    sb: RsSuperBlock,
    nodeid: u64,
    name: CStr,
    linkname: CStr,
    outarg: &mut fuse_entry_out,
) -> i32 {
    let _guard = begin_op(&sb);
    // Create new file
    let child = match create(&sb, nodeid, T_LNK, &name) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };

    let icache = ILOCK_CACHE.read();
    let inode_guard = match ilock(&sb, child.idx, &icache, child.inum) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };
    let mut internals = inode_guard.internals.write();

    let mut len_slice = [0; mem::size_of::<u32>()];
    let str_length = linkname.len() + 1;
    let strlen_slice = str_length.to_ne_bytes();
    len_slice.copy_from_slice(&strlen_slice);
    if let Err(x) = writei(
        &sb,
        &len_slice,
        0,
        mem::size_of::<u32>(),
        &mut internals,
        child.inum,
    ) {
        return x as i32;
    };

    // Write linkname to file
    let mut name_buf = match kmem::MemContainer::<raw::c_uchar>::alloc(linkname.len()) {
        Some(x) => x,
        None => return -1,
    };
    let name_slice = name_buf.to_slice_mut();
    name_slice.copy_from_slice(linkname.to_bytes_with_nul());
    if let Err(x) = writei(
        &sb,
        name_slice,
        mem::size_of::<u32>(),
        linkname.len(),
        &mut internals,
        child.inum,
    ) {
        return x as i32;
    };
    outarg.nodeid = child.inum as u64;
    outarg.generation = 0;
    outarg.attr_valid = 1;
    outarg.entry_valid = 1;
    outarg.attr_valid_nsec = 999999999;
    outarg.entry_valid_nsec = 999999999;
    match stati(outarg.nodeid, &mut outarg.attr, &internals) {
        Ok(()) => return 0,
        Err(x) => return x as i32,
    }
}

#[no_mangle]
pub fn xv6fs_ll_readlink_rs(
    sb: RsSuperBlock,
    nodeid: u64,
    buf: &mut kmem::MemContainer<u8>,
) -> i32 {
    let inode = match iget(&sb, nodeid) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };

    let icache = ILOCK_CACHE.read();
    let inode_guard = match ilock(&sb, inode.idx, &icache, inode.inum) {
        Ok(x) => x,
        Err(x) => return x as i32,
    };
    let mut internals = inode_guard.internals.write();

    // Check if inode is a file
    if internals.inode_type != T_LNK {
        return -1;
    }

    let mut len_slice = [0; 4];

    match readi(
        &sb,
        &mut len_slice,
        0,
        mem::size_of::<u32>(),
        &mut internals,
    ) {
        Ok(x) if x != mem::size_of::<usize>() => return -1,
        Err(_) => return -1,
        _ => {}
    }
    let mut str_len_bytes = [0; 4];
    str_len_bytes.copy_from_slice(&len_slice);
    let str_len = u32::from_ne_bytes(str_len_bytes);

    if buf.len() < str_len as usize {
        return -1;
    }

    let buf_slice = buf.to_slice_mut();

    let r = match readi(
        &sb,
        buf_slice,
        mem::size_of::<usize>(),
        str_len as usize,
        &mut internals,
    ) {
        Ok(x) => x,
        Err(_) => return -1,
    };
    return r as i32;
}

#[no_mangle]
pub fn xv6fs_ll_forget_rs(_sb: RsSuperBlock, _nodeid: u64, _nlookup: u64) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn xv6fs_ll_getlk(
    _sb: RsSuperBlock,
    _nodeid: u64,
    _inarg: &fuse_lk_in,
    _outarg: &mut fuse_lk_out,
) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn xv6fs_ll_setlk(_sb: RsSuperBlock, _nodeid: u64, _inarg: &fuse_lk_in, _sleep: bool) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn xv6fs_ll_bmap(
    _sb: RsSuperBlock,
    _nodeid: u64,
    _inarg: &fuse_bmap_in,
    _outarg: &mut fuse_bmap_out,
) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn xv6fs_ll_poll(
    _sb: RsSuperBlock,
    _nodeid: u64,
    _inarg: &fuse_poll_in,
    _outarg: &mut fuse_poll_out,
) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn xv6fs_ll_fallocate(_sb: RsSuperBlock, _nodeid: u64, _inarg: &fuse_fallocate_in) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn xv6fs_ll_setxattr(
    _sb: RsSuperBlock,
    _nodeid: u64,
    _inarg: &fuse_setxattr_in,
    _name: CStr,
    _value: &kmem::MemContainer<raw::c_uchar>,
) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn xv6fs_ll_removexattr(_sb: RsSuperBlock, _nodeid: u64, _name: CStr) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn xv6fs_ll_rename(
    _sb: RsSuperBlock,
    _nodeid: u64,
    _inarg: &fuse_rename2_in,
    _oldname: CStr,
    _newname: CStr,
) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn xv6fs_ll_release(_sb: RsSuperBlock, _nodeid: u64, _inarg: &fuse_release_in) -> i32 {
    return -(ENOSYS as i32);
}

#[no_mangle]
pub fn xv6fs_ll_releasedir(_sb: RsSuperBlock, _nodeid: u64, _inarg: &fuse_release_in) -> i32 {
    return -(ENOSYS as i32);
}
