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

/// Filesystem trait
///
/// This trait is derived from the Filesystem trait from the fuse Rust crate.
///
/// This trait must be implemented to provide a Bento filesystem. The methods
/// correspond to the `fuse_lowlevel_ops` in libfuse. The user must provide a
/// name for the file system. Otherwise, default implementations are
/// provided here to get a mountable filesystem that does nothing.
pub trait Filesystem {
    /// Get the name of the file system.
    ///
    /// This must be provided to mount the filesystem.
    fn get_name(&self) -> &str;

    /// Register the filesystem with Bento.
    ///
    /// This should be called when the filesystem module is inserted and before
    /// a filesystem is mounted.
    fn register(&self) -> i32
    where
        Self: core::marker::Sized,
    {
        return unsafe {
            register_bento_fs(
                self as *const Self as *const raw::c_void,
                self.get_name().as_bytes().as_ptr() as *const raw::c_void,
                dispatch::<Self> as *const raw::c_void,
            )
        };
    }

    /// Unregister the filesystem with Bento.
    ///
    /// This should be called when the filesystem module is removed and after a filesystem is
    /// unmounted.
    fn unregister(&self) -> i32 {
        return unsafe {
            unregister_bento_fs(self.get_name().as_bytes().as_ptr() as *const raw::c_void)
        };
    }

    /// Initialize the file system and fill in initialization flags.
    ///
    /// Possible initialization flags are defined /include/uapi/linux/fuse.h.
    /// No support is provided for readdirplus and async DIO.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `fc_info: &mut FuseConnInfo` - Connection information used to pass initialization
    /// arguments to Bento.
    fn init(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _fc_info: &mut FuseConnInfo,
    ) -> Result<(), i32> {
        return Err(-(ENOSYS as i32));
    }

    /// Perform any necessary cleanup on the file system.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    fn destroy(&mut self, _sb: RsSuperBlock, _req: &Request) -> Result<(), i32> {
        return Ok(());
    }

    /// Lookup a directory entry by name and get its attributes.
    ///
    /// If the entry exists, fill `reply` with the attributes.
    /// Otherwise, return `-ENOENT`.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `parent: u64` - The file system-provided inode number of the parent directory
    /// * `name: CStr` - The name of the file to lookup.
    /// * `reply: ReplyEntry` - Output data structure for the entry data or error vaule.
    fn lookup(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _parent: u64,
        _name: CStr,
        reply: ReplyEntry,
    ) {
        reply.error(-(ENOSYS as i32));
    }

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
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided inode number of the inode to forget.
    /// * `nlookup: u64` - The number of lookups to forget.
    fn forget(&mut self, _sb: RsSuperBlock, _req: &Request, _ino: u64, _nlookup: u64) {}

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
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided id of the inode.
    /// * `reply: ReplyAttr` - Output data structure for the attribute data or error value.
    fn getattr(&mut self, _sb: RsSuperBlock, _req: &Request, _ino: u64, reply: ReplyAttr) {
        reply.error(-(ENOSYS as i32));
    }

    /// Set file attributes
    ///
    /// Unless FUSE_CAP_HANDLE_KILLPRIV is disabled, this method is expected to reset the setuid
    /// and setgid bits if the file size or owner is being changed.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided id of the inode.
    /// * `mode: Option<u32>` - Attribute mode to set if provided, otherwise None.
    /// * `uid: Option<u32>` - Attribute uid to set if provided, otherwise None.
    /// * `gid: Option<u32>` - Attribute gid to set if provided, otherwise None.
    /// * `size: Option<u64>` - Attribute size to set if provided, otherwise None.
    /// * `atime: Option<Timespec>` - Attribute accessed time to set if provided, otherwise None.
    /// * `mtime: Option<Timespec>` - Attribute modified time to set if provided, otherwise None.
    /// * `fh: Option<u64>` - Attribute file handle to set if provided, otherwise None.
    /// * `crtime: Option<Timespec>` - Unused.
    /// * `chgtime: Option<Timespec>` - Unused.
    /// * `bkuptime: Option<Timespec>` - Unused.
    /// * `flags: Option<u32>` - Unused.
    /// * `reply: ReplyAttr` - Output data structure for the attribute data or error value.
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
        reply: ReplyAttr,
    ) {
        reply.error(-(ENOSYS as i32));
    }

    /// Read symbolic link.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided id of the inode.
    /// * `reply: ReplyData` - Output data structure for the read link data or error value.
    fn readlink(&self, _sb: RsSuperBlock, _req: &Request, _ino: u64, reply: ReplyData) {
        return reply.error(-(ENOSYS as i32));
    }

    /// Create file node
    ///
    /// Create a regular file, character device, block device, fifo or socket node.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `parent: u64` - Filesystem-provided inode number of the parent directory.
    /// * `name: CStr` - Name of the file to be created.
    /// * `mode: u32` - File creation mode. Specifies both the file mode and the type of node.
    /// * `rdev: u32` - Device number. Used if file mode is `S_IFCHR` of `S_IFBLK`.
    /// * `reply: ReplyEntry` - Output data structure for the entry data or error value.
    fn mknod(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _parent: u64,
        _name: CStr, //&OsStr
        _mode: u32,
        _rdev: u32,
        reply: ReplyEntry,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    /// Create directory.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `parent: u64` - Filesystem-provided inode number of the parent directory.
    /// * `name: CStr` - Name of the directory to be created.
    /// * `mode: u32` - Mode of the directory to be created.
    /// * `reply: ReplyEntry` - Output data structure for the entry data or error value.
    fn mkdir(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _parent: u64,
        _name: CStr, //&OsStr
        _mode: u32,
        reply: ReplyEntry,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    /// Remove a file.
    ///
    /// If the file's inode's lookup count is non-zero, the file system is expected to postpone any
    /// removal of the inode until the lookup count reaches zero (see description of the forget
    /// function).
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `parent: u64` - Filesystem-provided inode number of the parent directory.
    /// * `name: CStr` - Name of the file to be removed.
    /// * `reply: ReplyEmpty` - Output data structure for a possible error value.
    fn unlink(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _parent: u64,
        _name: CStr, //&OsStr
        reply: ReplyEmpty,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    /// Remove a directory.
    ///
    /// If the file's inode's lookup count is non-zero, the file system is expected to postpone any
    /// removal of the inode until the lookup count reaches zero (see description of the forget
    /// function).
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `parent: u64` - Filesystem-provided inode number of the parent directory.
    /// * `name: CStr` - Name of the file to be removed.
    /// * `reply: ReplyEmpty` - Output data structure for a possible error value.
    fn rmdir(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _parent: u64,
        _name: CStr, //&OsStr
        reply: ReplyEmpty,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    /// Create a symbolic link.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `parent: u64` - Filesystem-provided inode number of the parent directory.
    /// * `name: CStr` - Name of the file to be removed.
    /// * `linkname: CStr` - The contents of the symbolic link.
    /// * `reply: ReplyEntry` - Output data structure for the entry data or error value.
    fn symlink(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _parent: u64,
        _name: CStr, // &OsStr
        _link: CStr, // & Path
        reply: ReplyEntry,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

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
    /// * `req: &Request` - Request data structure.
    /// * `parent: u64` - Filesystem-provided inode number of the parent directory.
    /// * `name: CStr` - Name of the file to be removed.
    /// * `newparent: u64` - Filesystem-provided inode number of the new parent directory.
    /// * `newname: CStr` - New name of the file.
    /// * `reply: ReplyEmpty` - Output data structure for a possible error value.
    fn rename(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _parent: u64,
        _name: CStr, //&OsStr
        _newparent: u64,
        _newname: CStr, //&OsStr,
        reply: ReplyEmpty,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    /// Create a hard link.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided inode number of the old node.
    /// * `newparent: u64` - Filesystem-provided inode number of the new parent directory.
    /// * `newname: CStr` - New name of the file to create.
    /// * `reply: ReplyEntry` - Output data structure for the entry data or error value.
    fn link(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _newparent: u64,
        _newname: CStr, // &OsStr
        reply: ReplyEntry,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    /// Open a file.
    ///
    /// Open flags are available in `flags`. The following rules apply.
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
    /// Filesystem may store an arbitrary file handle (pointer, index, etc) in `fh` for `reply`, and
    /// use this in other all other file operations (read, write, flush, release, fsync).
    ///
    /// Filesystem may also implement stateless file I/O and not store anything in `fh`.
    ///
    /// There are also some flags (keep_cache) which the filesystem may set in `reply`, to change
    /// the way the file is opened. See `fuse_file_info` structure in <fuse_common.h> for more details.
    ///
    /// If this request is answered with an error code of `ENOSYS` and `FUSE_CAP_NO_OPEN_SUPPORT`
    /// is set in `fuse_conn_info.capable`, this is treated as success and future calls to open and
    /// release will also succeed without being sent to the filesystem.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided inode number of the file to open.
    /// * `flags: u32` - Open flags.
    /// * `reply: ReplyOpen` - Output data structure for the opened file data or error value.
    fn open(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _flags: u32,
        reply: ReplyOpen,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    /// Read data
    ///
    /// Read should send exactly the number of bytes requested except on EOF or error, otherwise
    /// the rest of the data will be substituted with zeroes.
    ///
    /// `fh` will contain the value set by the open method, or will be undefined if the open
    /// method didn't set any value.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided inode number.
    /// * `fh: u64` - Filesystem-provided file handle.
    /// * `offset: i64` - Offset to read into the file.
    /// * `size: u32` - Size of the data to read.
    /// * `reply: ReplyData` - Output data structure for the read data or error value.
    fn read(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        _size: u32,
        reply: ReplyData,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    /// Write data
    ///
    /// Write should return exactly the number of bytes requested except on error.
    ///
    /// Unless `FUSE_CAP_HANDLE_KILLPRIV` is disabled, this method is expected to reset the setuid
    /// and setgid bits.
    ///
    /// `fh` will contain the value set by the open method, or will be undefined if the open
    /// method didn't set any value.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided inode number.
    /// * `fh: u64` - Filesystem-provided file handle.
    /// * `offset: i64` - Offset into the file to write at.
    /// * `data: &[u8]` - Data to write.
    /// * `flags: u32` - Write flags.
    /// * `reply: ReplyWrite` - Output data structure for the write size or error value.
    fn write(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        _data: &[u8],
        _flags: u32,
        reply: ReplyWrite,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

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
    /// `fh` will contain the value set by the open method, or will be undefined if the open
    /// method didn't set any value.
    ///
    /// NOTE: the name of the method is misleading, since (unlike fsync) the filesystem is not
    /// forced to flush pending writes. One reason to flush data is if the filesystem wants to
    /// return write errors during close. However, such use is non-portable because POSIX does not
    /// require close to wait for delayed I/O to complete.
    ///
    /// If the filesystem supports file locking operations (setlk, getlk) it should remove all locks
    /// belonging to `lock_owner`.
    ///
    /// If this request is answered with an error code of `ENOSYS`, this is treated as success and
    /// future calls to `flush()` will succeed automatically without being sent to the filesystem.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided inode number.
    /// * `fh: u64` - Filesystem-provided file handle.
    /// * `lock_owner: u64` - Lock owner for removing locks on files.
    /// * `reply: ReplyEmpty` - Output data structure for a possible error value.
    fn flush(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _lock_owner: u64,
        reply: ReplyEmpty,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

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
    /// `fh` will contain the value set by the open method, or will be undefined if the open
    /// method didn't set any value. `flags` will contain the same flags as for open.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided inode number.
    /// * `fh: u64` - Filesystem-provided file handle.
    /// * `flags: u32` - Flags from open.
    /// * `lock_owner: u64` - Lock owner for removing locks on files.
    /// * `flush: bool` - Indicates if release should flush. Always false.
    /// * `reply: ReplyEmpty` - Output data structure for a possible error value.
    fn release(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _flags: u32,
        _lock_owner: u64,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    /// Synchronize file contents
    ///
    /// If the `flags` parameter is non-zero, then only the user data should be flushed,
    /// not the meta data.
    ///
    /// If this request is answered with an error code of `ENOSYS`, this is treated as success and
    /// future calls to `fsync()` will succeed automatically without being sent to the filesystem.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided inode number.
    /// * `fh: u64` - Filesystem-provided file handle.
    /// * `datasync: bool` - Indicates if data should be flushed as well as metadata.
    /// * `reply: ReplyEmpty` - Output data structure for a possible error value.
    fn fsync(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    /// Open a directory.
    ///
    /// Filesystem may store an arbitrary file handle (pointer, index, etc) in `fh`, and use
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
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided inode number.
    /// * `flags: u32` - Open flags.
    /// * `reply: ReplyOpen` - Output data structure for the opened file data or error value.
    fn opendir(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _flags: u32,
        reply: ReplyOpen,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    /// Read directory
    ///
    /// Send a buffer filled using bento_add_direntry(), with size not exceeding the requested size.
    /// Send an empty buffer on end of stream.
    ///
    /// `fh` will contain the value set by the opendir method, or will be undefined if the
    /// opendir method didn't set any value.
    ///
    /// Returning a directory entry from readdir() does not affect its lookup count.
    ///
    /// If `offset` is non-zero, then it will correspond to one of the `off` values that
    /// was previously returned by `readdir()` for the same directory handle. In this case,
    /// `readdir()` should skip over entries coming before the position defined by the
    /// `offset` value. If entries are added or removed while the directory handle is open,
    /// they filesystem may still include the entries that have been removed, and may not report the
    /// entries that have been created. However, addition or removal of entries must never cause
    /// `readdir()` to skip over unrelated entries or to report them more than once. This means that
    /// `offset` can not be a simple index that enumerates the entries that have been
    /// returned but must contain sufficient information to uniquely determine the next directory
    /// entry to return even when the set of entries is changing.
    ///
    /// The function does not have to report the '.' and '..' entries, but is allowed to do so. Note
    /// that, if readdir does not return '.' or '..', they will not be implicitly returned, and this
    /// behavior is observable by the caller.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided inode number.
    /// * `fh: u64` - Filesystem-provided file handle.
    /// * `offset: i64` - Offset into the directory. From the offset values used in previous
    /// readdir requests.
    /// * `reply: ReplyDirectory` - Output data structure for the read directory information or
    /// error value.
    fn readdir(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        reply: ReplyDirectory,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    /// Release an open directory
    ///
    /// For every opendir call there will be exactly one releasedir call (unless the filesystem is
    /// force-unmounted).
    /// `fh` will contain the value set by the opendir method, or will be undefined if the
    /// opendir method didn't set any value.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided inode number.
    /// * `fh: u64` - Filesystem-provided file handle.
    /// * `flags: u32` - Open flags.
    /// * `reply: ReplyEmpty` - Output data structure for a possible error value.
    fn releasedir(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _flags: u32,
        reply: ReplyEmpty,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    /// Synchronize directory contents
    ///
    /// If the `fsync_flags` parameter is non-zero, then only the directory contents should
    /// be flushed, not the meta data.
    ///
    /// `fh` will contain the value set by the opendir method, or will be undefined if the
    /// opendir method didn't set any value.
    ///
    /// If this request is answered with an error code of `ENOSYS`, this is treated as success and
    /// future calls to `fsyncdir()` will succeed automatically without being sent to the filesystem.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided inode number.
    /// * `fh: u64` - Filesystem-provided file handle.
    /// * `datasync: bool` - Indicates if data should be flushed as well as metadata.
    /// * `reply: ReplyEmpty` - Output data structure for a possible error value.
    fn fsyncdir(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    /// Get filesystem statistics.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided inode number, zero means "undefined".
    /// * `reply: ReplyStatfs` - Output data structure for file system stat data or error value.
    fn statfs(&mut self, _sb: RsSuperBlock, _req: &Request, _ino: u64, reply: ReplyStatfs) {
        return reply.error(-(ENOSYS as i32));
    }

    /// Set an extended attribute
    ///
    /// If this request is answered with an error code of `ENOSYS`, this is treated as a permanent
    /// failure with error code `EOPNOTSUPP`, i.e. all future `setxattr()` requests will fail with
    /// `EOPNOTSUPP` without being sent to the filesystem.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided inode number.
    /// * `name: CStr` - Name of the extended attribute.
    /// * `value: &[u8]` - Value to set the attribute to.
    /// * `flags: u32` - Set extended attribute flags.
    /// * `position: u32` - Size of the extended attribute value.
    /// * `reply: ReplyEmpty` - Output data structure for a possible error value.
    fn setxattr(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _name: CStr, //&OsStr,
        _value: &[u8],
        _flags: u32,
        _position: u32,
        reply: ReplyEmpty,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    /// Get an extended attribute
    ///
    /// If size is zero, the size of the value should be sent in `reply`.
    ///
    /// If the size is non-zero, and the value fits in the buffer, the value should be sent in `reply`.
    ///
    /// If the size is too small for the value, the `ERANGE` error should be sent.
    ///
    /// If this request is answered with an error code of `ENOSYS`, this is treated as a permanent
    /// failure with error code `EOPNOTSUPP`, i.e. all future `getxattr()` requests will fail
    /// with `EOPNOTSUPP` without being sent to the filesystem.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided inode number.
    /// * `name: CStr` - The name of the attribute.
    /// * `size: u32` - The size of the buffer to write xattr data into.
    /// * `reply: ReplyXattr` - Output data structure for the xattr data.
    fn getxattr(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _name: CStr, //&OsStr,
        _size: u32,
        reply: ReplyXattr,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    /// List extended attribute names
    ///
    /// If size is zero, the total size of the attribute list should be sent in `reply`.
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
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided inode number.
    /// * `size: u32` - The size of the buffer to write xattr data into.
    /// * `reply: ReplyXattr` - Output data structure for the xattr data.
    fn listxattr(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _size: u32,
        reply: ReplyXattr,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    /// Remove an extended attribute
    ///
    /// If this request is answered with an error code of `ENOSYS`, this is treated as a permanent
    /// failure with error code `EOPNOTSUPP`, i.e. all future `removexattr()` requests will fail
    /// with `EOPNOTSUPP` without being sent to the filesystem.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided inode number.
    /// * `name: CStr` - The name of the attribute to remove.
    /// * `reply: ReplyEmpty` - Output data structure for a possible error value.
    fn removexattr(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _name: CStr, //&OsStr,
        reply: ReplyEmpty,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

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
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided inode number.
    /// * `mask: u32` - Access mask.
    /// * `reply: ReplyEmpty` - Output data structure for a possible error value.
    fn access(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _mask: u32,
        reply: ReplyEmpty,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

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
    /// * `req: &Request` - Request data structure.
    /// * `parent: u64` - Filesystem-provided inode number of the parent directory.
    /// * `name: CStr` - The name of the new file.
    /// * `mode: u32` - Create mode.
    /// * `flags: u32` - Open flags.
    /// * `reply: ReplyCreate` - Output data structure for entry and open data or error value.
    fn create(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _parent: u64,
        _name: CStr, //&OsStr,
        _mode: u32,
        _flags: u32,
        reply: ReplyCreate,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

    /// Test for a POSIX file lock.
    ///
    /// Arguments:
    /// * `sb: RsSuperBlock` - Kernel `super_block` for disk accesses.
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided inode number.
    /// * `fh: u64` - Filesystem-provided file handle.
    /// * `lock_owner: u64` - Lock owner to test for.
    /// * `start: u64` - FUSE file lock start.
    /// * `end: u64` - FUSE file lock end.
    /// * `typ: u32` - FUSE file lock type.
    /// * `pid: u32` - FUSE file lock pid.
    /// * `reply: ReplyLock` - Output data structure for lock data or error value.
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
        reply: ReplyLock,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

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
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided inode number.
    /// * `fh: u64` - Filesystem-provided file handle.
    /// * `lock_owner: u64` - Lock owner to test for.
    /// * `start: u64` - FUSE file lock start.
    /// * `end: u64` - FUSE file lock end.
    /// * `typ: u32` - FUSE file lock type.
    /// * `pid: u32` - FUSE file lock pid.
    /// * `sleep: bool` - Should the filesystem sleep.
    /// * `reply: ReplyEmpty` - Output data structure for a possible error value.
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
        reply: ReplyEmpty,
    ) {
        return reply.error(-(ENOSYS as i32));
    }

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
    /// * `req: &Request` - Request data structure.
    /// * `ino: u64` - Filesystem-provided inode number.
    /// * `blocksize: u32` - Blocksize.
    /// * `idx: u64` - Block index.
    /// * `reply: ReplyBmap` - Output data structure for bmap data or error value.
    fn bmap(
        &mut self,
        _sb: RsSuperBlock,
        _req: &Request,
        _ino: u64,
        _blocksize: u32,
        _idx: u64,
        reply: ReplyBmap,
    ) {
        return reply.error(-(ENOSYS as i32));
    }
}
