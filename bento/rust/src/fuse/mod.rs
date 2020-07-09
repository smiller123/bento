mod internal;
mod reply;
mod request;

use time::Timespec;

pub use self::reply::{ReplyEmpty, ReplyData, ReplyEntry, ReplyAttr, ReplyOpen};
pub use self::reply::{ReplyWrite, ReplyStatfs, ReplyCreate, ReplyLock, ReplyBmap, ReplyDirectory};
pub use self::reply::ReplyXattr;
pub use self::request::{Request,FuseConnInfo,dispatch};

pub mod consts {
    // Bitmasks for fuse_setattr_in.valid
    pub const FATTR_MODE: u32               = 1 << 0;
    pub const FATTR_UID: u32                = 1 << 1;
    pub const FATTR_GID: u32                = 1 << 2;
    pub const FATTR_SIZE: u32               = 1 << 3;
    pub const FATTR_ATIME: u32              = 1 << 4;
    pub const FATTR_MTIME: u32              = 1 << 5;
    pub const FATTR_FH: u32                 = 1 << 6;

    // Flags returned by the open request
    pub const FOPEN_DIRECT_IO: u32          = 1 << 0;   // bypass page cache for this open file
    pub const FOPEN_KEEP_CACHE: u32         = 1 << 1;   // don't invalidate the data cache on open

    // Init request/reply flags
    pub const FUSE_ASYNC_READ: u32          = 1 << 0;
    pub const FUSE_POSIX_LOCKS: u32         = 1 << 1;

    // Release flags
    pub const FUSE_RELEASE_FLUSH: u32       = 1 << 0;

    // The read buffer is required to be at least 8k, but may be much larger
    pub const FUSE_MIN_READ_BUFFER: usize   = 8192;
}

#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub enum FileType {
    /// Named pipe (S_IFIFO)
    NamedPipe,
    /// Character device (S_IFCHR)
    CharDevice,
    /// Block device (S_IFBLK)
    BlockDevice,
    /// Directory (S_IFDIR)
    Directory,
    /// Regular file (S_IFREG)
    RegularFile,
    /// Symbolic link (S_IFLNK)
    Symlink,
    /// Unix domain socket (S_IFSOCK)
    Socket,
}

/// File attributes
#[derive(Clone, Copy, Debug)]
pub struct FileAttr {
    /// Inode number
    pub ino: u64,
    /// Size in bytes
    pub size: u64,
    /// Size in blocks
    pub blocks: u64,
    /// Time of last access
    pub atime: Timespec,
    /// Time of last modification
    pub mtime: Timespec,
    /// Time of last change
    pub ctime: Timespec,
    /// Time of creation (macOS only)
    pub crtime: Timespec,
    /// Kind of file (directory, file, pipe, etc)
    pub kind: FileType,
    /// Permissions
    pub perm: u16,
    /// Number of hard links
    pub nlink: u32,
    /// User id
    pub uid: u32,
    /// Group id
    pub gid: u32,
    /// Rdev
    pub rdev: u32,
    /// Flags (macOS only, see chflags(2))
    pub flags: u32,
}
