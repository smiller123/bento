use alloc::vec::Vec;

use core::mem;
use core::str;

use bento::kernel;
use kernel::errno;
use kernel::fs::*;
use kernel::fuse::*;
use kernel::kobj::*;
use kernel::mem as kmem;
use kernel::raw;
use kernel::time::*;

use bento::bentofs::*;
use bento::bindings::*;
use bento::c_str;
//use bento::println;
use bento::DataBlock;

use crate::log::*;
use crate::xv6fs_file::*;
use crate::xv6fs_fs::*;
use crate::xv6fs_utils::*;

pub fn create_internal<'a>(
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
            _ => {
                return Err(errno::Error::ENOTEMPTY);
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

pub static XV6FS: Xv6FileSystem = Xv6FileSystem {};

pub struct Xv6FileSystem {}

impl Xv6FileSystem {
    const NAME: &'static str = c_str!("xv6fs_ll");
}

impl FileSystem for Xv6FileSystem {
    fn get_name(&self) -> &str {
        Xv6FileSystem::NAME
    }

    fn init(&mut self, sb: RsSuperBlock, _req: &Request, fc_info: &mut FuseConnInfo) -> Result<(), i32> {
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
    
        iinit(&sb);
    
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

    fn statfs(&self, _sb: RsSuperBlock, _nodeid: u64, outarg: &mut fuse_statfs_out) -> i32 {
        // Read super_block from disk
        let fs_size = SB.read().size;
        outarg.st.blocks = fs_size as u64;
        outarg.st.bsize = BSIZE as u32;
        outarg.st.namelen = DIRSIZ as u32;
        return 0;
    }
   
    fn open(&mut self, sb: RsSuperBlock, _req: &Request, nodeid: u64, flags: u32, reply: ReplyOpen) {
        let inode = match iget(&sb, nodeid) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
        };
    
        let icache = ILOCK_CACHE.read();
        let inode_guard = match ilock(&sb, inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
        };
        let mut internals = inode_guard.internals.write();
    
        // Check if inode is a file
        if internals.inode_type != T_FILE {
            reply.error(-(EISDIR as i32));
            return;
        }
    
        if flags & O_TRUNC != 0 {
            let _guard = begin_op(&sb);
            internals.size = 0;
            if let Err(x) = iupdate(&sb, &internals, inode.inum) {
                reply.error(x as i32);
                return;
            }
        }
    
        let fh = 0;
        let open_flags = FOPEN_KEEP_CACHE;
        reply.opened(fh, open_flags);
    }
    
    fn opendir(&mut self, sb: RsSuperBlock, _req: &Request, nodeid: u64, _flags: u32, reply: ReplyOpen) {
        let inode = match iget(&sb, nodeid) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
        };
    
        let icache = ILOCK_CACHE.read();
        let inode_guard = match ilock(&sb, inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
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

    fn getattr(&mut self, 
        sb: RsSuperBlock,
        _req: &Request,
        nodeid: u64,
        reply: ReplyAttr,
    ) {
        let inode = match iget(&sb, nodeid) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
        };
    
        let icache = ILOCK_CACHE.read();
        let inode_guard = match ilock(&sb, inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
        };
        let internals = inode_guard.internals.read();
        let attr_valid = Timespec::new(1, 999999999);
        let mut attr = fuse_attr::new();
        match stati(nodeid, &mut attr, &internals) {
            Ok(()) => {
                reply.attr(&attr_valid, &attr);
            },
            Err(x) => {
                reply.error(x as i32);
            },
        };
    }
   
    fn setattr(
        &mut self,
        sb: RsSuperBlock,
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
        reply: ReplyAttr
    ) {
        let _guard = begin_op(&sb);
        let inode = match iget(&sb, ino) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
        };
    
        let icache = ILOCK_CACHE.read();
        let inode_guard = match ilock(&sb, inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
        };
        let internals = inode_guard.internals.read();
        let attr_valid = Timespec::new(1, 999999999);
        let mut attr = fuse_attr::new();
        match stati(ino, &mut attr, &internals) {
            Ok(()) => reply.attr(&attr_valid, &attr),
            Err(x) => reply.error(x as i32),
        }
    }

    fn lookup(&mut self, 
        sb: RsSuperBlock,
        _req: &Request,
        nodeid: u64,
        name: CStr,
        reply: ReplyEntry,
    ) {
        // Get inode number from nodeid
        let inode = match iget(&sb, nodeid) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
        };
        let icache = ILOCK_CACHE.read();
        let inode_guard = match ilock(&sb, inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
        };
        let mut internals = inode_guard.internals.write();
        let mut poff = 0;
        let child = match dirlookup(&sb, &mut internals, &name, &mut poff) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
        };
    
        let outarg_nodeid = child.inum as u64;
        let outarg_generation = 0;
        let attr_valid = Timespec::new(1, 999999999);
    
        let child_inode_guard = match ilock(&sb, child.idx, &icache, child.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
        };
        let child_internals = child_inode_guard.internals.read();
        let mut outarg_attr = fuse_attr::new();
        match stati(outarg_nodeid, &mut outarg_attr, &child_internals) {
            Ok(()) => {
                reply.entry(&attr_valid, &outarg_attr, outarg_generation)
            },
            Err(x) => {
                reply.error(x as i32);
            },
        };
    }

    fn read(
        &mut self,
        sb: RsSuperBlock,
        _req: &Request,
        nodeid: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        reply: ReplyData
    ) {
        // Get inode number nodeid
        let inode = match iget(&sb, nodeid) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
        };
    
        let icache = ILOCK_CACHE.read();
        let inode_guard = match ilock(&sb, inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
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

        let read_rs = match readi(&sb, buf_slice, off, n, &mut internals) {
            Ok(x) => x as i32,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
        };
        reply.data(&buf_slice[0..read_rs as usize]);
    }

    fn write(
        &mut self,
        sb: RsSuperBlock,
        _req: &Request,
        nodeid: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _flags: u32,
        reply: ReplyWrite
    ) {
        // Get the inode at nodeid
        let max = ((MAXOPBLOCKS - 1 - 1 - 2) / 2) * BSIZE;
        let mut i = 0;
        let n = data.len();
        let mut off = offset as usize;
        let mut file_off = 0;
        while i < n {
            let _guard = begin_op(&sb);
            let inode = match iget(&sb, nodeid) {
                Ok(x) => x,
                Err(x) => {
                    reply.error(x as i32);
                    return;
                },
            };
    
            let icache = ILOCK_CACHE.read();
            let inode_guard = match ilock(&sb, inode.idx, &icache, inode.inum) {
                Ok(x) => x,
                Err(x) => {
                    reply.error(x as i32);
                    return;
                },
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
            let r = match writei(&sb, data_region, off, n1, &mut internals, inode.inum) {
                Ok(x) => x,
                Err(x) => {
                    reply.error(x as i32);
                    return;
                },
            };
    
            off += r;
            file_off += r;
            i += r;
        }
        reply.written(n as u32);
    }

    fn readdir(&self, 
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

    fn create(&self, 
        sb: RsSuperBlock,
        nodeid: u64,
        _inarg: &fuse_create_in,
        name: CStr,
        outentry: &mut fuse_entry_out,
        outopen: &mut fuse_open_out,
    ) -> i32 {
        // Check if the file already exists
        let _guard = begin_op(&sb);
        let child = match create_internal(&sb, nodeid, T_FILE, &name) {
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

    fn mkdir(&mut self, 
        sb: RsSuperBlock,
        _req: &Request,
        parent: u64,
        name: CStr,
        _mode: u32,
        reply: ReplyEntry
    ) {
        let _guard = begin_op(&sb);
        let child = match create_internal(&sb, parent, T_DIR, &name) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
        };
    
        let icache = ILOCK_CACHE.read();
        let inode_guard = match ilock(&sb, child.idx, &icache, child.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
        };
        let internals = inode_guard.internals.read();
    
        let out_nodeid = child.inum as u64;
        let generation = 0;
        let attr_valid = Timespec::new(1, 999999999);
        let mut attr = fuse_attr::new();
        match stati(out_nodeid, &mut attr, &internals) {
            Ok(()) => {
                reply.entry(&attr_valid, &attr, generation);
            },
            Err(x) => {
                reply.error(x as i32);
                return;
            },
        }
    }

    fn rmdir(&mut self, sb: RsSuperBlock, _req: &Request, parent: u64, name: CStr, reply: ReplyEmpty) {
        let _guard = begin_op(&sb);
        match dounlink(&sb, parent, &name) {
            Ok(_) => reply.ok(),
            Err(x) => reply.error(x as i32),
        }
    }
   
    fn unlink(&mut self, sb: RsSuperBlock, _req: &Request, parent: u64, name: CStr, reply: ReplyEmpty) {
        let _guard = begin_op(&sb);
        match dounlink(&sb, parent, &name) {
            Ok(_) => reply.ok(),
            Err(x) => reply.error(x as i32),
        }
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

    fn fsync(
        &mut self,
        sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty
    ) {
        let mut error_sector = 0;
        blkdev_issue_flush_rust(&sb.s_bdev(), GFP_KERNEL as usize, &mut error_sector);
        reply.ok();
    }

    fn symlink(&mut self, 
        sb: RsSuperBlock,
        _req: &Request,
        nodeid: u64,
        name: CStr,
        linkname: CStr,
        reply: ReplyEntry,
    ) {
        let _guard = begin_op(&sb);
        // Create new file
        let child = match create_internal(&sb, nodeid, T_LNK, &name) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
        };
    
        let icache = ILOCK_CACHE.read();
        let inode_guard = match ilock(&sb, child.idx, &icache, child.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
        };
        let mut internals = inode_guard.internals.write();
    
        let mut len_slice = [0; mem::size_of::<u32>()];
        let str_length: u32 = linkname.len() as u32 + 1;
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
            reply.error(x as i32);
            return;
        };
    
        // Write linkname to file
        let mut name_buf = match kmem::MemContainer::<raw::c_uchar>::alloc(linkname.len()) {
            Some(x) => x,
            None => {
                reply.error(-(EIO as i32));
                return;
            },
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
            reply.error(x as i32);
            return;
        };
        let out_nodeid = child.inum as u64;
        let generation = 0;
        let attr_valid = Timespec::new(1, 999999999);
        let mut attr = fuse_attr::new();
        match stati(out_nodeid, &mut attr, &internals) {
            Ok(()) => {
                reply.entry(&attr_valid, &attr, generation)
            },
            Err(x) => {
                reply.error(x as i32);
            },
        }
    }
    
    fn readlink(&self, 
        sb: RsSuperBlock,
        _req: &Request,
        nodeid: u64,
        reply: ReplyData,
    ) {
        let inode = match iget(&sb, nodeid) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
        };
    
        let icache = ILOCK_CACHE.read();
        let inode_guard = match ilock(&sb, inode.idx, &icache, inode.inum) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
        };
        let mut internals = inode_guard.internals.write();
    
        // Check if inode is a file
        if internals.inode_type != T_LNK {
            reply.error(-1);
            return;
        }
    
        let mut len_slice = [0; 4];
    
        match readi(
            &sb,
            &mut len_slice,
            0,
            mem::size_of::<u32>(),
            &mut internals,
        ) {
            Ok(x) if x != mem::size_of::<u32>() => {
                reply.error(-(EIO as i32));
                return;
            },
            Err(x) => {
                reply.error(x as i32);
                return;
            },
            _ => {}
        }
        let mut str_len_bytes = [0; 4];
        str_len_bytes.copy_from_slice(&len_slice);
        let str_len = u32::from_ne_bytes(str_len_bytes);
    
        let mut buf_vec: Vec<u8> = vec![0; str_len as usize];
        let buf_slice = buf_vec.as_mut_slice();
    
        match readi(
            &sb,
            buf_slice,
            mem::size_of::<u32>(),
            str_len as usize,
            &mut internals,
        ) {
            Ok(x) => x,
            Err(x) => {
                reply.error(x as i32);
                return;
            },
        };
        reply.data(buf_slice);
    }
}
