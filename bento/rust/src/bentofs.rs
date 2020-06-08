use core::str;

use crate::bindings::*;
use kernel::errno;
use kernel::ffi::*;
use kernel::fuse::*;
use kernel::kobj::*;
use kernel::mem::*;
use kernel::raw;
use kernel::stat;
use kernel::time::Timespec;

pub const BENTO_KERNEL_VERSION: u32 = 1;
pub const BENTO_KERNEL_MINOR_VERSION: u32 = 0;

#[repr(C)]
pub struct bento_in_arg {
    size: u32,
    value: *const raw::c_void,
}

#[repr(C)]
pub struct bento_in {
    h: fuse_in_header,
    argpages: u8,
    numargs: u32,
    args: [bento_in_arg; 3],
}

#[repr(C)]
pub struct bento_arg {
    size: u32,
    value: *const raw::c_void,
}

#[repr(C)]
pub struct bento_out {
    h: fuse_out_header,
    argvar: u8,
    argpages: u8,
    page_zeroing: u8,
    page_replace: u8,
    numargs: u32,
    args: [bento_arg; 2],
}

#[derive(Default)]
#[allow(dead_code)]
pub struct FuseConnInfo {
    pub proto_major: u32,
    pub proto_minor: u32,
    pub max_write: u32,
    pub max_read: u32,
    pub max_readahead: u32,
    pub capable: u32,
    pub want: u32,
    pub max_background: u32,
    pub congestion_threshold: u32,
    pub time_gran: u32,
    reserved: [u32;22],
}

impl FuseConnInfo {
    fn from_init_in(inarg: &fuse_init_in) -> Self {
        let mut me: Self = Default::default();
        me.proto_major = inarg.major;
        me.proto_minor = inarg.minor;
        me.max_readahead = inarg.max_readahead;
        // Not sure if this is right
        me.capable = inarg.flags;
        me
    }

    fn to_init_out(&self, outarg: &mut fuse_init_out) {
        outarg.major = self.proto_major;
        outarg.minor = self.proto_minor;
        outarg.max_readahead = self.max_readahead;
        outarg.flags = self.want & self.capable;
        outarg.max_background = self.max_background as u16;
        outarg.congestion_threshold = self.congestion_threshold as u16;
        outarg.max_write = self.max_write;
        outarg.time_gran = self.time_gran;
    }
}


pub struct Request<'a> {
    h: &'a fuse_in_header
}

impl<'a> Request<'a> {
    #[inline]
    #[allow(dead_code)]
    pub fn unique(&self) -> u64 {
        self.h.unique
    }

    #[inline]
    #[allow(dead_code)]
    pub fn uid(&self) -> u32 {
        self.h.uid
    }

    /// Returns the gid of this request
    #[inline]
    #[allow(dead_code)]
    pub fn gid(&self) -> u32 {
        self.h.gid
    }

    /// Returns the pid of this request
    #[inline]
    #[allow(dead_code)]
    pub fn pid(&self) -> u32 {
        self.h.pid
    }
}

pub type ReplyEntry<'a, 'b> = &'a mut ReplyEntryInternal<'b>;

#[derive(Debug)]
pub struct ReplyEntryInternal<'a> {
    reply: Result<&'a mut fuse_entry_out, i32>,
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
    reply: Result<&'a mut fuse_attr_out, i32>,
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
    reply: Result<&'a mut MemContainer<raw::c_uchar>, i32>,
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

/// Register a file system with the BentoFS kernel module.
///
/// Should be called in the init function of a Bento file system module, before
/// the file system is mounted.
///
/// The name passed into fs_name is used to identify the file system on mount. It
/// must be a null-terminated string.
pub fn register_bento_fs_rs<T: FileSystem>(fs_name: &'static str, ops: &'static fs_ops<T>, _fs: T) -> i32 {
    return unsafe {
        register_bento_fs(
            0 as *const raw::c_void,
            fs_name.as_bytes().as_ptr() as *const raw::c_void,
            ops as *const fs_ops<T> as *const raw::c_void,
            T::dispatch as *const raw::c_void,
        )
    };
}

/// Unregister a file system from the BentoFS kernel module.
///
/// Should be called in the exit function of a Bento file system module, before
/// the file system is mounted.
///
/// The name passed into fs_name is used to identify the file system that should
/// be unregistered. It must be a null-terminated string.
pub fn unregister_bento_fs_rs(fs_name: &'static str) -> i32 {
    return unsafe { unregister_bento_fs(fs_name.as_bytes().as_ptr() as *const raw::c_void) };
}

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
    //_inarg_offset: usize,
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
    dirent.off = off + entlen_padded as u64;
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

pub const fn get_fs_ops<T: FileSystem>(_fs: &T) -> fs_ops<T> {
    fs_ops {
        mknod: T::mknod_unused,
        mkdir: T::mkdir,
        unlink: T::unlink,
        rmdir: T::rmdir,
        rename: T::rename,
        open: T::open,
        read: T::read,
        write: T::write,
        flush: T::flush,
        release: T::release,
        fsync: T::fsync,
        opendir: T::opendir,
        readdir: T::readdir,
        releasedir: T::releasedir,
        fsyncdir: T::fsyncdir,
        statfs: T::statfs,
        setxattr: T::setxattr,
        getxattr: T::getxattr,
        listxattr: T::listxattr,
        removexattr: T::removexattr,
        access: T::access,
        create: T::create,
        getlk: T::getlk,
        setlk: T::setlk,
        bmap: T::bmap,
        ioctl: T::ioctl,
        poll: T::poll,
        fallocate: T::fallocate,
        lseek: T::lseek,
    }
}

pub trait FileSystem {
    fn get_name(&self) -> &str;

    fn register(&self, ops: &'static fs_ops<Self>) -> i32 
        where Self: core::marker::Sized {
        return unsafe {
            register_bento_fs(
                self as *const Self as *const raw::c_void,
                self.get_name().as_bytes().as_ptr() as *const raw::c_void,
                ops as *const fs_ops<Self> as *const raw::c_void,
                Self::dispatch as *const raw::c_void,
            )
        };
    }

    fn init(&mut self, _sb: RsSuperBlock, _req: &Request, _fc_info: &mut FuseConnInfo) -> Result<(), i32>
    {
        return Err(-(ENOSYS as i32));
    }
    fn destroy(&mut self, _sb: RsSuperBlock, _req: &Request) -> Result<(), i32>
    {
        return Ok(());
    }
    fn lookup(&mut self, _sb: RsSuperBlock, _req: &Request, _parent: u64, _name: CStr, reply: ReplyEntry)
    {
        reply.error(-(ENOSYS as i32));
    }
    fn forget(&mut self, _sb: RsSuperBlock, _req: &Request,  _ino: u64, _nlookup: u64) {}
    fn getattr(&mut self, _sb: RsSuperBlock, _req: &Request, _ino: u64, reply: ReplyAttr)
    {
        reply.error(-(ENOSYS as i32));
    }
    fn setattr(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
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
    )
    {
        reply.error(-(ENOSYS as i32));
    }

    fn readlink(&self, _sb: RsSuperBlock, _req: &Request, _ino: u64, reply: ReplyData) {
        return reply.error(-(ENOSYS as i32));
    }

    fn mknod_unused(&self, _sb: RsSuperBlock, _nodeid: u64, _in_arg: &fuse_mknod_in, _name: CStr, _out_arg: &mut fuse_entry_out) -> i32
    {
        return -(ENOSYS as i32);
    }
    fn mknod(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _parent: u64,
        _name: CStr, //&OsStr
        _mode: u32,
        _rdev: u32,
        reply: ReplyEntry
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn mkdir(&self, _sb: RsSuperBlock, _nodeid: u64, _in_arg: &fuse_mkdir_in, _name: CStr, _out_arg: &mut fuse_entry_out) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn unlink(&self, _sb: RsSuperBlock, _nodeid: u64, _name: CStr) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn rmdir(&self, _sb: RsSuperBlock, _nodeid: u64, _name: CStr) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn symlink(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _parent: u64,
        _name: CStr,
        //_name: &OsStr,
        _link: CStr,
        //_link: &Path,
        reply: ReplyEntry
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn rename(&self, _sb: RsSuperBlock, _nodeid: u64, _in_arg: &fuse_rename2_in, _name1: CStr, _name2: CStr) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn open(&self, _sb: RsSuperBlock, _name: u64, _in_arg: &fuse_open_in, _out_arg: &mut fuse_open_out) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn read(&self, _sb: RsSuperBlock, _nodeid: u64, _in_arg: &fuse_read_in, _buf: &mut MemContainer<raw::c_uchar>) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn write(&self,
        _sb: RsSuperBlock,
        _nodeid: u64,
        _in_arg: &fuse_write_in,
        _buf: &MemContainer<raw::c_uchar>,
        _out_arg: &mut fuse_write_out,
    ) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn flush(&self, _sb: RsSuperBlock, _nodeid: u64, _in_arg: &fuse_flush_in) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn release(&self, _sb: RsSuperBlock, _nodeid: u64, _in_arg: &fuse_release_in) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn fsync(&self, _sb: RsSuperBlock, _nodeid: u64, _in_arg: &fuse_fsync_in) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn opendir(&self, _sb: RsSuperBlock, _nodeid: u64, _in_arg: &fuse_open_in, _out_arg: &mut fuse_open_out) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn readdir(&self, _sb: RsSuperBlock, _nodeid: u64, _in_arg: &fuse_read_in, _buf: &mut MemContainer<raw::c_uchar>, _offset: &mut usize) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn releasedir(&self, _sb: RsSuperBlock, _nodeid: u64, _in_arg: &fuse_release_in) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn fsyncdir(&self, _sb: RsSuperBlock, _nodeid: u64, _in_arg: &fuse_fsync_in) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn statfs(&self, _sb: RsSuperBlock, _nodeid: u64, _out_arg: &mut fuse_statfs_out) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn setxattr(&self, _sb: RsSuperBlock, _nodeid: u64, _in_arg: &fuse_setxattr_in, _name: CStr, _buf: &MemContainer<raw::c_uchar>) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn getxattr(&self,
        _sb: RsSuperBlock,
        _nodeid: u64,
        _in_arg: &fuse_getxattr_in,
        _name: CStr,
        _numargs: raw::c_size_t,
        _out_arg: &mut fuse_getxattr_out,
        _buf: &mut MemContainer<raw::c_uchar>,
    ) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn listxattr(&self,
        _sb: RsSuperBlock,
        _nodeid: u64,
        _in_arg: &fuse_getxattr_in,
        _numargs: raw::c_size_t,
        _out_arg: &mut fuse_getxattr_out,
        _buf: &mut MemContainer<raw::c_uchar>,
    ) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn removexattr(&self, _sb: RsSuperBlock, _nodeid: u64, _name: CStr) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn access(&self, _sb: RsSuperBlock, _nodeid: u64, _in_arg: &fuse_access_in) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn create(&self,
        _sb: RsSuperBlock,
        _nodeid: u64,
        _in_arg: &fuse_create_in,
        _name: CStr,
        _out_entry: &mut fuse_entry_out,
        _out_open: &mut fuse_open_out,
    ) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn getlk(&self, _sb: RsSuperBlock, _nodeid: u64, _in_arg: &fuse_lk_in, _out_arg: &mut fuse_lk_out) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn setlk(&self, _sb: RsSuperBlock, _nodeid: u64, _in_arg: &fuse_lk_in, _sleep: bool) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn bmap(&self, _sb: RsSuperBlock, _nodeid: u64, _in_arg: &fuse_bmap_in, _out_arg: &mut fuse_bmap_out) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn ioctl(&self,
        _sb: RsSuperBlock,
        _nodeid: u64,
        _in_arg: &fuse_ioctl_in,
        _out_arg: &mut fuse_ioctl_out,
        _buf: &mut MemContainer<raw::c_uchar>,
    ) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn poll(&self, _sb: RsSuperBlock, _nodeid: u64, _in_arg: &fuse_poll_in, _out_arg: &mut fuse_poll_out) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn fallocate(&self, _sb: RsSuperBlock, _nodeid: u64, _in_arg: &fuse_fallocate_in) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn lseek(&self, _sb: RsSuperBlock, _nodeid: u64, _in_arg: &fuse_lseek_in, _out_arg: &mut fuse_lseek_out) -> i32
    {
        return -(ENOSYS as i32);
    }

    fn dispatch(&mut self, opcode: fuse_opcode, sb: RsSuperBlock, inarg: &bento_in, outarg: &mut bento_out) -> i32 {
        println!("dispatching");
        match opcode {
            fuse_opcode_FUSE_INIT => {
                if inarg.numargs != 1 || outarg.numargs != 1 {
                    return -1;
                }

                let req = Request { h: &inarg.h };

                let init_in = unsafe { &*(inarg.args[0].value as *const fuse_init_in) };
                let init_out = unsafe { &mut *(outarg.args[0].value as *mut fuse_init_out) };
                let mut fc_info = FuseConnInfo::from_init_in(&init_in);
                match self.init(sb, &req, &mut fc_info) {
                    Ok(()) => {
                        fc_info.to_init_out(init_out);
                        0
                    },
                    Err(x) => x as i32,
                }
            },
            fuse_opcode_FUSE_DESTROY => {
                let req = Request { h: &inarg.h };
                match self.destroy(sb, &req) {
                    Ok(()) => 0,
                    Err(x) => x as i32,
                }
            },
            fuse_opcode_FUSE_LOOKUP => {
                if inarg.numargs != 1 || outarg.numargs != 1 {
                    return -1;
                }
                let req = Request { h: &inarg.h };
                let name = unsafe { CStr::from_raw(inarg.args[0].value as *const raw::c_char) };
                let entry_out = unsafe { &mut *(outarg.args[0].value as *mut fuse_entry_out) };
                let mut reply = ReplyEntryInternal { reply: Ok(entry_out) };
                self.lookup(sb, &req, inarg.h.nodeid, name, &mut reply);
                match reply.reply() {
                    Ok(_) => {
                        0
                    },
                    Err(x) => {
                        *x as i32
                    },
                }
            },
            fuse_opcode_FUSE_FORGET => {
                if inarg.numargs != 1 || outarg.numargs != 0 {
                    return -1;
                }
                let req = Request { h: &inarg.h };
                let forget_in = unsafe { &*(inarg.args[0].value as *const fuse_forget_in) };

                self.forget(sb, &req, inarg.h.nodeid, forget_in.nlookup);
                0
            },
            fuse_opcode_FUSE_GETATTR => {
                if inarg.numargs != 1 || outarg.numargs != 1 {
                    return -1;
                }

                let req = Request { h: &inarg.h };

                let _getattr_in = unsafe { &*(inarg.args[0].value as *const fuse_getattr_in) };
                let getattr_out = unsafe { &mut *(outarg.args[0].value as *mut fuse_attr_out) };
                let mut reply = ReplyAttrInternal { reply: Ok(getattr_out) };
                self.getattr(sb, &req, inarg.h.nodeid, &mut reply);
                match reply.reply() {
                    Ok(_) => {
                        0
                    },
                    Err(x) => {
                        *x as i32
                    },
                }
            },
            fuse_opcode_FUSE_SETATTR => {
                if inarg.numargs != 1 || outarg.numargs != 1 {
                    return -1;
                }

                let req = Request { h: &inarg.h };

                let setattr_in = unsafe { &*(inarg.args[0].value as *const fuse_setattr_in) };
                let setattr_out = unsafe { &mut *(outarg.args[0].value as *mut fuse_attr_out) };
                let mut reply = ReplyAttrInternal { reply: Ok(setattr_out) };
                let mode = match setattr_in.valid & FATTR_MODE {
                    0 => None,
                    _ => Some(setattr_in.mode),
                };
                let uid = match setattr_in.valid & FATTR_UID {
                    0 => None,
                    _ => Some(setattr_in.uid),
                };
                let gid = match setattr_in.valid & FATTR_GID {
                    0 => None,
                    _ => Some(setattr_in.gid),
                };
                let size = match setattr_in.valid & FATTR_SIZE {
                    0 => None,
                    _ => Some(setattr_in.size),
                };
                let atime = match setattr_in.valid & FATTR_ATIME {
                    0 => None,
                    _ => Some(Timespec {
                        sec: setattr_in.atime as i64,
                        nsec: setattr_in.atimensec as i32,
                    }),
                };
                let mtime = match setattr_in.valid & FATTR_MTIME {
                    0 => None,
                    _ => Some(Timespec {
                        sec: setattr_in.mtime as i64,
                        nsec: setattr_in.mtimensec as i32,
                    }),
                };
                let fh = match setattr_in.valid & FATTR_FH {
                    0 => None,
                    _ => Some(setattr_in.fh),
                };
                self.setattr(sb, &req, inarg.h.nodeid, mode, uid, gid, size, atime,
                             mtime, fh, None, None, None, None, &mut reply);
                match reply.reply() {
                    Ok(_) => {
                        0
                    },
                    Err(x) => {
                        *x as i32
                    },
                }
            },
            fuse_opcode_FUSE_READLINK => {
                if outarg.numargs != 1 {
                    return -1;
                }

                let req = Request { h: &inarg.h };
                let data_out = unsafe { &mut *(outarg.args[0].value as *mut MemContainer<raw::c_uchar>) };
                let mut reply = ReplyDataInternal { reply: Ok(data_out) };
                self.readlink(sb, &req, inarg.h.nodeid, &mut reply);
                match reply.reply() {
                    Ok(buf) => {
                        let buf_slice = buf.to_slice();
                        let buf_str = str::from_utf8(buf_slice).unwrap_or("");
                        buf_str.len() as i32
                    },
                    Err(x) => {
                        *x as i32
                    },
                }            
            },
            fuse_opcode_FUSE_MKNOD => {
                if inarg.numargs != 2 || outarg.numargs != 1 {
                    return -1;
                }
                let req = Request { h: &inarg.h };
                let mknod_in = unsafe { &*(inarg.args[0].value as *const fuse_mknod_in) };
                let name = unsafe { CStr::from_raw(inarg.args[1].value as *const raw::c_char) };
                let entry_out = unsafe { &mut *(outarg.args[0].value as *mut fuse_entry_out) };
                let mut reply = ReplyEntryInternal { reply: Ok(entry_out) };
                self.mknod(sb, &req, inarg.h.nodeid, name, mknod_in.mode, mknod_in.rdev, &mut reply);
                match reply.reply() {
                    Ok(_) => {
                        0
                    },
                    Err(x) => {
                        *x as i32
                    },
                }            
            },
            fuse_opcode_FUSE_SYMLINK => {
                if inarg.numargs != 2 || outarg.numargs != 1 {
                    return -1;
                }

                let req = Request { h: &inarg.h };
                let name = unsafe { CStr::from_raw(inarg.args[0].value as *const raw::c_char) };
                let link = unsafe { CStr::from_raw(inarg.args[1].value as *const raw::c_char) };
                let entry_out = unsafe { &mut *(outarg.args[0].value as *mut fuse_entry_out) };
                let mut reply = ReplyEntryInternal { reply: Ok(entry_out) };
                self.symlink(sb, &req, inarg.h.nodeid, name, link, &mut reply);
                match reply.reply() {
                    Ok(_) => {
                        0
                    },
                    Err(x) => {
                        *x as i32
                    },
                }            
            },
            _ => {
                println!("got a different opcode");
                0
            },
        }
    }
}


/// Filesystem operations.
///
/// These functions are implemented by the file system and provided to Bento
/// through `register_bento_fs_rs`. The BentoFS kernel module then calls these
/// functions.
///
/// These functions are modeled after the FUSE lowlevel API and require similar
/// functionality. Since these functions are called from C, they return i32
/// rather than Result.
///
/// The `fuse_*_in` and `fuse_*_out` and defined in the Linux kernel in
/// /include/uapi/linux/fuse.h.
#[repr(C)]
pub struct fs_ops<T: FileSystem> {
    /// Initialize the file system and fill in initialization flags.
    ///
    /// Possible initialization flags are defined /include/uapi/linux/fuse.h.
    /// No support is provided for readdirplus and async DIO.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `in_arg: &fuse_init_in` - Data structure containing init args from Bento.
    /// * `out_arg: &mut fuse_init_out` - Data structure to be filled.

    /// Perform any necessary cleanup on the file system.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.

    /// Look up a directory entry by name and get its attributes.
    ///
    /// If the entry exists, fill `fuse_entry_out` with the attributes.
    /// Otherwise, return `-ENOENT`.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - The file system-provided inode number of the parent directory
    /// * `name: CStr` - The name of the file to lookup.

    /// Forget about an inode
    ///
    /// This function is called when the kernel removes an inode from its internal caches.
    ///
    /// Inodes with a non-zero lookup count may receive request from Bento even after calls to
    /// unlink, rmdir or (when overwriting an existing file) rename. Filesystems must handle such
    /// requests properly and it is recommended to defer removal of the inode until the lookup
    /// count reaches zero. Calls to unlink, rmdir or rename will be followed closely by forget
    /// unless the file or directory is open, in which case Bento issues forget only after the
    /// release or releasedir calls.
    ///
    /// Note that if a file system will be exported over NFS the inodes lifetime must extend even
    /// beyond forget. See the generation field in struct fuse_entry_param above.
    ///
    /// On unmount the lookup count for all inodes implicitly drops to zero. It is not guaranteed
    /// that the file system will receive corresponding forget messages for the affected inodes.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number of the inode to forget.
    /// * `nlookup: u64` - The number of lookups to forget.

    /// Get file attributes.
    ///
    /// If writeback caching is enabled, Bento may have a better idea of a file's length than the
    /// file system (eg if there has been a write that extended the file size, but that has not
    /// yet been passed to the filesystem.
    ///
    /// In this case, the st_size value provided by the file system will be ignored.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided id of the inode.
    /// * `in_arg: &fuse_getattr_in` - Data structure containing getattr input arguments.
    /// * `out_arg: &mut fuse_attr_out` - Data structure to be filled with attribute information.

    /// Set file attributes
    ///
    /// In the 'attr' argument only members indicated by the 'to_set' bitmask contain valid values.
    /// Other members contain undefined values.
    ///
    /// Unless FUSE_CAP_HANDLE_KILLPRIV is disabled, this method is expected to reset the setuid
    /// and setgid bits if the file size or owner is being changed.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided id of the inode.
    /// * `in_arg: &fuse_setattr_in` - Data structure containing setattr input arguments.
    /// * `out_arg: &mut fuse_attr_out` - Data structure to be filled with attribute information.

    /// Read symbolic link.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided id of the inode.
    /// * `buf: &mut MemContainer<kernel::raw::c_uchar>` - Bento-provided buffer for the link name.

    /// Create file node
    ///
    /// Create a regular file, character device, block device, fifo or socket node.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number of the parent directory.
    /// * `in_arg: &fuse_mknod_in` - Data structure containing mknod input arguments.
    /// * `name: CStr` - Name of the file to be created.
    /// * `out_arg: &mut fuse_entry_out` - Data structure to be filled with data about the newly
    /// created file.
    pub mknod: fn(&T, RsSuperBlock, u64, &fuse_mknod_in, CStr, &mut fuse_entry_out) -> i32,

    /// Create directory.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number of the parent directory.
    /// * `in_arg: &fuse_mkdir_in` - Data structure containing mkdir input arguments.
    /// * `name: CStr` - Name of the directory to be created.
    /// * `out_arg: &mut fuse_entry_out` - Data structure to be filled with data about the newly
    /// created directory.
    pub mkdir: fn(&T, RsSuperBlock, u64, &fuse_mkdir_in, CStr, &mut fuse_entry_out) -> i32,

    /// Remove a file.
    ///
    /// If the file's inode's lookup count is non-zero, the file system is expected to postpone any
    /// removal of the inode until the lookup count reaches zero (see description of the forget
    /// function).
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number of the parent directory.
    /// * `name: CStr` - Name of the file to be removed.
    pub unlink: fn(&T, RsSuperBlock, u64, CStr) -> i32,

    /// Remove a directory.
    ///
    /// If the file's inode's lookup count is non-zero, the file system is expected to postpone any
    /// removal of the inode until the lookup count reaches zero (see description of the forget
    /// function).
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number of the parent directory.
    /// * `name: CStr` - Name of the directory to be removed.
    pub rmdir: fn(&T, RsSuperBlock, u64, CStr) -> i32,

    /// Create a symbolic link.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number of the parent directory.
    /// * `name: CStr` - Name of the link to be created.
    /// * `linkname: CStr` - The contents of the symbolic link.
    /// * `out_arg: &mut fuse_entry_out` - Data structure to be filled with data about the newly
    /// created link.

    /// Rename a file
    ///
    /// If the target exists it should be atomically replaced. If the target's inode's lookup count
    /// is non-zero, the file system is expected to postpone any removal of the inode until the
    /// lookup count reaches zero (see description of the forget function).
    ///
    /// If this request is answered with an error code of ENOSYS, this is treated as a permanent
    /// failure with error code EINVAL, i.e. all future bmap requests will fail with EINVAL without
    /// being sent to the filesystem.
    ///
    /// `flags` in `fuse_rename2_in` may be `RENAME_EXCHANGE` or `RENAME_NOREPLACE`. If
    /// `RENAME_NOREPLACE` is specified, the filesystem must not overwrite newname if it exists
    /// and return an error instead. If `RENAME_EXCHANGE` is specified, the filesystem must
    /// atomically exchange the two files, i.e. both must exist and neither may be deleted.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number of the parent directory.
    /// * `in_arg: &fuse_rename2_in` - Data structure containing rename input arguments.
    /// * `oldname: CStr` - Old name of the file.
    /// * `newname: CStr` - New name of the file.
    pub rename: fn(&T, RsSuperBlock, u64, &fuse_rename2_in, CStr, CStr) -> i32,

    /// Open a file.
    ///
    /// Open flags are available in `in_arg->flags`. The following rules apply.
    ///
    /// Creation (`O_CREAT`, `O_EXCL`, `O_NOCTTY`) flags will be filtered out / handled by Bento.
    /// Access modes (`O_RDONLY`, `O_WRONLY`, `O_RDWR`) should be used by the filesystem to check
    /// if the operation is permitted. If the -o default_permissions mount option is given, this
    /// check is already done by Bento before calling `open()` and may thus be omitted by the
    /// filesystem.
    /// When writeback caching is enabled, Bento may send read requests even for files opened with
    /// `O_WRONLY`. The filesystem should be prepared to handle this.
    /// When writeback caching is disabled, the filesystem is expected to properly handle the
    /// `O_APPEND` flag and ensure that each write is appending to the end of the file.
    /// When writeback caching is enabled, Bento will handle `O_APPEND`. However, unless all changes
    /// to the file come through Bento this will not work reliably. The filesystem should thus
    /// either ignore the `O_APPEND` flag (and let Bento handle it), or return an error (indicating
    /// that reliably `O_APPEND` is not available).
    /// Filesystem may store an arbitrary file handle (pointer, index, etc) in `out_arg->fh`, and
    /// use this in other all other file operations (read, write, flush, release, fsync).
    ///
    /// Filesystem may also implement stateless file I/O and not store anything in `out_arg->fh`.
    ///
    /// There are also some flags (keep_cache) which the filesystem may set in `out_arg`, to change
    /// the way the file is opened. See `fuse_file_info` structure in <fuse_common.h> for more details.
    ///
    /// If this request is answered with an error code of `ENOSYS` and `FUSE_CAP_NO_OPEN_SUPPORT`
    /// is set in `fuse_conn_info.capable`, this is treated as success and future calls to open and
    /// release will also succeed without being sent to the filesystem.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number.
    /// * `in_arg: &fuse_open_in` - Data structure containing open input arguments.
    /// * `out_arg: &mut fuse_open_out` - Data structure to be filled with open output.
    pub open: fn(&T, RsSuperBlock, u64, &fuse_open_in, &mut fuse_open_out) -> i32,

    /// Read data
    ///
    /// Read should send exactly the number of bytes requested except on EOF or error, otherwise
    /// the rest of the data will be substituted with zeroes.
    ///
    /// `in_arg->fh` will contain the value set by the open method, or will be undefined if the open
    /// method didn't set any value.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number.
    /// * `in_arg: &fuse_read_in` - Data structure containing read input arguments.
    /// * `buf: &mut MemContainer<kernel::raw::c_uchar` - Buffer to be filled with the read data.
    pub read: fn(&T, RsSuperBlock, u64, &fuse_read_in, &mut MemContainer<raw::c_uchar>) -> i32,

    /// Write data
    ///
    /// Write should return exactly the number of bytes requested except on error.
    ///
    /// Unless `FUSE_CAP_HANDLE_KILLPRIV` is disabled, this method is expected to reset the setuid
    /// and setgid bits.
    ///
    /// `in_arg->fh` will contain the value set by the open method, or will be undefined if the open
    /// method didn't set any value.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number.
    /// * `in_arg: &fuse_write_in` - Data structure containing write input arguments.
    /// * `buf: &MemContainer<kernel::raw::c_uchar` - Buffer containing the data to write.
    /// * `out_arg: &mut fuse_write_out` - Data structure to be filled with write output arguments.
    pub write: fn(&T, 
        RsSuperBlock,
        u64,
        &fuse_write_in,
        &MemContainer<raw::c_uchar>,
        &mut fuse_write_out,
    ) -> i32,

    /// Flush method
    ///
    /// This is called on each `close()` of the opened file.
    ///
    /// Since file descriptors can be duplicated (dup, dup2, fork), for one open call there may be
    /// many flush calls.
    ///
    /// Filesystems shouldn't assume that flush will always be called after some writes, or that if
    /// will be called at all.
    ///
    /// `in_arg->fh` will contain the value set by the open method, or will be undefined if the open
    /// method didn't set any value.
    ///
    /// NOTE: the name of the method is misleading, since (unlike fsync) the filesystem is not
    /// forced to flush pending writes. One reason to flush data is if the filesystem wants to
    /// return write errors during close. However, such use is non-portable because POSIX does not
    /// require close to wait for delayed I/O to complete.
    ///
    /// If the filesystem supports file locking operations (setlk, getlk) it should remove all locks
    /// belonging to `in_arg->lock_owner`.
    ///
    /// If this request is answered with an error code of `ENOSYS`, this is treated as success and
    /// future calls to `flush()` will succeed automatically without being sent to the filesystem.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number.
    /// * `in_arg: &fuse_flush_in` - Data structure containing flush input arguments.
    pub flush: fn(&T, RsSuperBlock, u64, &fuse_flush_in) -> i32,

    /// Release an open file
    ///
    /// Release is called when there are no more references to an open file: all file descriptors
    /// are closed and all memory mappings are unmapped.
    ///
    /// For every open call there will be exactly one release call (unless the filesystem is
    /// force-unmounted).
    ///
    /// The filesystem may reply with an error, but error values are not returned to `close()` or
    /// `munmap()` which triggered the release.
    ///
    /// `in_arg->fh` will contain the value set by the open method, or will be undefined if the open
    /// method didn't set any value. `in_arg->flags` will contain the same flags as for open.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number.
    /// * `in_arg: &fuse_release_in` - Data structure containing release input arguments.
    pub release: fn(&T, RsSuperBlock, u64, &fuse_release_in) -> i32,

    /// Synchronize file contents
    ///
    /// If the `in_arg->flags` parameter is non-zero, then only the user data should be flushed,
    /// not the meta data.
    ///
    /// If this request is answered with an error code of `ENOSYS`, this is treated as success and
    /// future calls to `fsync()` will succeed automatically without being sent to the filesystem.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number.
    /// * `in_arg: &fuse_fsync_in` - Data structure containing fsync input arguments.
    pub fsync: fn(&T, RsSuperBlock, u64, &fuse_fsync_in) -> i32,

    /// Open a directory.
    ///
    /// Filesystem may store an arbitrary file handle (pointer, index, etc) in `in_arg->fh`, and use
    /// this in other all other directory stream operations (readdir, releasedir, fsyncdir).
    ///
    /// If this request is answered with an error code of `ENOSYS` and `FUSE_CAP_NO_OPENDIR_SUPPORT`
    /// is set in `fuse_conn_info.capable`, this is treated as success and future calls to opendir
    /// and releasedir will also succeed without being sent to the filesystem. In addition,
    /// Bento will cache readdir results as if opendir returned
    /// `FOPEN_KEEP_CACHE` | `FOPEN_CACHE_DIR`.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number.
    /// * `in_arg: &fuse_open_in` - Data structure containing open input arguments.
    /// * `out_arg: &fuse_open_out` - Data structure to be filled with open information.
    pub opendir: fn(&T, RsSuperBlock, u64, &fuse_open_in, &mut fuse_open_out) -> i32,

    /// Read directory
    ///
    /// Send a buffer filled using bento_add_direntry(), with size not exceeding the requested size.
    /// Send an empty buffer on end of stream.
    ///
    /// `in_arg->fh` will contain the value set by the opendir method, or will be undefined if the
    /// opendir method didn't set any value.
    ///
    /// Returning a directory entry from readdir() does not affect its lookup count.
    ///
    /// If `in_arg->offset` is non-zero, then it will correspond to one of the `off` values that
    /// was previously returned by `readdir()` for the same directory handle. In this case,
    /// `readdir()` should skip over entries coming before the position defined by the
    /// `in_arg->offset` value. If entries are added or removed while the directory handle is open,
    /// they filesystem may still include the entries that have been removed, and may not report the
    /// entries that have been created. However, addition or removal of entries must never cause
    /// `readdir()` to skip over unrelated entries or to report them more than once. This means that
    /// `in_arg->offset` can not be a simple index that enumerates the entries that have been
    /// returned but must contain sufficient information to uniquely determine the next directory
    /// entry to return even when the set of entries is changing.
    ///
    /// The function does not have to report the '.' and '..' entries, but is allowed to do so. Note
    /// that, if readdir does not return '.' or '..', they will not be implicitly returned, and this
    /// behavior is observable by the caller.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number of the directory.
    /// * `in_arg: &fuse_read_in` - Data structure containing read input arguments.
    /// * `buf: &mut MemContainer<kernel::raw::c_uchar` - Buffer to be filled with the direntry data.
    pub readdir:
        fn(&T, RsSuperBlock, u64, &fuse_read_in, &mut MemContainer<raw::c_uchar>, &mut usize) -> i32,

    /// Release an open directory
    ///
    /// For every opendir call there will be exactly one releasedir call (unless the filesystem is
    /// force-unmounted).
    /// `in_arg->fh` will contain the value set by the opendir method, or will be undefined if the
    /// opendir method didn't set any value.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number of the directory.
    /// * `in_arg: &fuse_release_in` - Data structure containing release input arguments.
    pub releasedir: fn(&T, RsSuperBlock, u64, &fuse_release_in) -> i32,

    /// Synchronize directory contents
    ///
    /// If the `in_arg->fsync_flags` parameter is non-zero, then only the directory contents should
    /// be flushed, not the meta data.
    ///
    /// `in_arg->fh` will contain the value set by the opendir method, or will be undefined if the
    /// opendir method didn't set any value.
    ///
    /// If this request is answered with an error code of `ENOSYS`, this is treated as success and
    /// future calls to `fsyncdir()` will succeed automatically without being sent to the filesystem.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number of the directory.
    /// * `in_arg: &fuse_fsync_in` - Data structure containing fsync input arguments.
    pub fsyncdir: fn(&T, RsSuperBlock, u64, &fuse_fsync_in) -> i32,

    /// Get filesystem statistics.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number, zero means "undefined".
    /// * `out_arg: &fuse_statfs_out` - Data structure to be filled with statfs information.
    pub statfs: fn(&T, RsSuperBlock, u64, &mut fuse_statfs_out) -> i32,

    /// Set an extended attribute
    ///
    /// If this request is answered with an error code of `ENOSYS`, this is treated as a permanent
    /// failure with error code `EOPNOTSUPP`, i.e. all future `setxattr()` requests will fail with
    /// `EOPNOTSUPP` without being sent to the filesystem.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number.
    /// * `in_arg: &fuse_setxattr_in_out` - Data structure containing setxattr input arguments.
    /// * `name: CStr` - The name of the attribute.
    /// * `buf: &MemContainer<kernel::raw::c_uchar>` - Buffer containing the data to be set in the
    /// attribute.
    pub setxattr:
        fn(&T, RsSuperBlock, u64, &fuse_setxattr_in, CStr, &MemContainer<raw::c_uchar>) -> i32,

    /// Get an extended attribute
    ///
    /// If size is zero, the size of the value should be sent in `out_arg->size`.
    ///
    /// If the size is non-zero, and the value fits in the buffer, the value should be sent in `buf`.
    ///
    /// If the size is too small for the value, the `ERANGE` error should be sent.
    ///
    /// If this request is answered with an error code of `ENOSYS`, this is treated as a permanent
    /// failure with error code `EOPNOTSUPP`, i.e. all future `getxattr()` requests will fail
    /// with `EOPNOTSUPP` without being sent to the filesystem.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number.
    /// * `in_arg: &fuse_getxattr_in_out` - Data structure containing getxattr input arguments.
    /// * `name: CStr` - The name of the attribute.
    /// * `size: kernel::raw::c_size_t` - The size of the buffer.
    /// * `out_arg: &mut fuse_getxattr_out` - Data structure to be filled with getxattr output
    /// information.
    /// * `buf: &mut MemContainer<kernel::raw::c_uchar>` - Buffer to be filled with the attribute data.
    pub getxattr: fn(&T, 
        RsSuperBlock,
        u64,
        &fuse_getxattr_in,
        CStr,
        raw::c_size_t,
        &mut fuse_getxattr_out,
        &mut MemContainer<raw::c_uchar>,
    ) -> i32,

    /// List extended attribute names
    ///
    /// If size is zero, the total size of the attribute list should be sent in `fuse_getxattr_out`.
    ///
    /// If the size is non-zero, and the null character separated attribute list fits in the buffer,
    /// the list should be sent in the buffer.
    ///
    /// If the size is too small for the list, the `ERANGE` error should be sent.
    ///
    /// If this request is answered with an error code of `ENOSYS`, this is treated as a permanent
    /// failure with error code `EOPNOTSUPP`, i.e. all future `listxattr()` requests will fail with
    /// `EOPNOTSUPP` without being sent to the filesystem.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number.
    /// * `in_arg: &fuse_getxattr_in_out` - Data structure containing getxattr input arguments.
    /// * `name: CStr` - The name of the attribute.
    /// * `size: kernel::raw::c_size_t` - The size of the buffer.
    /// * `out_arg: &mut fuse_getxattr_out` - Data structure to be filled with getxattr output
    /// information.
    /// * `buf: &mut MemContainer<kernel::raw::c_uchar>` - Buffer to be filled with the attribute data.
    pub listxattr: fn(&T, 
        RsSuperBlock,
        u64,
        &fuse_getxattr_in,
        raw::c_size_t,
        &mut fuse_getxattr_out,
        &mut MemContainer<raw::c_uchar>,
    ) -> i32,

    /// Remove an extended attribute
    ///
    /// If this request is answered with an error code of `ENOSYS`, this is treated as a permanent
    /// failure with error code `EOPNOTSUPP`, i.e. all future `removexattr()` requests will fail
    /// with `EOPNOTSUPP` without being sent to the filesystem.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number.
    /// * `name: CStr` - The name of the attribute to remove.
    pub removexattr: fn(&T, RsSuperBlock, u64, CStr) -> i32,

    /// Check file access permissions
    ///
    /// This will be called for the `access()` and `chdir()` system calls. If the
    /// 'default_permissions' mount option is given, this method is not called.
    ///
    /// If this request is answered with an error code of `ENOSYS`, this is treated as a permanent
    /// success, i.e. this and all future `access()` requests will succeed without being sent to
    /// the filesystem.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number.
    /// * `in_arg: &fuse_access_in` - Data structure containing access input arguments.
    pub access: fn(&T, RsSuperBlock, u64, &fuse_access_in) -> i32,

    /// Create and open a file
    ///
    /// If the file does not exist, first create it with the specified mode, and then open it.
    ///
    /// See the description of the open handler for more information.
    ///
    /// If this method is not implemented, the mknod() and open() methods will be called instead.
    ///
    /// If this request is answered with an error code of `ENOSYS`, the handler is treated as not
    /// implemented (i.e., for this and future requests the mknod() and open() handlers will be
    /// called instead).
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number of the parent directory.
    /// * `in_arg: &fuse_create_in` - Data structure containing create input arguments.
    /// * `name: CStr` - The name of the new file.
    /// * `out_entry: &mut fuse_entry_out` - Data structure to be filled with information about the
    /// newly created entry.
    /// * `out_open: &mut fuse_open_out` - Data structure to be filled with information about the
    /// newly opened file.
    pub create: fn(&T, 
        RsSuperBlock,
        u64,
        &fuse_create_in,
        CStr,
        &mut fuse_entry_out,
        &mut fuse_open_out,
    ) -> i32,

    /// Test for a POSIX file lock.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number.
    /// * `in_arg: &fuse_lk_in` - Data structure containing getlk input arguments.
    /// * `out_arg: &mut fuse_lk_out` - Data structure to be filled with getlk output information.
    pub getlk: fn(&T, RsSuperBlock, u64, &fuse_lk_in, &mut fuse_lk_out) -> i32,

    /// Acquire, modify or release a POSIX file lock
    ///
    /// For POSIX threads (NPTL) there's a 1-1 relation between pid and owner, but otherwise this is
    /// not always the case. For checking lock ownership, `in_rg->owner` must be used. The `pid`
    /// field in `struct fuse_file_lock` should only be used to fill in this field in `getlk()`.
    ///
    /// Note: if the locking methods are not implemented, the kernel will still allow file locking
    /// to work locally. Hence these are only interesting for network filesystems and similar.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number.
    /// * `in_arg: &fuse_lk_in` - Data structure containing setlk input arguments.
    /// * `sleep: bool` - Should the filesystem sleep.
    pub setlk: fn(&T, RsSuperBlock, u64, &fuse_lk_in, bool) -> i32,

    /// Map block index within file to block index within device
    ///
    /// Note: This makes sense only for block device backed filesystems mounted with the 'blkdev'
    /// option.
    ///
    /// If this request is answered with an error code of `ENOSYS`, this is treated as a permanent
    /// failure, i.e. all future `bmap()` requests will fail with the same error code without being
    /// sent to the filesystem.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number.
    /// * `in_arg: &fuse_bmap_in` - Data structure containing bmap input arguments.
    /// * `out_arg: &mut fuse_bmap_out` - Data structure to be filled with bmap output information.
    pub bmap: fn(&T, RsSuperBlock, u64, &fuse_bmap_in, &mut fuse_bmap_out) -> i32,

    /// Ioctl
    ///
    /// Note: For unrestricted ioctls, data in and out areas can be discovered by giving iovs and
    /// setting `FUSE_IOCTL_RETRY` in flags. For restricted ioctls, Bento prepares in/out data area
    /// according to the information encoded in `cmd`.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number.
    /// * `in_arg: &fuse_ioctl_in` - Data structure containing ioctl input arguments.
    /// * `out_arg: &mut fuse_ioctl_out` - Data structure to be filled with ioctl output information.
    /// * `buf: &mut MemContainer<kernel::raw::c_uchar>` - Buffer to be filled with ioctl output
    /// information.
    pub ioctl: fn(&T, 
        RsSuperBlock,
        u64,
        &fuse_ioctl_in,
        &mut fuse_ioctl_out,
        &mut MemContainer<raw::c_uchar>,
    ) -> i32,

    /// Poll for IO readiness
    ///
    /// If this request is answered with an error code of `ENOSYS`, this is treated as success
    /// (with a kernel-defined default poll-mask) and future calls to `poll()` will succeed the
    /// same way without being sent to the filesystem.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number.
    /// * `in_arg: &fuse_poll_in` - Data structure containing poll input arguments.
    /// * `out_arg: &mut fuse_poll_out` - Data structure to be filled with poll output information.
    pub poll: fn(&T, RsSuperBlock, u64, &fuse_poll_in, &mut fuse_poll_out) -> i32,

    /// Allocate requested space. If this function returns success then subsequent writes to the
    /// specified range shall not fail due to the lack of free space on the file system storage
    /// media.
    ///
    /// If this request is answered with an error code of `ENOSYS`, this is treated as a permanent
    /// failure with error code `EOPNOTSUPP`, i.e. all future `fallocate()` requests will fail with
    /// `EOPNOTSUPP` without being sent to the filesystem.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number.
    /// * `in_arg: &fuse_fallocate_in` - Data structure containing fallocate input arguments.
    pub fallocate: fn(&T, RsSuperBlock, u64, &fuse_fallocate_in) -> i32,

    /// Find next data or hole after the specified offset
    ///
    /// If this request is answered with an error code of `ENOSYS`, this is treated as a permanent
    /// failure, i.e. all future `lseek()` requests will fail with the same error code without being
    /// sent to the filesystem.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `nodeid: u64` - Filesystem-provided inode number.
    /// * `in_arg: &fuse_lseek_in` - Data structure containing lseek input arguments.
    /// * `out_arg: &mut fuse_lseek_out` - Data structure to be filled with lseek output information.
    pub lseek: fn(&T, RsSuperBlock, u64, &fuse_lseek_in, &mut fuse_lseek_out) -> i32,
}
