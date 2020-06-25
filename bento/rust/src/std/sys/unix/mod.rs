use crate::kernel::errno::Error;
use crate::std::io::ErrorKind;

pub fn decode_error_kind(errno: Error) -> ErrorKind {
    match errno {
        Error::ECONNREFUSED => ErrorKind::ConnectionRefused,
        Error::ECONNRESET => ErrorKind::ConnectionReset,
        Error::EPERM | Error::EACCES => ErrorKind::PermissionDenied,
        Error::EPIPE => ErrorKind::BrokenPipe,
        Error::ENOTCONN => ErrorKind::NotConnected,
        Error::ECONNABORTED => ErrorKind::ConnectionAborted,
        Error::EADDRNOTAVAIL => ErrorKind::AddrNotAvailable,
        Error::EADDRINUSE => ErrorKind::AddrInUse,
        Error::ENOENT => ErrorKind::NotFound,
        Error::EINTR => ErrorKind::Interrupted,
        Error::EINVAL => ErrorKind::InvalidInput,
        Error::ETIMEDOUT => ErrorKind::TimedOut,
        Error::EEXIST => ErrorKind::AlreadyExists,

        // These two constants can have the same value on some systems,
        // but different values on others, so we can't use a match
        // clause
        x if x == Error::EAGAIN  => ErrorKind::WouldBlock,

        _ => ErrorKind::Other,
    }
}
