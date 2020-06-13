use crate::bindings::*;

use kernel::errno;
use kernel::fuse::*;
use kernel::mem::MemContainer;
use kernel::raw;
use kernel::stat;
use kernel::time::Timespec;

/// Add a directory entry to the buffer
///
/// Buffer needs to be large enough to hold the entry. If it's not, then the entry is not filled
/// in and `EOVERFLOW` is returned.
///
/// `off` should be any non-zero value that the filesystem can use to identify the current point in
/// the directory stream. It does not need to be the actual physical position. A value of zero is
/// reserved to mean "from the beginning", and should therefore never be used (the first call to
/// `bento_add_direntry` should be passed the offset of the second directory entry).
pub fn bento_add_direntry(
    buf_slice: &mut [raw::c_uchar],
    name: &str,
    nodeid: u64,
    mode: u16,
    off: u64,
) -> Result<usize, errno::Error> {
    let namelen = name.len();
    let entlen = FUSE_NAME_OFFSET + namelen;
    let entlen_padded = fuse_dirent_align(entlen);

    if entlen_padded > buf_slice.len() {
        return Err(errno::Error::EOVERFLOW);
    }

    let write_region = &mut buf_slice[FUSE_NAME_OFFSET..FUSE_NAME_OFFSET + name.len()];

    write_region.copy_from_slice(name.as_bytes());

    let write_region = &mut buf_slice[entlen..entlen_padded];
    for byte_mut in write_region.iter_mut() {
        *byte_mut = 0;
    }

    let mut dirent = fuse_dirent {
        ino: 0,
        off: 0,
        namelen: 0,
        type_: 0,
        name: __IncompleteArrayField::new(),
    };
    dirent.ino = nodeid;
    dirent.off = off;
    dirent.namelen = namelen as u32;
    dirent.type_ = (mode & stat::S_IFMT) as u32 >> 12;
    let ino_bytes = dirent.ino.to_ne_bytes();
    buf_slice[0..8].copy_from_slice(&ino_bytes);

    let off_bytes = dirent.off.to_ne_bytes();
    buf_slice[8..16].copy_from_slice(&off_bytes);

    let namelen_bytes = dirent.namelen.to_ne_bytes();
    buf_slice[16..20].copy_from_slice(&namelen_bytes);

    let d_type_bytes = dirent.type_.to_ne_bytes();
    buf_slice[20..24].copy_from_slice(&d_type_bytes);

    return Ok(entlen_padded);
}

pub type ReplyEntry<'a, 'b> = &'a mut ReplyEntryInternal<'b>;

#[derive(Debug)]
pub struct ReplyEntryInternal<'a> {
    pub reply: Result<&'a mut fuse_entry_out, i32>,
}

impl<'a> ReplyEntryInternal<'a> {
    pub fn entry(&mut self, ttl: &Timespec, attr: &fuse_attr, generation: u64) {
        if let Ok(rep) = &mut self.reply {
            rep.nodeid = attr.ino;
            rep.generation = generation;
            rep.entry_valid = ttl.sec as u64;
            rep.attr_valid = ttl.sec as u64;
            rep.entry_valid_nsec = ttl.nsec as u32;
            rep.attr_valid_nsec = ttl.nsec as u32;
            rep.attr.ino = attr.ino;
    	    rep.attr.size = attr.size;
	        rep.attr.blocks = attr.blocks;
	        rep.attr.atime = attr.atime;
	        rep.attr.mtime = attr.mtime;
    	    rep.attr.ctime = attr.ctime;
	        rep.attr.atimensec = attr.atimensec;
	        rep.attr.mtimensec = attr.mtimensec;
	        rep.attr.ctimensec = attr.ctimensec;
    	    rep.attr.mode = attr.mode;
	        rep.attr.nlink = attr.nlink;
	        rep.attr.uid = attr.uid;
	        rep.attr.gid = attr.gid;
    	    rep.attr.rdev = attr.rdev;
	        rep.attr.blksize = attr.blksize;
	        rep.attr.padding = attr.padding;
        }
    }

    pub fn error(&mut self, err: i32) {
        self.reply = Err(err);
    }

    pub fn reply(&self) -> &Result<&'a mut fuse_entry_out, i32> {
        return &self.reply;
    }
}

pub type ReplyAttr<'a, 'b> = &'a mut ReplyAttrInternal<'b>;

#[derive(Debug)]
pub struct ReplyAttrInternal<'a> {
    pub reply: Result<&'a mut fuse_attr_out, i32>,
}

impl<'a> ReplyAttrInternal<'a> {
    pub fn attr(&mut self, ttl: &Timespec, attr: &fuse_attr) {
        if let Ok(rep) = &mut self.reply {
            rep.attr_valid = ttl.sec as u64;
            rep.attr_valid_nsec = ttl.nsec as u32;
            rep.dummy = 0;
            rep.attr.ino = attr.ino;
    	    rep.attr.size = attr.size;
	        rep.attr.blocks = attr.blocks;
	        rep.attr.atime = attr.atime;
	        rep.attr.mtime = attr.mtime;
    	    rep.attr.ctime = attr.ctime;
	        rep.attr.atimensec = attr.atimensec;
	        rep.attr.mtimensec = attr.mtimensec;
	        rep.attr.ctimensec = attr.ctimensec;
    	    rep.attr.mode = attr.mode;
	        rep.attr.nlink = attr.nlink;
	        rep.attr.uid = attr.uid;
	        rep.attr.gid = attr.gid;
    	    rep.attr.rdev = attr.rdev;
	        rep.attr.blksize = attr.blksize;
	        rep.attr.padding = attr.padding;
        }
    }

    pub fn error(&mut self, err: i32) {
        self.reply = Err(err);
    }

    pub fn reply(&self) -> &Result<&'a mut fuse_attr_out, i32> {
        return &self.reply;
    }
}

pub type ReplyData<'a, 'b> = &'a mut ReplyDataInternal<'b>;

#[derive(Debug)]
pub struct ReplyDataInternal<'a> {
    pub reply: Result<&'a mut MemContainer<raw::c_uchar>, i32>,
}

impl<'a> ReplyDataInternal<'a> {
    pub fn data(&mut self, data: &[u8]) {
        if let Ok(rep) = &mut self.reply {
            rep.truncate(data.len());
            let rep_slice = rep.to_slice_mut();
            rep_slice.copy_from_slice(data);
        }
    }

    pub fn error(&mut self, err: i32) {
        self.reply = Err(err);
    }

    pub fn reply(&self) -> &Result<&'a mut MemContainer<raw::c_uchar>, i32> {
        return &self.reply;
    }
}

pub type ReplyEmpty<'a> = &'a mut ReplyEmptyInternal;

#[derive(Debug)]
pub struct ReplyEmptyInternal {
    pub reply: Result<(), i32>,
}

impl<'a> ReplyEmptyInternal {
    pub fn ok(&mut self) {
        self.reply = Ok(());
    }

    pub fn error(&mut self, err: i32) {
        self.reply = Err(err);
    }

    pub fn reply(&self) -> &Result<(), i32> {
        return &self.reply;
    }
}

pub type ReplyOpen<'a, 'b> = &'a mut ReplyOpenInternal<'b>;

#[derive(Debug)]
pub struct ReplyOpenInternal<'a> {
    pub reply: Result<&'a mut fuse_open_out, i32>,
}

impl<'a> ReplyOpenInternal<'a> {
    pub fn opened(&mut self, fh: u64, flags: u32) {
        if let Ok(rep) = &mut self.reply {
            rep.fh = fh;
            rep.open_flags = flags;
            rep.padding = 0;
        }
    }

    pub fn error(&mut self, err: i32) {
        self.reply = Err(err);
    }

    pub fn reply(&self) -> &Result<&'a mut fuse_open_out, i32> {
        return &self.reply;
    }
}

pub type ReplyWrite<'a, 'b> = &'a mut ReplyWriteInternal<'b>;

#[derive(Debug)]
pub struct ReplyWriteInternal<'a> {
    pub reply: Result<&'a mut fuse_write_out, i32>,
}

impl<'a> ReplyWriteInternal<'a> {
    pub fn written(&mut self, size: u32) {
        if let Ok(rep) = &mut self.reply {
            rep.size = size;
            rep.padding = 0;
        }
    }

    pub fn error(&mut self, err: i32) {
        self.reply = Err(err);
    }

    pub fn reply(&self) -> &Result<&'a mut fuse_write_out, i32> {
        return &self.reply;
    }
}

pub type ReplyDirectory<'a, 'b> = &'a mut ReplyDirectoryInternal<'b>;

#[derive(Debug)]
pub struct ReplyDirectoryInternal<'a> {
    pub reply: Result<&'a mut MemContainer<raw::c_uchar>, i32>,
    pub length: usize,
}

impl<'a> ReplyDirectoryInternal<'a> {
    pub fn add(&mut self, ino: u64, offset: i64, kind: u16, /*FileType */ name: &str) -> bool {
        if let Ok(rep) = &mut self.reply {
            let buf = rep.to_slice_mut();
            let buf_slice = &mut buf[self.length..];
            return match bento_add_direntry(buf_slice, name, ino, kind, offset as u64) {
                Ok(len) => {
                    self.length += len;
                    false
                },
                Err(errno::Error::EOVERFLOW) => true,
                _ => false,
            }
        }
        return false;
    }

    pub fn ok(&mut self) {
        if let Ok(rep) = &mut self.reply {
            rep.truncate(self.length);
        }
    }

    pub fn error(&mut self, err: i32) {
        self.reply = Err(err);
    }

    pub fn reply(&self) -> &Result<&'a mut MemContainer<raw::c_uchar>, i32> {
        return &self.reply;
    }
}

pub type ReplyStatfs<'a, 'b> = &'a mut ReplyStatfsInternal<'b>;

#[derive(Debug)]
pub struct ReplyStatfsInternal<'a> {
    pub reply: Result<&'a mut fuse_statfs_out, i32>,
}

impl<'a> ReplyStatfsInternal<'a> {
    pub fn statfs(&mut self, blocks: u64, bfree: u64, bavail: u64, files: u64,
                  ffree: u64, bsize: u32, namelen: u32, frsize: u32) {
        if let Ok(rep) = &mut self.reply {
            rep.st.blocks = blocks;
            rep.st.bfree = bfree;
            rep.st.bavail = bavail;
            rep.st.files = files;
            rep.st.ffree = ffree;
            rep.st.bsize = bsize;
            rep.st.namelen = namelen;
            rep.st.frsize = frsize;
        }
    }

    pub fn error(&mut self, err: i32) {
        self.reply = Err(err);
    }

    pub fn reply(&self) -> &Result<&'a mut fuse_statfs_out, i32> {
        return &self.reply;
    }
}

pub type ReplyXattr<'a, 'b> = &'a mut ReplyXattrInternal<'b>;

#[derive(Debug)]
pub struct ReplyXattrInternal<'a> {
    pub reply_arg: Result<&'a mut fuse_getxattr_out, i32>,
    pub reply_buf: Result<&'a mut MemContainer<raw::c_uchar>, i32>,
}

impl<'a> ReplyXattrInternal<'a> {
    pub fn size(&mut self, size: u32) {
        if let Ok(rep) = &mut self.reply_arg {
            rep.size = size;
        }
    }

    pub fn data(&mut self, data: &[u8]) {
        if let Ok(rep) = &mut self.reply_buf {
            rep.truncate(data.len());
            let rep_slice = rep.to_slice_mut();
            rep_slice.copy_from_slice(data);
        }
    }

    pub fn error(&mut self, err: i32) {
        self.reply_buf = Err(err);
        self.reply_arg = Err(err);
    }

    pub fn reply_arg(&self) -> &Result<&'a mut fuse_getxattr_out, i32> {
        return &self.reply_arg;
    }

    pub fn reply_buf(&self) -> &Result<&'a mut MemContainer<raw::c_uchar>, i32> {
        return &self.reply_buf;
    }
}

pub type ReplyCreate<'a, 'b> = &'a mut ReplyCreateInternal<'b>;

#[derive(Debug)]
pub struct ReplyCreateInternal<'a> {
    pub reply: Result<(&'a mut fuse_entry_out, &'a mut fuse_open_out), i32>,
}

impl<'a> ReplyCreateInternal<'a> {
    pub fn created(&mut self, ttl: &Timespec, attr: &fuse_attr, generation: u64, fh: u64,
               flags: u32) { 
        if let Ok((rep_entry, rep_open)) = &mut self.reply {
            rep_entry.nodeid = attr.ino;
            rep_entry.generation = generation;
            rep_entry.entry_valid = ttl.sec as u64;
            rep_entry.attr_valid = ttl.sec as u64;
            rep_entry.entry_valid_nsec = ttl.nsec as u32;
            rep_entry.attr_valid_nsec = ttl.nsec as u32;
            rep_entry.attr.ino = attr.ino;
    	    rep_entry.attr.size = attr.size;
	        rep_entry.attr.blocks = attr.blocks;
	        rep_entry.attr.atime = attr.atime;
	        rep_entry.attr.mtime = attr.mtime;
    	    rep_entry.attr.ctime = attr.ctime;
	        rep_entry.attr.atimensec = attr.atimensec;
	        rep_entry.attr.mtimensec = attr.mtimensec;
	        rep_entry.attr.ctimensec = attr.ctimensec;
    	    rep_entry.attr.mode = attr.mode;
	        rep_entry.attr.nlink = attr.nlink;
	        rep_entry.attr.uid = attr.uid;
	        rep_entry.attr.gid = attr.gid;
    	    rep_entry.attr.rdev = attr.rdev;
	        rep_entry.attr.blksize = attr.blksize;
	        rep_entry.attr.padding = attr.padding;
            rep_open.fh = fh;
            rep_open.open_flags = flags;
            rep_open.padding = 0;
        }
    }

    pub fn error(&mut self, err: i32) {
        self.reply = Err(err);
    }

    pub fn reply(&self) -> &Result<(&'a mut fuse_entry_out, &'a mut fuse_open_out), i32> {
        return &self.reply;
    }
}

pub type ReplyLock<'a, 'b> = &'a mut ReplyLockInternal<'b>;

#[derive(Debug)]
pub struct ReplyLockInternal<'a> {
    pub reply: Result<&'a mut fuse_lk_out, i32>,
}

impl<'a> ReplyLockInternal<'a> {
    pub fn locked(&mut self, start: u64, end: u64, typ: u32, pid: u32) {
        if let Ok(rep) = &mut self.reply {
            rep.lk.start = start;
            rep.lk.end = end;
            rep.lk.type_ = typ;
            rep.lk.pid = pid;
        }
    }

    pub fn error(&mut self, err: i32) {
        self.reply = Err(err);
    }

    pub fn reply(&self) -> &Result<&'a mut fuse_lk_out, i32> {
        return &self.reply;
    }
}

pub type ReplyBmap<'a, 'b> = &'a mut ReplyBmapInternal<'b>;

#[derive(Debug)]
pub struct ReplyBmapInternal<'a> {
    pub reply: Result<&'a mut fuse_bmap_out, i32>,
}

impl<'a> ReplyBmapInternal<'a> {
    pub fn bmap(&mut self, block: u64) {
        if let Ok(rep) = &mut self.reply {
            rep.block = block;
        }
    }

    pub fn error(&mut self, err: i32) {
        self.reply = Err(err);
    }

    pub fn reply(&self) -> &Result<&'a mut fuse_bmap_out, i32> {
        return &self.reply;
    }
}
