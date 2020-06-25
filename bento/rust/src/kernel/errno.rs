use crate::bindings::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    EPERM = -(EPERM as isize),
    ENOENT = -(ENOENT as isize),
    EIO = -(EIO as isize),
    ENOMEM = -(ENOMEM as isize),
    EEXIST = -(EEXIST as isize),
    ENOTDIR = -(ENOTDIR as isize),
    EISDIR = -(EISDIR as isize),
    EINVAL = -(EINVAL as isize),
    ENOSYS = -(ENOSYS as isize),
    ENAMETOOLONG = -(ENAMETOOLONG as isize),
    EOVERFLOW = -(EOVERFLOW as isize),
    ENOTEMPTY = -(ENOTEMPTY as isize),
    ECONNREFUSED = -(ECONNREFUSED as isize),
    ECONNRESET = -(ECONNRESET as isize),
    EPIPE = -(EPIPE as isize),
    ENOTCONN = -(ENOTCONN as isize),
    ECONNABORTED = -(ECONNABORTED as isize),
    EADDRNOTAVAIL = -(EADDRNOTAVAIL as isize),
    EINTR = -(EINTR as isize),
    ETIMEDOUT = -(ETIMEDOUT as isize),
    EAGAIN = -(EAGAIN as isize),
    EADDRINUSE = -(EADDRINUSE as isize),
    EACCES = -(EACCES as isize),
}
