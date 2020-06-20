use crate::xv6fs_fs::*;
use crate::xv6fs_utils::*;

use bento::kernel;
use kernel::semaphore::*;

pub struct CachedInode {
    pub idx: usize,
    pub inum: u32,
}

impl Drop for CachedInode {
    fn drop(&mut self) {
        let _ = iput(self);
    }
}

pub struct Inode {
    pub dev: u32,
    pub inum: u32,
    pub nref: i32,
    pub internals: Semaphore<InodeInternal>,
}

impl Inode {
    pub const fn new() -> Self {
        Inode {
            dev: 0,
            inum: 0,
            nref: 0,
            internals: Semaphore::new(InodeInternal::new()),
        }
    }
}

pub struct InodeInternal {
    pub valid: i32,

    pub inode_type: u16,
    pub major: u16,
    pub minor: u16,
    pub nlink: u16,
    pub size: u64,
    pub addrs: [u32; NDIRECT as usize + 2],
}

impl InodeInternal {
    pub const fn new() -> Self {
        InodeInternal {
            valid: 0,
            inode_type: 0,
            major: 0,
            minor: 0,
            nlink: 0,
            size: 0,
            addrs: [0; NDIRECT as usize + 2],
        }
    }
}
