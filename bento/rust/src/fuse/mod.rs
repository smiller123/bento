pub mod reply;
pub mod request;

use core::str;

use crate::bindings::*;
use crate::fuse::reply::*;
use crate::fuse::request::*;

use kernel::ffi::*;
use kernel::kobj::*;
use kernel::raw;
use kernel::time::Timespec;

pub const BENTO_KERNEL_VERSION: u32 = 1;
pub const BENTO_KERNEL_MINOR_VERSION: u32 = 0;

pub trait FileSystem {
    fn get_name(&self) -> &str;

    fn register(&self) -> i32 
        where Self: core::marker::Sized {
        return unsafe {
            register_bento_fs(
                self as *const Self as *const raw::c_void,
                self.get_name().as_bytes().as_ptr() as *const raw::c_void,
                dispatch::<Self> as *const raw::c_void,
            )
        };
    }
    
    fn unregister(&self) -> i32 {
        return unsafe {
            unregister_bento_fs(self.get_name().as_bytes().as_ptr() as *const raw::c_void)
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

    fn mkdir(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _parent: u64,
        _name: CStr, //&OsStr
        _mode: u32,
        reply: ReplyEntry
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn unlink(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _parent: u64,
        _name: CStr, //&OsStr
        reply: ReplyEmpty
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn rmdir(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _parent: u64,
        _name: CStr, //&OsStr
        reply: ReplyEmpty
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn symlink(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _parent: u64,
        _name: CStr, // &OsStr
        _link: CStr, // & Path
        reply: ReplyEntry
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn rename(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _parent: u64,
        _name: CStr, //&OsStr
        _newparent: u64,
        _newname: CStr, //&OsStr,
        reply: ReplyEmpty
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn link(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _newparent: u64,
        _newname: CStr, // &OsStr
        reply: ReplyEntry
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn open(&mut self, _sb: RsSuperBlock, _req: &Request, _ino: u64, _flags: u32, reply: ReplyOpen)
    {
        return reply.error(-(ENOSYS as i32));
    }

    fn read(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        _size: u32,
        reply: ReplyData
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn write(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        _data: &[u8],
        _flags: u32,
        reply: ReplyWrite
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn flush(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _lock_owner: u64,
        reply: ReplyEmpty
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn release(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _flags: u32,
        _lock_owner: u64,
        _flush: bool,
        reply: ReplyEmpty
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn fsync(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn opendir(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _flags: u32,
        reply: ReplyOpen
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn readdir(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        reply: ReplyDirectory
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn releasedir(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _flags: u32,
        reply: ReplyEmpty
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn fsyncdir(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn statfs(&mut self, _sb: RsSuperBlock, _req: &Request, _ino: u64, reply: ReplyStatfs) {
        return reply.error(-(ENOSYS as i32));
    }

    fn setxattr(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _name: CStr, //&OsStr,
        _value: &[u8],
        _flags: u32,
        _position: u32,
        reply: ReplyEmpty
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn getxattr(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _name: CStr, //&OsStr,
        _size: u32,
        reply: ReplyXattr
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn listxattr(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _size: u32,
        reply: ReplyXattr
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn removexattr(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _name: CStr, //&OsStr,
        reply: ReplyEmpty
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn access(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _mask: u32,
        reply: ReplyEmpty
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn create(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _parent: u64,
        _name: CStr, //&OsStr,
        _mode: u32,
        _flags: u32,
        reply: ReplyCreate
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn getlk(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _lock_owner: u64,
        _start: u64,
        _end: u64,
        _typ: u32,
        _pid: u32,
        reply: ReplyLock
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn setlk(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _lock_owner: u64,
        _start: u64,
        _end: u64,
        _typ: u32,
        _pid: u32,
        _sleep: bool,
        reply: ReplyEmpty
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    fn bmap(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _blocksize: u32,
        _idx: u64,
        reply: ReplyBmap
    ) {
        return reply.error(-(ENOSYS as i32));
    }
}


///// Filesystem operations.
/////
///// These functions are implemented by the file system and provided to Bento
///// through `register_bento_fs_rs`. The BentoFS kernel module then calls these
///// functions.
/////
///// These functions are modeled after the FUSE lowlevel API and require similar
///// functionality. Since these functions are called from C, they return i32
///// rather than Result.
/////
///// The `fuse_*_in` and `fuse_*_out` and defined in the Linux kernel in
///// /include/uapi/linux/fuse.h.
//    /// Initialize the file system and fill in initialization flags.
//    ///
//    /// Possible initialization flags are defined /include/uapi/linux/fuse.h.
//    /// No support is provided for readdirplus and async DIO.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `in_arg: &fuse_init_in` - Data structure containing init args from Bento.
//    /// * `out_arg: &mut fuse_init_out` - Data structure to be filled.
//
//    /// Perform any necessary cleanup on the file system.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//
//    /// Look up a directory entry by name and get its attributes.
//    ///
//    /// If the entry exists, fill `fuse_entry_out` with the attributes.
//    /// Otherwise, return `-ENOENT`.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - The file system-provided inode number of the parent directory
//    /// * `name: CStr` - The name of the file to lookup.
//
//    /// Forget about an inode
//    ///
//    /// This function is called when the kernel removes an inode from its internal caches.
//    ///
//    /// Inodes with a non-zero lookup count may receive request from Bento even after calls to
//    /// unlink, rmdir or (when overwriting an existing file) rename. Filesystems must handle such
//    /// requests properly and it is recommended to defer removal of the inode until the lookup
//    /// count reaches zero. Calls to unlink, rmdir or rename will be followed closely by forget
//    /// unless the file or directory is open, in which case Bento issues forget only after the
//    /// release or releasedir calls.
//    ///
//    /// Note that if a file system will be exported over NFS the inodes lifetime must extend even
//    /// beyond forget. See the generation field in struct fuse_entry_param above.
//    ///
//    /// On unmount the lookup count for all inodes implicitly drops to zero. It is not guaranteed
//    /// that the file system will receive corresponding forget messages for the affected inodes.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number of the inode to forget.
//    /// * `nlookup: u64` - The number of lookups to forget.
//
//    /// Get file attributes.
//    ///
//    /// If writeback caching is enabled, Bento may have a better idea of a file's length than the
//    /// file system (eg if there has been a write that extended the file size, but that has not
//    /// yet been passed to the filesystem.
//    ///
//    /// In this case, the st_size value provided by the file system will be ignored.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided id of the inode.
//    /// * `in_arg: &fuse_getattr_in` - Data structure containing getattr input arguments.
//    /// * `out_arg: &mut fuse_attr_out` - Data structure to be filled with attribute information.
//
//    /// Set file attributes
//    ///
//    /// In the 'attr' argument only members indicated by the 'to_set' bitmask contain valid values.
//    /// Other members contain undefined values.
//    ///
//    /// Unless FUSE_CAP_HANDLE_KILLPRIV is disabled, this method is expected to reset the setuid
//    /// and setgid bits if the file size or owner is being changed.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided id of the inode.
//    /// * `in_arg: &fuse_setattr_in` - Data structure containing setattr input arguments.
//    /// * `out_arg: &mut fuse_attr_out` - Data structure to be filled with attribute information.
//
//    /// Read symbolic link.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided id of the inode.
//    /// * `buf: &mut MemContainer<kernel::raw::c_uchar>` - Bento-provided buffer for the link name.
//
//    /// Create file node
//    ///
//    /// Create a regular file, character device, block device, fifo or socket node.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number of the parent directory.
//    /// * `in_arg: &fuse_mknod_in` - Data structure containing mknod input arguments.
//    /// * `name: CStr` - Name of the file to be created.
//    /// * `out_arg: &mut fuse_entry_out` - Data structure to be filled with data about the newly
//    /// created file.
//
//    /// Create directory.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number of the parent directory.
//    /// * `in_arg: &fuse_mkdir_in` - Data structure containing mkdir input arguments.
//    /// * `name: CStr` - Name of the directory to be created.
//    /// * `out_arg: &mut fuse_entry_out` - Data structure to be filled with data about the newly
//    /// created directory.
//
//    /// Remove a file.
//    ///
//    /// If the file's inode's lookup count is non-zero, the file system is expected to postpone any
//    /// removal of the inode until the lookup count reaches zero (see description of the forget
//    /// function).
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number of the parent directory.
//    /// * `name: CStr` - Name of the file to be removed.
//
//    /// Remove a directory.
//    ///
//    /// If the file's inode's lookup count is non-zero, the file system is expected to postpone any
//    /// removal of the inode until the lookup count reaches zero (see description of the forget
//    /// function).
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number of the parent directory.
//    /// * `name: CStr` - Name of the directory to be removed.
//
//    /// Create a symbolic link.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number of the parent directory.
//    /// * `name: CStr` - Name of the link to be created.
//    /// * `linkname: CStr` - The contents of the symbolic link.
//    /// * `out_arg: &mut fuse_entry_out` - Data structure to be filled with data about the newly
//    /// created link.
//
//    /// Rename a file
//    ///
//    /// If the target exists it should be atomically replaced. If the target's inode's lookup count
//    /// is non-zero, the file system is expected to postpone any removal of the inode until the
//    /// lookup count reaches zero (see description of the forget function).
//    ///
//    /// If this request is answered with an error code of ENOSYS, this is treated as a permanent
//    /// failure with error code EINVAL, i.e. all future bmap requests will fail with EINVAL without
//    /// being sent to the filesystem.
//    ///
//    /// `flags` in `fuse_rename2_in` may be `RENAME_EXCHANGE` or `RENAME_NOREPLACE`. If
//    /// `RENAME_NOREPLACE` is specified, the filesystem must not overwrite newname if it exists
//    /// and return an error instead. If `RENAME_EXCHANGE` is specified, the filesystem must
//    /// atomically exchange the two files, i.e. both must exist and neither may be deleted.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number of the parent directory.
//    /// * `in_arg: &fuse_rename2_in` - Data structure containing rename input arguments.
//    /// * `oldname: CStr` - Old name of the file.
//    /// * `newname: CStr` - New name of the file.
//
//    /// Open a file.
//    ///
//    /// Open flags are available in `in_arg->flags`. The following rules apply.
//    ///
//    /// Creation (`O_CREAT`, `O_EXCL`, `O_NOCTTY`) flags will be filtered out / handled by Bento.
//    /// Access modes (`O_RDONLY`, `O_WRONLY`, `O_RDWR`) should be used by the filesystem to check
//    /// if the operation is permitted. If the -o default_permissions mount option is given, this
//    /// check is already done by Bento before calling `open()` and may thus be omitted by the
//    /// filesystem.
//    /// When writeback caching is enabled, Bento may send read requests even for files opened with
//    /// `O_WRONLY`. The filesystem should be prepared to handle this.
//    /// When writeback caching is disabled, the filesystem is expected to properly handle the
//    /// `O_APPEND` flag and ensure that each write is appending to the end of the file.
//    /// When writeback caching is enabled, Bento will handle `O_APPEND`. However, unless all changes
//    /// to the file come through Bento this will not work reliably. The filesystem should thus
//    /// either ignore the `O_APPEND` flag (and let Bento handle it), or return an error (indicating
//    /// that reliably `O_APPEND` is not available).
//    /// Filesystem may store an arbitrary file handle (pointer, index, etc) in `out_arg->fh`, and
//    /// use this in other all other file operations (read, write, flush, release, fsync).
//    ///
//    /// Filesystem may also implement stateless file I/O and not store anything in `out_arg->fh`.
//    ///
//    /// There are also some flags (keep_cache) which the filesystem may set in `out_arg`, to change
//    /// the way the file is opened. See `fuse_file_info` structure in <fuse_common.h> for more details.
//    ///
//    /// If this request is answered with an error code of `ENOSYS` and `FUSE_CAP_NO_OPEN_SUPPORT`
//    /// is set in `fuse_conn_info.capable`, this is treated as success and future calls to open and
//    /// release will also succeed without being sent to the filesystem.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number.
//    /// * `in_arg: &fuse_open_in` - Data structure containing open input arguments.
//    /// * `out_arg: &mut fuse_open_out` - Data structure to be filled with open output.
//
//    /// Read data
//    ///
//    /// Read should send exactly the number of bytes requested except on EOF or error, otherwise
//    /// the rest of the data will be substituted with zeroes.
//    ///
//    /// `in_arg->fh` will contain the value set by the open method, or will be undefined if the open
//    /// method didn't set any value.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number.
//    /// * `in_arg: &fuse_read_in` - Data structure containing read input arguments.
//    /// * `buf: &mut MemContainer<kernel::raw::c_uchar` - Buffer to be filled with the read data.
//
//    /// Write data
//    ///
//    /// Write should return exactly the number of bytes requested except on error.
//    ///
//    /// Unless `FUSE_CAP_HANDLE_KILLPRIV` is disabled, this method is expected to reset the setuid
//    /// and setgid bits.
//    ///
//    /// `in_arg->fh` will contain the value set by the open method, or will be undefined if the open
//    /// method didn't set any value.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number.
//    /// * `in_arg: &fuse_write_in` - Data structure containing write input arguments.
//    /// * `buf: &MemContainer<kernel::raw::c_uchar` - Buffer containing the data to write.
//    /// * `out_arg: &mut fuse_write_out` - Data structure to be filled with write output arguments.
//
//    /// Flush method
//    ///
//    /// This is called on each `close()` of the opened file.
//    ///
//    /// Since file descriptors can be duplicated (dup, dup2, fork), for one open call there may be
//    /// many flush calls.
//    ///
//    /// Filesystems shouldn't assume that flush will always be called after some writes, or that if
//    /// will be called at all.
//    ///
//    /// `in_arg->fh` will contain the value set by the open method, or will be undefined if the open
//    /// method didn't set any value.
//    ///
//    /// NOTE: the name of the method is misleading, since (unlike fsync) the filesystem is not
//    /// forced to flush pending writes. One reason to flush data is if the filesystem wants to
//    /// return write errors during close. However, such use is non-portable because POSIX does not
//    /// require close to wait for delayed I/O to complete.
//    ///
//    /// If the filesystem supports file locking operations (setlk, getlk) it should remove all locks
//    /// belonging to `in_arg->lock_owner`.
//    ///
//    /// If this request is answered with an error code of `ENOSYS`, this is treated as success and
//    /// future calls to `flush()` will succeed automatically without being sent to the filesystem.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number.
//    /// * `in_arg: &fuse_flush_in` - Data structure containing flush input arguments.
//
//    /// Release an open file
//    ///
//    /// Release is called when there are no more references to an open file: all file descriptors
//    /// are closed and all memory mappings are unmapped.
//    ///
//    /// For every open call there will be exactly one release call (unless the filesystem is
//    /// force-unmounted).
//    ///
//    /// The filesystem may reply with an error, but error values are not returned to `close()` or
//    /// `munmap()` which triggered the release.
//    ///
//    /// `in_arg->fh` will contain the value set by the open method, or will be undefined if the open
//    /// method didn't set any value. `in_arg->flags` will contain the same flags as for open.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number.
//    /// * `in_arg: &fuse_release_in` - Data structure containing release input arguments.
//
//    /// Synchronize file contents
//    ///
//    /// If the `in_arg->flags` parameter is non-zero, then only the user data should be flushed,
//    /// not the meta data.
//    ///
//    /// If this request is answered with an error code of `ENOSYS`, this is treated as success and
//    /// future calls to `fsync()` will succeed automatically without being sent to the filesystem.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number.
//    /// * `in_arg: &fuse_fsync_in` - Data structure containing fsync input arguments.
//
//    /// Open a directory.
//    ///
//    /// Filesystem may store an arbitrary file handle (pointer, index, etc) in `in_arg->fh`, and use
//    /// this in other all other directory stream operations (readdir, releasedir, fsyncdir).
//    ///
//    /// If this request is answered with an error code of `ENOSYS` and `FUSE_CAP_NO_OPENDIR_SUPPORT`
//    /// is set in `fuse_conn_info.capable`, this is treated as success and future calls to opendir
//    /// and releasedir will also succeed without being sent to the filesystem. In addition,
//    /// Bento will cache readdir results as if opendir returned
//    /// `FOPEN_KEEP_CACHE` | `FOPEN_CACHE_DIR`.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number.
//    /// * `in_arg: &fuse_open_in` - Data structure containing open input arguments.
//    /// * `out_arg: &fuse_open_out` - Data structure to be filled with open information.
//
//    /// Read directory
//    ///
//    /// Send a buffer filled using bento_add_direntry(), with size not exceeding the requested size.
//    /// Send an empty buffer on end of stream.
//    ///
//    /// `in_arg->fh` will contain the value set by the opendir method, or will be undefined if the
//    /// opendir method didn't set any value.
//    ///
//    /// Returning a directory entry from readdir() does not affect its lookup count.
//    ///
//    /// If `in_arg->offset` is non-zero, then it will correspond to one of the `off` values that
//    /// was previously returned by `readdir()` for the same directory handle. In this case,
//    /// `readdir()` should skip over entries coming before the position defined by the
//    /// `in_arg->offset` value. If entries are added or removed while the directory handle is open,
//    /// they filesystem may still include the entries that have been removed, and may not report the
//    /// entries that have been created. However, addition or removal of entries must never cause
//    /// `readdir()` to skip over unrelated entries or to report them more than once. This means that
//    /// `in_arg->offset` can not be a simple index that enumerates the entries that have been
//    /// returned but must contain sufficient information to uniquely determine the next directory
//    /// entry to return even when the set of entries is changing.
//    ///
//    /// The function does not have to report the '.' and '..' entries, but is allowed to do so. Note
//    /// that, if readdir does not return '.' or '..', they will not be implicitly returned, and this
//    /// behavior is observable by the caller.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number of the directory.
//    /// * `in_arg: &fuse_read_in` - Data structure containing read input arguments.
//    /// * `buf: &mut MemContainer<kernel::raw::c_uchar` - Buffer to be filled with the direntry data.
//
//    /// Release an open directory
//    ///
//    /// For every opendir call there will be exactly one releasedir call (unless the filesystem is
//    /// force-unmounted).
//    /// `in_arg->fh` will contain the value set by the opendir method, or will be undefined if the
//    /// opendir method didn't set any value.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number of the directory.
//    /// * `in_arg: &fuse_release_in` - Data structure containing release input arguments.
//
//    /// Synchronize directory contents
//    ///
//    /// If the `in_arg->fsync_flags` parameter is non-zero, then only the directory contents should
//    /// be flushed, not the meta data.
//    ///
//    /// `in_arg->fh` will contain the value set by the opendir method, or will be undefined if the
//    /// opendir method didn't set any value.
//    ///
//    /// If this request is answered with an error code of `ENOSYS`, this is treated as success and
//    /// future calls to `fsyncdir()` will succeed automatically without being sent to the filesystem.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number of the directory.
//    /// * `in_arg: &fuse_fsync_in` - Data structure containing fsync input arguments.
//
//    /// Get filesystem statistics.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number, zero means "undefined".
//    /// * `out_arg: &fuse_statfs_out` - Data structure to be filled with statfs information.
//
//    /// Set an extended attribute
//    ///
//    /// If this request is answered with an error code of `ENOSYS`, this is treated as a permanent
//    /// failure with error code `EOPNOTSUPP`, i.e. all future `setxattr()` requests will fail with
//    /// `EOPNOTSUPP` without being sent to the filesystem.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number.
//    /// * `in_arg: &fuse_setxattr_in_out` - Data structure containing setxattr input arguments.
//    /// * `name: CStr` - The name of the attribute.
//    /// * `buf: &MemContainer<kernel::raw::c_uchar>` - Buffer containing the data to be set in the
//    /// attribute.
//
//    /// Get an extended attribute
//    ///
//    /// If size is zero, the size of the value should be sent in `out_arg->size`.
//    ///
//    /// If the size is non-zero, and the value fits in the buffer, the value should be sent in `buf`.
//    ///
//    /// If the size is too small for the value, the `ERANGE` error should be sent.
//    ///
//    /// If this request is answered with an error code of `ENOSYS`, this is treated as a permanent
//    /// failure with error code `EOPNOTSUPP`, i.e. all future `getxattr()` requests will fail
//    /// with `EOPNOTSUPP` without being sent to the filesystem.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number.
//    /// * `in_arg: &fuse_getxattr_in_out` - Data structure containing getxattr input arguments.
//    /// * `name: CStr` - The name of the attribute.
//    /// * `size: kernel::raw::c_size_t` - The size of the buffer.
//    /// * `out_arg: &mut fuse_getxattr_out` - Data structure to be filled with getxattr output
//    /// information.
//    /// * `buf: &mut MemContainer<kernel::raw::c_uchar>` - Buffer to be filled with the attribute data.
//
//    /// List extended attribute names
//    ///
//    /// If size is zero, the total size of the attribute list should be sent in `fuse_getxattr_out`.
//    ///
//    /// If the size is non-zero, and the null character separated attribute list fits in the buffer,
//    /// the list should be sent in the buffer.
//    ///
//    /// If the size is too small for the list, the `ERANGE` error should be sent.
//    ///
//    /// If this request is answered with an error code of `ENOSYS`, this is treated as a permanent
//    /// failure with error code `EOPNOTSUPP`, i.e. all future `listxattr()` requests will fail with
//    /// `EOPNOTSUPP` without being sent to the filesystem.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number.
//    /// * `in_arg: &fuse_getxattr_in_out` - Data structure containing getxattr input arguments.
//    /// * `name: CStr` - The name of the attribute.
//    /// * `size: kernel::raw::c_size_t` - The size of the buffer.
//    /// * `out_arg: &mut fuse_getxattr_out` - Data structure to be filled with getxattr output
//    /// information.
//    /// * `buf: &mut MemContainer<kernel::raw::c_uchar>` - Buffer to be filled with the attribute data.
//
//    /// Remove an extended attribute
//    ///
//    /// If this request is answered with an error code of `ENOSYS`, this is treated as a permanent
//    /// failure with error code `EOPNOTSUPP`, i.e. all future `removexattr()` requests will fail
//    /// with `EOPNOTSUPP` without being sent to the filesystem.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number.
//    /// * `name: CStr` - The name of the attribute to remove.
//
//    /// Check file access permissions
//    ///
//    /// This will be called for the `access()` and `chdir()` system calls. If the
//    /// 'default_permissions' mount option is given, this method is not called.
//    ///
//    /// If this request is answered with an error code of `ENOSYS`, this is treated as a permanent
//    /// success, i.e. this and all future `access()` requests will succeed without being sent to
//    /// the filesystem.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number.
//    /// * `in_arg: &fuse_access_in` - Data structure containing access input arguments.
//
//    /// Create and open a file
//    ///
//    /// If the file does not exist, first create it with the specified mode, and then open it.
//    ///
//    /// See the description of the open handler for more information.
//    ///
//    /// If this method is not implemented, the mknod() and open() methods will be called instead.
//    ///
//    /// If this request is answered with an error code of `ENOSYS`, the handler is treated as not
//    /// implemented (i.e., for this and future requests the mknod() and open() handlers will be
//    /// called instead).
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number of the parent directory.
//    /// * `in_arg: &fuse_create_in` - Data structure containing create input arguments.
//    /// * `name: CStr` - The name of the new file.
//    /// * `out_entry: &mut fuse_entry_out` - Data structure to be filled with information about the
//    /// newly created entry.
//    /// * `out_open: &mut fuse_open_out` - Data structure to be filled with information about the
//    /// newly opened file.
//
//    /// Test for a POSIX file lock.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number.
//    /// * `in_arg: &fuse_lk_in` - Data structure containing getlk input arguments.
//    /// * `out_arg: &mut fuse_lk_out` - Data structure to be filled with getlk output information.
//
//    /// Acquire, modify or release a POSIX file lock
//    ///
//    /// For POSIX threads (NPTL) there's a 1-1 relation between pid and owner, but otherwise this is
//    /// not always the case. For checking lock ownership, `in_rg->owner` must be used. The `pid`
//    /// field in `struct fuse_file_lock` should only be used to fill in this field in `getlk()`.
//    ///
//    /// Note: if the locking methods are not implemented, the kernel will still allow file locking
//    /// to work locally. Hence these are only interesting for network filesystems and similar.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number.
//    /// * `in_arg: &fuse_lk_in` - Data structure containing setlk input arguments.
//    /// * `sleep: bool` - Should the filesystem sleep.
//
//    /// Map block index within file to block index within device
//    ///
//    /// Note: This makes sense only for block device backed filesystems mounted with the 'blkdev'
//    /// option.
//    ///
//    /// If this request is answered with an error code of `ENOSYS`, this is treated as a permanent
//    /// failure, i.e. all future `bmap()` requests will fail with the same error code without being
//    /// sent to the filesystem.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number.
//    /// * `in_arg: &fuse_bmap_in` - Data structure containing bmap input arguments.
//    /// * `out_arg: &mut fuse_bmap_out` - Data structure to be filled with bmap output information.
//
//    /// Ioctl
//    ///
//    /// Note: For unrestricted ioctls, data in and out areas can be discovered by giving iovs and
//    /// setting `FUSE_IOCTL_RETRY` in flags. For restricted ioctls, Bento prepares in/out data area
//    /// according to the information encoded in `cmd`.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number.
//    /// * `in_arg: &fuse_ioctl_in` - Data structure containing ioctl input arguments.
//    /// * `out_arg: &mut fuse_ioctl_out` - Data structure to be filled with ioctl output information.
//    /// * `buf: &mut MemContainer<kernel::raw::c_uchar>` - Buffer to be filled with ioctl output
//    /// information.
//
//    /// Poll for IO readiness
//    ///
//    /// If this request is answered with an error code of `ENOSYS`, this is treated as success
//    /// (with a kernel-defined default poll-mask) and future calls to `poll()` will succeed the
//    /// same way without being sent to the filesystem.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number.
//    /// * `in_arg: &fuse_poll_in` - Data structure containing poll input arguments.
//    /// * `out_arg: &mut fuse_poll_out` - Data structure to be filled with poll output information.
//
//    /// Allocate requested space. If this function returns success then subsequent writes to the
//    /// specified range shall not fail due to the lack of free space on the file system storage
//    /// media.
//    ///
//    /// If this request is answered with an error code of `ENOSYS`, this is treated as a permanent
//    /// failure with error code `EOPNOTSUPP`, i.e. all future `fallocate()` requests will fail with
//    /// `EOPNOTSUPP` without being sent to the filesystem.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number.
//    /// * `in_arg: &fuse_fallocate_in` - Data structure containing fallocate input arguments.
//
//    /// Find next data or hole after the specified offset
//    ///
//    /// If this request is answered with an error code of `ENOSYS`, this is treated as a permanent
//    /// failure, i.e. all future `lseek()` requests will fail with the same error code without being
//    /// sent to the filesystem.
//    ///
//    /// Arguments:
//    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
//    /// * `nodeid: u64` - Filesystem-provided inode number.
//    /// * `in_arg: &fuse_lseek_in` - Data structure containing lseek input arguments.
//    /// * `out_arg: &mut fuse_lseek_out` - Data structure to be filled with lseek output information.
