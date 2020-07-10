use crate::kernel::raw::c_int;

pub type RawFd = c_int;

pub trait AsRawFd {
    fn as_raw_fd(&self) -> RawFd;
}
