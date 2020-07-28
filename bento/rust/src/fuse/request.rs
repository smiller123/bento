use alloc::boxed::Box;
use core::str;

use crate::libc;

use crate::std::ffi::OsStr;
use crate::std::path::Path;

use kernel::kobj::*;
use kernel::mem::*;
use kernel::raw;
use crate::time::Timespec;

use fuse::reply::*;
use crate::bento_utils::BentoFilesystem;
use fuse::internal::*;

use serde::{Serialize, Deserialize};

const BENTO_UPDATE_PREPARE: u32 = 8192;
const BENTO_UPDATE_TRANSFER: u32 = 8193;

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
    reserved: [u32; 22],
}

impl FuseConnInfo {
    fn from_init_in(inarg: &bento_init_in) -> Self {
        let mut me: Self = Default::default();
        me.proto_major = inarg.major;
        me.proto_minor = inarg.minor;
        me.max_readahead = inarg.max_readahead;
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
    pub h: &'a fuse_in_header,
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

#[repr(C)]
struct bento_init_in {
    major: u32,
    minor: u32,
    max_readahead: u32,
    flags: u32,
    devname: CStr,
}

pub fn dispatch<'de, TransferIn: Send + Deserialize<'de>, TransferOut: Send + Serialize, T: BentoFilesystem<'de, TransferIn, TransferOut>>(
    fs: &'static mut T,
    opcode: fuse_opcode,
    inarg: &bento_in,
    outarg: &mut bento_out,
) -> i32 {
    match opcode {
        fuse_opcode_FUSE_INIT => {
            if inarg.numargs != 1 || outarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };

            let init_in = unsafe { &*(inarg.args[0].value as *const bento_init_in) };
            let init_out = unsafe { &mut *(outarg.args[0].value as *mut fuse_init_out) };
            let mut fc_info = FuseConnInfo::from_init_in(&init_in);
            let devname_str = if init_in.devname.to_raw().is_null() {
                ""
            } else {
                str::from_utf8(init_in.devname.to_bytes_with_nul()).unwrap()
            };
            let devname = OsStr::new(devname_str);
            match fs.bento_init(&req, devname, &mut fc_info) {
                Ok(()) => {
                    fc_info.to_init_out(init_out);
                    0
                }
                Err(x) => x as i32,
            }
        }
        fuse_opcode_FUSE_DESTROY => {
            let req = Request { h: &inarg.h };
            fs.bento_destroy(&req);
            0
        }
        fuse_opcode_FUSE_LOOKUP => {
            if inarg.numargs != 1 || outarg.numargs != 1 {
                return -1;
            }
            let req = Request { h: &inarg.h };
            let name = unsafe { CStr::from_raw(inarg.args[0].value as *const raw::c_char) };
            let entry_out = unsafe { &mut *(outarg.args[0].value as *mut fuse_entry_out) };
            let mut reply = ReplyEntryInternal {
                reply: Ok(entry_out),
            };
            let name_str = OsStr::new(str::from_utf8(name.to_bytes_with_nul()).unwrap());
            fs.bento_lookup(&req, inarg.h.nodeid, name_str, &mut reply);
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_FORGET => {
            if inarg.numargs != 1 || outarg.numargs != 0 {
                return -1;
            }
            let req = Request { h: &inarg.h };
            let forget_in = unsafe { &*(inarg.args[0].value as *const fuse_forget_in) };

            fs.bento_forget(&req, inarg.h.nodeid, forget_in.nlookup);
            0
        }
        fuse_opcode_FUSE_GETATTR => {
            if inarg.numargs != 1 || outarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };

            let _getattr_in = unsafe { &*(inarg.args[0].value as *const fuse_getattr_in) };
            let getattr_out = unsafe { &mut *(outarg.args[0].value as *mut fuse_attr_out) };
            let mut reply = ReplyAttrInternal {
                reply: Ok(getattr_out),
            };
            fs.bento_getattr(&req, inarg.h.nodeid, &mut reply);
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_SETATTR => {
            if inarg.numargs != 1 || outarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };

            let setattr_in = unsafe { &*(inarg.args[0].value as *const fuse_setattr_in) };
            let setattr_out = unsafe { &mut *(outarg.args[0].value as *mut fuse_attr_out) };
            let mut reply = ReplyAttrInternal {
                reply: Ok(setattr_out),
            };
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
            fs.bento_setattr(
                &req,
                inarg.h.nodeid,
                mode,
                uid,
                gid,
                size,
                atime,
                mtime,
                fh,
                None,
                None,
                None,
                None,
                &mut reply,
            );
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_READLINK => {
            if outarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };
            let data_out =
                unsafe { &mut *(outarg.args[0].value as *mut MemContainer<raw::c_uchar>) };
            let mut reply = ReplyDataInternal {
                reply: Ok(data_out),
            };
            fs.bento_readlink(&req, inarg.h.nodeid, &mut reply);
            match reply.reply() {
                Ok(buf) => {
                    let buf_slice = buf.to_slice();
                    let buf_str = str::from_utf8(buf_slice).unwrap_or("");
                    buf_str.len() as i32
                }
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_MKNOD => {
            if inarg.numargs != 2 || outarg.numargs != 1 {
                return -1;
            }
            let req = Request { h: &inarg.h };
            let mknod_in = unsafe { &*(inarg.args[0].value as *const fuse_mknod_in) };
            let name = unsafe { CStr::from_raw(inarg.args[1].value as *const raw::c_char) };
            let name_str = OsStr::new(str::from_utf8(name.to_bytes_with_nul()).unwrap());
            let entry_out = unsafe { &mut *(outarg.args[0].value as *mut fuse_entry_out) };
            let mut reply = ReplyEntryInternal {
                reply: Ok(entry_out),
            };
            fs.bento_mknod(
                &req,
                inarg.h.nodeid,
                name_str,
                mknod_in.mode,
                mknod_in.rdev,
                &mut reply,
            );
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_MKDIR => {
            if inarg.numargs != 2 || outarg.numargs != 1 {
                return -1;
            }
            let req = Request { h: &inarg.h };
            let mkdir_in = unsafe { &*(inarg.args[0].value as *const fuse_mkdir_in) };
            let name = unsafe { CStr::from_raw(inarg.args[1].value as *const raw::c_char) };
            let name_str = OsStr::new(str::from_utf8(name.to_bytes_with_nul()).unwrap());
            let entry_out = unsafe { &mut *(outarg.args[0].value as *mut fuse_entry_out) };
            let mut reply = ReplyEntryInternal {
                reply: Ok(entry_out),
            };
            fs.bento_mkdir(&req, inarg.h.nodeid, name_str, mkdir_in.mode, &mut reply);
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_UNLINK => {
            if inarg.numargs != 1 {
                return -1;
            }
            let req = Request { h: &inarg.h };
            let name = unsafe { CStr::from_raw(inarg.args[0].value as *const raw::c_char) };
            let name_str = OsStr::new(str::from_utf8(name.to_bytes_with_nul()).unwrap());
            let mut reply = ReplyEmptyInternal {
                reply: Err(libc::ENOSYS),
            };
            fs.bento_unlink(&req, inarg.h.nodeid, name_str, &mut reply);
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_RMDIR => {
            if inarg.numargs != 1 {
                return -1;
            }
            let req = Request { h: &inarg.h };
            let name = unsafe { CStr::from_raw(inarg.args[0].value as *const raw::c_char) };
            let name_str = OsStr::new(str::from_utf8(name.to_bytes_with_nul()).unwrap());
            let mut reply = ReplyEmptyInternal {
                reply: Err(libc::ENOSYS),
            };
            fs.bento_rmdir(&req, inarg.h.nodeid, name_str, &mut reply);
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_SYMLINK => {
            if inarg.numargs != 2 || outarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };
            let name = unsafe { CStr::from_raw(inarg.args[0].value as *const raw::c_char) };
            let name_str = OsStr::new(str::from_utf8(name.to_bytes_with_nul()).unwrap());
            let link = unsafe { CStr::from_raw(inarg.args[1].value as *const raw::c_char) };
            let link_path = Path::new(str::from_utf8(link.to_bytes_with_nul()).unwrap());
            let entry_out = unsafe { &mut *(outarg.args[0].value as *mut fuse_entry_out) };
            let mut reply = ReplyEntryInternal {
                reply: Ok(entry_out),
            };
            fs.bento_symlink(&req, inarg.h.nodeid, name_str, link_path, &mut reply);
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_RENAME | fuse_opcode_FUSE_RENAME2 => {
            if inarg.numargs != 3 {
                return -1;
            }
            let req = Request { h: &inarg.h };
            let rename_in = unsafe { &*(inarg.args[0].value as *const fuse_rename2_in) };
            let oldname = unsafe { CStr::from_raw(inarg.args[1].value as *const raw::c_char) };
            let oldname_str = OsStr::new(str::from_utf8(oldname.to_bytes_with_nul()).unwrap());
            let newname = unsafe { CStr::from_raw(inarg.args[2].value as *const raw::c_char) };
            let newname_str = OsStr::new(str::from_utf8(newname.to_bytes_with_nul()).unwrap());
            let mut reply = ReplyEmptyInternal {
                reply: Err(libc::ENOSYS),
            };
            fs.bento_rename(
                &req,
                inarg.h.nodeid,
                oldname_str,
                rename_in.newdir,
                newname_str,
                &mut reply,
            );
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_LINK => {
            if inarg.numargs != 2 || outarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };
            let link_in = unsafe { &*(inarg.args[0].value as *const fuse_link_in) };
            let name = unsafe { CStr::from_raw(inarg.args[1].value as *const raw::c_char) };
            let name_str = OsStr::new(str::from_utf8(name.to_bytes_with_nul()).unwrap());
            let entry_out = unsafe { &mut *(outarg.args[0].value as *mut fuse_entry_out) };
            let mut reply = ReplyEntryInternal {
                reply: Ok(entry_out),
            };
            fs.bento_link(
                &req,
                link_in.oldnodeid,
                inarg.h.nodeid,
                name_str,
                &mut reply,
            );
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_OPEN => {
            if inarg.numargs != 1 || outarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };

            let open_in = unsafe { &*(inarg.args[0].value as *const fuse_open_in) };
            let open_out = unsafe { &mut *(outarg.args[0].value as *mut fuse_open_out) };
            let mut reply = ReplyOpenInternal {
                reply: Ok(open_out),
            };
            fs.bento_open(&req, inarg.h.nodeid, open_in.flags, &mut reply);
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_READ => {
            if inarg.numargs != 1 || outarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };
            let read_in = unsafe { &*(inarg.args[0].value as *const fuse_read_in) };
            let data_out =
                unsafe { &mut *(outarg.args[0].value as *mut MemContainer<raw::c_uchar>) };
            let mut reply = ReplyDataInternal {
                reply: Ok(data_out),
            };
            fs.bento_read(
                &req,
                inarg.h.nodeid,
                read_in.fh,
                read_in.offset as i64,
                read_in.size,
                &mut reply,
            );
            match reply.reply() {
                Ok(buf) => buf.len() as i32,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_WRITE => {
            if inarg.numargs != 2 || outarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };
            let write_in = unsafe { &*(inarg.args[0].value as *const fuse_write_in) };
            let data_in = unsafe { &mut *(inarg.args[1].value as *mut MemContainer<raw::c_uchar>) };
            let data = data_in.to_slice();
            let write_out = unsafe { &mut *(outarg.args[0].value as *mut fuse_write_out) };
            let mut reply = ReplyWriteInternal {
                reply: Ok(write_out),
            };
            fs.bento_write(
                &req,
                inarg.h.nodeid,
                write_in.fh,
                write_in.offset as i64,
                data,
                write_in.write_flags,
                &mut reply,
            );
            match reply.reply() {
                Ok(rep) => rep.size as i32,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_FLUSH => {
            if inarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };
            let flush_in = unsafe { &*(inarg.args[0].value as *const fuse_flush_in) };
            let mut reply = ReplyEmptyInternal {
                reply: Err(libc::ENOSYS),
            };
            fs.bento_flush(
                &req,
                inarg.h.nodeid,
                flush_in.fh,
                flush_in.lock_owner,
                &mut reply,
            );
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_RELEASE => {
            if inarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };
            let release_in = unsafe { &*(inarg.args[0].value as *const fuse_release_in) };
            let mut reply = ReplyEmptyInternal {
                reply: Err(libc::ENOSYS),
            };
            fs.bento_release(
                &req,
                inarg.h.nodeid,
                release_in.fh,
                release_in.flags,
                release_in.lock_owner,
                false,
                &mut reply,
            );
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_FSYNC => {
            if inarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };
            let fsync_in = unsafe { &*(inarg.args[0].value as *const fuse_fsync_in) };
            let mut reply = ReplyEmptyInternal {
                reply: Err(libc::ENOSYS),
            };
            let datasync = match fsync_in.fsync_flags {
                1 => true,
                _ => false,
            };
            fs.bento_fsync(&req, inarg.h.nodeid, fsync_in.fh, datasync, &mut reply);
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_OPENDIR => {
            if inarg.numargs != 1 || outarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };

            let open_in = unsafe { &*(inarg.args[0].value as *const fuse_open_in) };
            let open_out = unsafe { &mut *(outarg.args[0].value as *mut fuse_open_out) };
            let mut reply = ReplyOpenInternal {
                reply: Ok(open_out),
            };
            fs.bento_opendir(&req, inarg.h.nodeid, open_in.flags, &mut reply);
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_READDIR => {
            if inarg.numargs != 1 || outarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };
            let read_in = unsafe { &*(inarg.args[0].value as *const fuse_read_in) };
            let data_out =
                unsafe { &mut *(outarg.args[0].value as *mut MemContainer<raw::c_uchar>) };
            let mut reply = ReplyDirectoryInternal {
                reply: Ok(data_out),
                length: 0,
            };
            fs.bento_readdir(
                &req,
                inarg.h.nodeid,
                read_in.fh,
                read_in.offset as i64,
                &mut reply,
            );
            match reply.reply() {
                Ok(buf) => {
                    outarg.args[0].size = buf.len() as u32;
                    0
                }
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_RELEASEDIR => {
            if inarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };
            let release_in = unsafe { &*(inarg.args[0].value as *const fuse_release_in) };
            let mut reply = ReplyEmptyInternal {
                reply: Err(libc::ENOSYS),
            };
            fs.bento_releasedir(
                &req,
                inarg.h.nodeid,
                release_in.fh,
                release_in.flags,
                &mut reply,
            );
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_FSYNCDIR => {
            if inarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };
            let fsync_in = unsafe { &*(inarg.args[0].value as *const fuse_fsync_in) };
            let mut reply = ReplyEmptyInternal {
                reply: Err(libc::ENOSYS),
            };
            let datasync = match fsync_in.fsync_flags {
                1 => true,
                _ => false,
            };
            fs.bento_fsyncdir(&req, inarg.h.nodeid, fsync_in.fh, datasync, &mut reply);
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_STATFS => {
            if inarg.numargs != 0 || outarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };
            let statfs_out = unsafe { &mut *(outarg.args[0].value as *mut fuse_statfs_out) };
            let mut reply = ReplyStatfsInternal {
                reply: Ok(statfs_out),
            };
            fs.bento_statfs(&req, inarg.h.nodeid, &mut reply);
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_SETXATTR => {
            if inarg.numargs != 3 {
                return -1;
            }

            let req = Request { h: &inarg.h };

            let setxattr_in = unsafe { &*(inarg.args[0].value as *const fuse_setxattr_in) };
            let name = unsafe { CStr::from_raw(inarg.args[1].value as *const raw::c_char) };
            let name_str = OsStr::new(str::from_utf8(name.to_bytes_with_nul()).unwrap());
            let value_in =
                unsafe { &mut *(inarg.args[2].value as *mut MemContainer<raw::c_uchar>) };
            let value = value_in.to_slice();
            let mut reply = ReplyEmptyInternal {
                reply: Err(libc::ENOSYS),
            };
            fs.bento_setxattr(
                &req,
                inarg.h.nodeid,
                name_str,
                value,
                setxattr_in.flags,
                setxattr_in.size,
                &mut reply,
            );
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_GETXATTR => {
            if inarg.numargs != 2 || outarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };

            let getxattr_in = unsafe { &*(inarg.args[0].value as *const fuse_getxattr_in) };
            let name = unsafe { CStr::from_raw(inarg.args[1].value as *const raw::c_char) };
            let name_str = OsStr::new(str::from_utf8(name.to_bytes_with_nul()).unwrap());
            if outarg.argvar == 1 {
                let data_out =
                    unsafe { &mut *(outarg.args[0].value as *mut MemContainer<raw::c_uchar>) };
                let mut reply = ReplyXattrInternal {
                    reply_arg: Err(libc::ENOSYS),
                    reply_buf: Ok(data_out),
                };
                fs.bento_getxattr(&req, inarg.h.nodeid, name_str, getxattr_in.size, &mut reply);
                match reply.reply_buf() {
                    Ok(_) => 0,
                    Err(x) => -*x,
                }
            } else {
                let getxattr_out =
                    unsafe { &mut *(outarg.args[0].value as *mut fuse_getxattr_out) };
                let mut reply = ReplyXattrInternal {
                    reply_arg: Ok(getxattr_out),
                    reply_buf: Err(libc::ENOSYS),
                };
                fs.bento_getxattr(&req, inarg.h.nodeid, name_str, getxattr_in.size, &mut reply);
                match reply.reply_arg() {
                    Ok(_) => 0,
                    Err(x) => -*x,
                }
            }
        }
        fuse_opcode_FUSE_LISTXATTR => {
            if inarg.numargs != 1 || outarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };

            let getxattr_in = unsafe { &*(inarg.args[0].value as *const fuse_getxattr_in) };
            if outarg.argvar == 1 {
                let data_out =
                    unsafe { &mut *(outarg.args[0].value as *mut MemContainer<raw::c_uchar>) };
                let mut reply = ReplyXattrInternal {
                    reply_arg: Err(libc::ENOSYS),
                    reply_buf: Ok(data_out),
                };
                fs.bento_listxattr(&req, inarg.h.nodeid, getxattr_in.size, &mut reply);
                match reply.reply_buf() {
                    Ok(_) => 0,
                    Err(x) => -*x,
                }
            } else {
                let getxattr_out =
                    unsafe { &mut *(outarg.args[0].value as *mut fuse_getxattr_out) };
                let mut reply = ReplyXattrInternal {
                    reply_arg: Ok(getxattr_out),
                    reply_buf: Err(libc::ENOSYS),
                };
                fs.bento_listxattr(&req, inarg.h.nodeid, getxattr_in.size, &mut reply);
                match reply.reply_arg() {
                    Ok(_) => 0,
                    Err(x) => -*x,
                }
            }
        }
        fuse_opcode_FUSE_REMOVEXATTR => {
            if inarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };

            let name = unsafe { CStr::from_raw(inarg.args[0].value as *const raw::c_char) };
            let name_str = OsStr::new(str::from_utf8(name.to_bytes_with_nul()).unwrap());
            let mut reply = ReplyEmptyInternal {
                reply: Err(libc::ENOSYS),
            };
            fs.bento_removexattr(&req, inarg.h.nodeid, name_str, &mut reply);
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_ACCESS => {
            if inarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };

            let access_in = unsafe { &*(inarg.args[0].value as *const fuse_access_in) };
            let mut reply = ReplyEmptyInternal {
                reply: Err(libc::ENOSYS),
            };
            fs.bento_access(&req, inarg.h.nodeid, access_in.mask, &mut reply);
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_CREATE => {
            if inarg.numargs != 2 || outarg.numargs != 2 {
                return -1;
            }

            let req = Request { h: &inarg.h };
            let create_in = unsafe { &*(inarg.args[0].value as *const fuse_create_in) };
            let name = unsafe { CStr::from_raw(inarg.args[1].value as *const raw::c_char) };
            let name_str = OsStr::new(str::from_utf8(name.to_bytes_with_nul()).unwrap());
            let entry_out = unsafe { &mut *(outarg.args[0].value as *mut fuse_entry_out) };
            let open_out = unsafe { &mut *(outarg.args[1].value as *mut fuse_open_out) };
            let mut reply = ReplyCreateInternal {
                reply: Ok((entry_out, open_out)),
            };
            fs.bento_create(
                &req,
                inarg.h.nodeid,
                name_str,
                create_in.mode,
                create_in.flags,
                &mut reply,
            );
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_GETLK => {
            if inarg.numargs != 1 || outarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };
            let getlk_in = unsafe { &*(inarg.args[0].value as *const fuse_lk_in) };
            let getlk_out = unsafe { &mut *(outarg.args[0].value as *mut fuse_lk_out) };
            let mut reply = ReplyLockInternal {
                reply: Ok(getlk_out),
            };
            fs.bento_getlk(
                &req,
                inarg.h.nodeid,
                getlk_in.fh,
                getlk_in.owner,
                getlk_in.lk.start,
                getlk_in.lk.end,
                getlk_in.lk.type_,
                getlk_in.lk.pid,
                &mut reply,
            );
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_SETLK => {
            if inarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };
            let setlk_in = unsafe { &*(inarg.args[0].value as *const fuse_lk_in) };
            let mut reply = ReplyEmptyInternal {
                reply: Err(libc::ENOSYS),
            };
            fs.bento_setlk(
                &req,
                inarg.h.nodeid,
                setlk_in.fh,
                setlk_in.owner,
                setlk_in.lk.start,
                setlk_in.lk.end,
                setlk_in.lk.type_,
                setlk_in.lk.pid,
                false,
                &mut reply,
            );
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_SETLKW => {
            if inarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };
            let setlk_in = unsafe { &*(inarg.args[0].value as *const fuse_lk_in) };
            let mut reply = ReplyEmptyInternal {
                reply: Err(libc::ENOSYS),
            };
            fs.bento_setlk(
                &req,
                inarg.h.nodeid,
                setlk_in.fh,
                setlk_in.owner,
                setlk_in.lk.start,
                setlk_in.lk.end,
                setlk_in.lk.type_,
                setlk_in.lk.pid,
                true,
                &mut reply,
            );
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        fuse_opcode_FUSE_BMAP => {
            if inarg.numargs != 1 || outarg.numargs != 1 {
                return -1;
            }

            let req = Request { h: &inarg.h };
            let bmap_in = unsafe { &*(inarg.args[0].value as *const fuse_bmap_in) };
            let bmap_out = unsafe { &mut *(outarg.args[0].value as *mut fuse_bmap_out) };
            let mut reply = ReplyBmapInternal {
                reply: Ok(bmap_out),
            };
            fs.bento_bmap(
                &req,
                inarg.h.nodeid,
                bmap_in.blocksize,
                bmap_in.block,
                &mut reply,
            );
            match reply.reply() {
                Ok(_) => 0,
                Err(x) => -*x,
            }
        }
        BENTO_UPDATE_PREPARE => {
            if outarg.numargs != 1 {
                return -1;
            }

            match fs.bento_update_prepare() {
                Some(x_val) => {
                    // TODO: Serialize the struct once bincode supports no_std
                    outarg.args[0].value = Box::into_raw(Box::new(x_val)) as *const _ as *const raw::c_void;
                    0
                }
                None => 0
            }
        }
        BENTO_UPDATE_TRANSFER => {
            if inarg.numargs != 1 {
                return -1;
            }

            let transfer_ptr = inarg.args[0].value as *mut raw::c_void;
            let transfer_in = if transfer_ptr.is_null() {
                None
            } else {
                // TODO: Deserialize the struct once bincode supports no_std
                unsafe { Some(*Box::from_raw(transfer_ptr as *mut TransferIn)) }
            };
            fs.bento_update_transfer(transfer_in);
            0
        }
        _ => {
            println!("got a different opcode");
            0
        }
    }
}
