use alloc::vec::Vec;

use core::mem;
use core::str;

use bento::kernel;
use kernel::errno;
use kernel::fs::Disk;
use kernel::fuse::*;
use kernel::kobj::*;
use kernel::time::*;
//use kernel::time::*;

use bento::bindings::*;
use bento::c_str;
//use bento::println;
use bento::fuse::reply::*;
use bento::fuse::request::*;
use bento::fuse::*;
use bento::DataBlock;

use crate::log::*;
use crate::xv6fs_file::*;
use crate::xv6fs_fs::*;
use crate::xv6fs_utils::*;

pub fn create_internal(
    nodeid: u64,
    itype: u16,
    name: &CStr,
) -> Result<CachedInode, errno::Error> {
    // Get inode for parent directory

    let parent = iget(nodeid)?;
    let icache = ILOCK_CACHE.read();
    // Get inode for new file
    let parent_inode_guard = ilock(parent.idx, &icache, parent.inum)?;
    let mut parent_internals = parent_inode_guard.internals.write();

    let inode = ialloc(itype)?;
    if (parent_internals.size as usize + mem::size_of::<Xv6fsDirent>()) > (MAXFILE as usize * BSIZE)
    {
        return Err(errno::Error::EIO);
    }

    let inode_guard = ilock(inode.idx, &icache, inode.inum)?;
    let mut internals = inode_guard.internals.write();

    internals.major = parent_internals.major;
    internals.minor = parent_internals.minor;
    internals.nlink = 1;

    iupdate(&internals, inode.inum)?;

    if itype == T_DIR {
        parent_internals.nlink += 1;
        iupdate(&parent_internals, parent.inum)?;
        let d_bytes = &['.' as u8, '\0' as u8];
        let d = CStr::from_bytes_with_nul(d_bytes)?;
        dirlink(&mut internals, &d, inode.inum, inode.inum)?;

        let dd_bytes = &['.' as u8, '.' as u8, '\0' as u8];
        let dd = CStr::from_bytes_with_nul(dd_bytes)?;
        dirlink(&mut internals, &dd, nodeid as u32, inode.inum)?;
    }

    dirlink(&mut parent_internals, name, inode.inum, parent.inum)?;
    return Ok(inode);
}

fn isdirempty(internals: &mut InodeInternal) -> Result<bool, errno::Error> {
    let de_len = mem::size_of::<Xv6fsDirent>();
    let mut de_vec: Vec<u8> = vec![0; de_len];
    for off in (2 * de_len..internals.size as usize).step_by(de_len) {
        let de_slice = de_vec.as_mut_slice();
        match readi(de_slice, off as usize, de_len, internals) {
            Ok(x) if x != de_len => return Err(errno::Error::EIO),
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

fn dounlink(nodeid: u64, name: &CStr) -> Result<usize, errno::Error> {
    let parent = iget(nodeid)?;
    let icache = ILOCK_CACHE.read();
    let parent_inode_guard = ilock(parent.idx, &icache, parent.inum)?;
    let mut parent_internals = parent_inode_guard.internals.write();
    let mut poff = 0;
    if namecmp(name, ".") == 0 || namecmp(name, "..") == 0 {
        return Err(errno::Error::EIO);
    }
    let inode = dirlookup(&mut parent_internals, name, &mut poff)?;

    let inode_guard = ilock(inode.idx, &icache, inode.inum)?;
    let mut inode_internals = inode_guard.internals.write();

    if inode_internals.nlink < 1 {
        return Err(errno::Error::EIO);
    }

    if inode_internals.inode_type == T_DIR {
        match isdirempty(&mut inode_internals) {
            Ok(true) => {}
            _ => {
                return Err(errno::Error::ENOTEMPTY);
            }
        }
    }

    let de_arr = [0; mem::size_of::<Xv6fsDirent>()];
    let buf_len = mem::size_of::<Xv6fsDirent>();
    let r = writei(
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
        iupdate(&parent_internals, parent.inum)?;
    }

    inode_internals.nlink -= 1;
    iupdate(&inode_internals, inode.inum)?;

    return Ok(0);
}

pub static XV6FS: Xv6FileSystem = Xv6FileSystem {};

pub struct Xv6FileSystem {}

impl Xv6FileSystem {
    const NAME: &'static str = c_str!("xv6fs_ll");
}

impl Filesystem for Xv6FileSystem {
    fn get_name(&self) -> &str {
        Xv6FileSystem::NAME
    }

    fn init(
        &mut self,
        _req: &Request,
        devname: &CStr,
        fc_info: &mut FuseConnInfo,
    ) -> Result<(), i32> {
        fc_info.proto_major = BENTO_KERNEL_VERSION;
        fc_info.proto_minor = BENTO_KERNEL_MINOR_VERSION;
        fc_info.want = 0;

        let mut bufsize = FUSE_MAX_MAX_PAGES * PAGE_SIZE as u32 + FUSE_BUFFER_HEADER_SIZE;
        let mut max_write = u32::MAX;
        let mut max_readahead = u32::MAX;

        if bufsize < FUSE_MIN_READ_BUFFER {
            bufsize = FUSE_MIN_READ_BUFFER;
        }

        if max_write > bufsize - FUSE_BUFFER_HEADER_SIZE {
            max_write = bufsize - FUSE_BUFFER_HEADER_SIZE;
        }

        if fc_info.max_readahead < max_readahead {
            max_readahead = fc_info.max_readahead;
        }
        let devname_str = str::from_utf8(devname.to_bytes_with_nul()).unwrap();
        let mut mut_disk = DISK.write();
        *mut_disk = Some(Disk::new(devname_str, BSIZE as u64));

        iinit();

        fc_info.want |= FUSE_BIG_WRITES;
        fc_info.want |= FUSE_ATOMIC_O_TRUNC;
        fc_info.want |= FUSE_WRITEBACK_CACHE;

        fc_info.max_readahead = max_readahead;
        fc_info.max_write = max_write;
        fc_info.max_background = 0;
        fc_info.congestion_threshold = 0;
        fc_info.time_gran = 1;

        return Ok(());
    }

    fn statfs(&mut self, _req: &Request, _ino: u64, reply: ReplyStatfs) {
        let fs_size = SB.read().size;
        reply.statfs(fs_size as u64, 0, 0, 0, 0, BSIZE as u32, DIRSIZ as u32, 0);
    }

    fn open(
        &mut self,
        _req: &Request,
        nodeid: u64,
        flags: u32,
        reply: ReplyOpen,
    ) {
        let inode = match iget(nodeid) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };

        let icache = ILOCK_CACHE.read();
        let inode_guard = match ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };
        let mut internals = inode_guard.internals.write();

        // Check if inode is a file
        if internals.inode_type != T_FILE {
            reply.error(-(EISDIR as i32));
            return;
        }

        if flags & O_TRUNC != 0 {
            let _guard = begin_op();
            internals.size = 0;
            if let Err(x) = iupdate(&internals, inode.inum) {
                reply.error(x as i32);
                return;
            }
        }

        let fh = 0;
        let open_flags = FOPEN_KEEP_CACHE;
        reply.opened(fh, open_flags);
    }

    fn opendir(
        &mut self,
        _req: &Request,
        nodeid: u64,
        _flags: u32,
        reply: ReplyOpen,
    ) {
        let inode = match iget(nodeid) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };

        let icache = ILOCK_CACHE.read();
        let inode_guard = match ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };
        let internals = inode_guard.internals.write();

        if internals.inode_type != T_DIR {
            reply.error(-(ENOTDIR as i32));
        } else {
            let fh = 0;
            let open_flags = 0;
            reply.opened(fh, open_flags);
        }
    }

    fn getattr(&mut self, _req: &Request, nodeid: u64, reply: ReplyAttr) {
        let inode = match iget(nodeid) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };

        let icache = ILOCK_CACHE.read();
        let inode_guard = match ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };
        let internals = inode_guard.internals.read();
        let attr_valid = Timespec::new(1, 999999999);
        let mut attr = fuse_attr::new();
        match stati(nodeid, &mut attr, &internals) {
            Ok(()) => {
                reply.attr(&attr_valid, &attr);
            }
            Err(x) => {
                reply.error(x as i32);
            }
        };
    }

    fn setattr(
        &mut self,
        _req: &Request,
        ino: u64,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        _size: Option<u64>,
        _atime: Option<Timespec>,
        _mtime: Option<Timespec>,
        _fh: Option<u64>,
        _crtime: Option<Timespec>,
        _chgtime: Option<Timespec>,
        _bkuptime: Option<Timespec>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        let _guard = begin_op();
        let inode = match iget(ino) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };

        let icache = ILOCK_CACHE.read();
        let inode_guard = match ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };
        let internals = inode_guard.internals.read();
        let attr_valid = Timespec::new(1, 999999999);
        let mut attr = fuse_attr::new();
        match stati(ino, &mut attr, &internals) {
            Ok(()) => reply.attr(&attr_valid, &attr),
            Err(x) => reply.error(x as i32),
        }
    }

    fn lookup(
        &mut self,
        _req: &Request,
        nodeid: u64,
        name: CStr,
        reply: ReplyEntry,
    ) {
        // Get inode number from nodeid
        let inode = match iget(nodeid) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };
        let icache = ILOCK_CACHE.read();
        let inode_guard = match ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };
        let mut internals = inode_guard.internals.write();
        let mut poff = 0;
        let child = match dirlookup(&mut internals, &name, &mut poff) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };

        let outarg_nodeid = child.inum as u64;
        let outarg_generation = 0;
        let attr_valid = Timespec::new(1, 999999999);

        let child_inode_guard = match ilock(child.idx, &icache, child.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };
        let child_internals = child_inode_guard.internals.read();
        let mut outarg_attr = fuse_attr::new();
        match stati(outarg_nodeid, &mut outarg_attr, &child_internals) {
            Ok(()) => reply.entry(&attr_valid, &outarg_attr, outarg_generation),
            Err(x) => {
                reply.error(x as i32);
            }
        };
    }

    fn read(
        &mut self,
        _req: &Request,
        nodeid: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        reply: ReplyData,
    ) {
        // Get inode number nodeid
        let inode = match iget(nodeid) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };

        let icache = ILOCK_CACHE.read();
        let inode_guard = match ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };
        let mut internals = inode_guard.internals.write();

        // Check if inode is a file
        if internals.inode_type != T_FILE {
            reply.error(-(EISDIR as i32));
            return;
        }

        let off = offset as usize;
        let n = size as usize;

        let mut buf_vec: Vec<u8> = vec![0; n as usize];
        let buf_slice = buf_vec.as_mut_slice();

        let read_rs = match readi(buf_slice, off, n, &mut internals) {
            Ok(x) => x as i32,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };
        reply.data(&buf_slice[0..read_rs as usize]);
    }

    fn write(
        &mut self,
        _req: &Request,
        nodeid: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _flags: u32,
        reply: ReplyWrite,
    ) {
        // Get the inode at nodeid
        let max = ((MAXOPBLOCKS - 1 - 1 - 2) / 2) * BSIZE;
        let mut i = 0;
        let n = data.len();
        let mut off = offset as usize;
        let mut file_off = 0;
        while i < n {
            let _guard = begin_op();
            let inode = match iget(nodeid) {
                Ok(x) => x,
                Err(x) => {
                    reply.error(x as i32);
                    return;
                }
            };

            let icache = ILOCK_CACHE.read();
            let inode_guard = match ilock(inode.idx, &icache, inode.inum) {
                Ok(x) => x,
                Err(x) => {
                    reply.error(x as i32);
                    return;
                }
            };
            let mut internals = inode_guard.internals.write();

            // Check if inode is a file
            if internals.inode_type != T_FILE {
                reply.error(-(EISDIR as i32));
                return;
            }

            let mut n1 = n - i;
            if n1 > max {
                n1 = max;
            }
            let data_region = &data[file_off..];
            let r = match writei(data_region, off, n1, &mut internals, inode.inum) {
                Ok(x) => x,
                Err(x) => {
                    reply.error(x as i32);
                    return;
                }
            };

            off += r;
            file_off += r;
            i += r;
        }
        reply.written(n as u32);
    }

    fn readdir(
        &mut self,
        _req: &Request,
        nodeid: u64,
        _fh: u64,
        offset: i64,
        reply: ReplyDirectory,
    ) {
        // Get inode number nodeid
        let inode = match iget(nodeid) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };

        let icache = ILOCK_CACHE.read();
        let inode_guard = match ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };
        let mut internals = inode_guard.internals.write();

        // Check if inode is directory
        if internals.inode_type != T_DIR {
            reply.error(-(ENOTDIR as i32));
            return;
        }

        let de_len = mem::size_of::<Xv6fsDirent>();
        let mut de_vec: Vec<u8> = vec![0; de_len];

        let mut buf_off = 1;
        let mut inarg_offset = offset as usize;
        for off in (0..internals.size).step_by(de_len) {
            if inarg_offset >= 1 {
                inarg_offset -= 1;
                buf_off += 1;
                continue;
            }
            let de_slice = de_vec.as_mut_slice();
            match readi(de_slice, off as usize, de_len, &mut internals) {
                Ok(x) if x != de_len => {
                    reply.error(-1);
                    return;
                }
                Err(x) => {
                    reply.error(x as i32);
                    return;
                }
                _ => {}
            };
            let mut de = Xv6fsDirent::new();
            if de.extract_from(de_slice).is_err() {
                reply.error(-(EIO as i32));
                return;
            }

            let name_str = match str::from_utf8(&de.name) {
                Ok(x) => x,
                Err(_) => "",
            };
            if reply.add(de.inum as u64, buf_off, 0, name_str) {
                break;
            }
            buf_off += 1;
        }
        reply.ok();
    }

    fn create(
        &mut self,
        _req: &Request,
        parent: u64,
        name: CStr, //&OsStr,
        _mode: u32,
        _flags: u32,
        reply: ReplyCreate,
    ) {
        // Check if the file already exists
        let _guard = begin_op();
        let child = match create_internal(parent, T_FILE, &name) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };

        let icache = ILOCK_CACHE.read();
        let inode_guard = match ilock(child.idx, &icache, child.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };
        let internals = inode_guard.internals.read();

        let fh = 0;
        let open_flags = FOPEN_KEEP_CACHE;
        let nodeid = child.inum as u64;
        let generation = 0;
        let attr_valid = Timespec::new(1, 999999999);
        let mut attr = fuse_attr::new();
        match stati(nodeid, &mut attr, &internals) {
            Ok(()) => {
                reply.created(&attr_valid, &attr, generation, fh, open_flags);
            }
            Err(x) => {
                reply.error(x as i32);
            }
        }
    }

    fn mkdir(
        &mut self,
        _req: &Request,
        parent: u64,
        name: CStr,
        _mode: u32,
        reply: ReplyEntry,
    ) {
        let _guard = begin_op();
        let child = match create_internal(parent, T_DIR, &name) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };

        let icache = ILOCK_CACHE.read();
        let inode_guard = match ilock(child.idx, &icache, child.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };
        let internals = inode_guard.internals.read();

        let out_nodeid = child.inum as u64;
        let generation = 0;
        let attr_valid = Timespec::new(1, 999999999);
        let mut attr = fuse_attr::new();
        match stati(out_nodeid, &mut attr, &internals) {
            Ok(()) => {
                reply.entry(&attr_valid, &attr, generation);
            }
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        }
    }

    fn rmdir(
        &mut self,
        _req: &Request,
        parent: u64,
        name: CStr,
        reply: ReplyEmpty,
    ) {
        let _guard = begin_op();
        match dounlink(parent, &name) {
            Ok(_) => reply.ok(),
            Err(x) => reply.error(x as i32),
        }
    }

    fn unlink(
        &mut self,
        _req: &Request,
        parent: u64,
        name: CStr,
        reply: ReplyEmpty,
    ) {
        let _guard = begin_op();
        match dounlink(parent, &name) {
            Ok(_) => reply.ok(),
            Err(x) => reply.error(x as i32),
        }
    }

    fn fsync(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        force_commit();
        reply.ok();
    }

    fn symlink(
        &mut self,
        _req: &Request,
        nodeid: u64,
        name: CStr,
        linkname: CStr,
        reply: ReplyEntry,
    ) {
        let _guard = begin_op();
        // Create new file
        let child = match create_internal(nodeid, T_LNK, &name) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };

        let icache = ILOCK_CACHE.read();
        let inode_guard = match ilock(child.idx, &icache, child.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };
        let mut internals = inode_guard.internals.write();

        let mut len_slice = [0; mem::size_of::<u32>()];
        let str_length: u32 = linkname.len() as u32 + 1;
        let strlen_slice = str_length.to_ne_bytes();
        len_slice.copy_from_slice(&strlen_slice);
        if let Err(x) = writei(
            &len_slice,
            0,
            mem::size_of::<u32>(),
            &mut internals,
            child.inum,
        ) {
            reply.error(x as i32);
            return;
        };

        if let Err(x) = writei(
            linkname.to_bytes_with_nul(),
            mem::size_of::<u32>(),
            linkname.len(),
            &mut internals,
            child.inum,
        ) {
            reply.error(x as i32);
            return;
        };
        let out_nodeid = child.inum as u64;
        let generation = 0;
        let attr_valid = Timespec::new(1, 999999999);
        let mut attr = fuse_attr::new();
        match stati(out_nodeid, &mut attr, &internals) {
            Ok(()) => reply.entry(&attr_valid, &attr, generation),
            Err(x) => {
                reply.error(x as i32);
            }
        }
    }

    fn readlink(&self, _req: &Request, nodeid: u64, reply: ReplyData) {
        let inode = match iget(nodeid) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };

        let icache = ILOCK_CACHE.read();
        let inode_guard = match ilock(inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };
        let mut internals = inode_guard.internals.write();

        // Check if inode is a file
        if internals.inode_type != T_LNK {
            reply.error(-1);
            return;
        }

        let mut len_slice = [0; 4];

        match readi(
            &mut len_slice,
            0,
            mem::size_of::<u32>(),
            &mut internals,
        ) {
            Ok(x) if x != mem::size_of::<u32>() => {
                reply.error(-(EIO as i32));
                return;
            }
            Err(x) => {
                reply.error(x as i32);
                return;
            }
            _ => {}
        }
        let mut str_len_bytes = [0; 4];
        str_len_bytes.copy_from_slice(&len_slice);
        let str_len = u32::from_ne_bytes(str_len_bytes);

        let mut buf_vec: Vec<u8> = vec![0; str_len as usize];
        let buf_slice = buf_vec.as_mut_slice();

        match readi(
            buf_slice,
            mem::size_of::<u32>(),
            str_len as usize,
            &mut internals,
        ) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            }
        };
        reply.data(buf_slice);
    }
}
