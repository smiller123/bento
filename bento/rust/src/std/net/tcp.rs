use crate::bindings as c;

use crate::kernel::ffi;
use crate::kernel::raw;

use crate::libc;

use crate::std::io;
use crate::std::net::{SocketAddr, SocketAddrV4, SocketAddrV6, Ipv4Addr, Ipv6Addr};

use core::iter::Iterator;
use core::mem;
use core::time::Duration;

#[cfg(feature = "capnproto")]
use alloc::string::ToString;

#[cfg(feature = "capnproto")]
use capnp;

pub struct TcpStream {
    pub inner: *mut c::socket
}

unsafe impl Send for TcpStream {}
unsafe impl Sync for TcpStream {}

pub struct TcpListener {
    pub inner: *mut c::socket
}

unsafe impl Send for TcpListener {}
unsafe impl Sync for TcpListener {}

pub struct Incoming<'a> {
    listener: &'a TcpListener,
}

pub enum Shutdown {
    Read,
    Write,
    Both
}

impl TcpStream {
    pub fn connect(addr: SocketAddr) -> io::Result<TcpStream> {
        unsafe {
            let mut sock = 0 as *mut c::socket;

            let family = match addr {
                SocketAddr::V4(..) => c::AF_INET,
                SocketAddr::V6(..) => c::AF_INET6,
            };
            let ret = ffi::sock_create_kern(
                ffi::current_net(),
                family as i32,
                c::sock_type_SOCK_STREAM as i32,
                c::IPPROTO_TCP as i32,
                &mut sock as *mut *mut c::socket as *mut *mut raw::c_void
            );
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            }

            // Connect to the given address
            match addr {
                SocketAddr::V4(addrv4) => {
                    let sockaddr = addrv4.as_inner() as *const c::sockaddr_in as *const raw::c_void;
                    let sockaddr_size = mem::size_of::<c::sockaddr_in>() as i32;
                    let ret = ffi::kernel_connect(sock as *mut raw::c_void, sockaddr, sockaddr_size, 0);
                    if ret != 0 {
                        return Err(io::Error::from_raw_os_error(ret));
                    }
                },
                SocketAddr::V6(addrv6) => {
                    let sockaddr = addrv6.as_inner() as *const c::sockaddr_in6 as *const raw::c_void;
                    let sockaddr_size = mem::size_of::<c::sockaddr_in6>() as i32;
                    let ret = ffi::kernel_connect(sock as *mut raw::c_void, sockaddr, sockaddr_size, 0);
                    if ret != 0 {
                        return Err(io::Error::from_raw_os_error(ret));
                    }
                },
            };
            return Ok( TcpStream { inner: sock } );
        }
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        unsafe {
            let mut sockaddr: c::sockaddr = c::sockaddr {
                sa_family: 0,
                sa_data: [0; 14],
            };
            let mut addrlen = 0;
            let ret = ffi::kernel_getsockname(
                self.inner as *mut raw::c_void,
                &mut sockaddr as *mut c::sockaddr as *mut raw::c_void,
                &mut addrlen as *mut i32
            );
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            }
            match sockaddr.sa_family as u32 {
                c::AF_INET => {
                    let sin = *(&sockaddr as *const c::sockaddr as *const c::sockaddr_in);
                    let sin_addr = sin.sin_addr.s_addr.to_be_bytes();
                    let ipaddr = Ipv4Addr::new(sin_addr[0], sin_addr[1], sin_addr[2], sin_addr[3]);
                    return Ok(
                         SocketAddr::V4(SocketAddrV4::new(ipaddr, sin.sin_port))
                    );
                },
                c::AF_INET6 => {
                    let sin6 = *(&sockaddr as *const c::sockaddr as *const c::sockaddr_in6);
                    let sin_addr = sin6.sin6_addr.in6_u.u6_addr16;
                    let ipaddr = Ipv6Addr::new(
                        sin_addr[0],
                        sin_addr[1],
                        sin_addr[2],
                        sin_addr[3],
                        sin_addr[4],
                        sin_addr[5],
                        sin_addr[6],
                        sin_addr[7]
                    );
                    return Ok(
                        SocketAddr::V6(
                            SocketAddrV6::new(
                                ipaddr,
                                sin6.sin6_port,
                                sin6.sin6_flowinfo,
                                sin6.sin6_scope_id
                            )
                        )
                    );
                },
                _ => {
                    return Err(io::Error::from_raw_os_error(-libc::EIO));
                }
            }
        }
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        unsafe {
            let mut sockaddr: c::sockaddr = c::sockaddr {
                sa_family: 0,
                sa_data: [0; 14],
            };
            let mut addrlen = 0;
            let ret = ffi::kernel_getpeername(
                self.inner as *mut raw::c_void,
                &mut sockaddr as *mut c::sockaddr as *mut raw::c_void,
                &mut addrlen as *mut i32
            );
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            }
            match sockaddr.sa_family as u32 {
                c::AF_INET => {
                    let sin = *(&sockaddr as *const c::sockaddr as *const c::sockaddr_in);
                    let sin_addr = sin.sin_addr.s_addr.to_be_bytes();
                    let ipaddr = Ipv4Addr::new(sin_addr[0], sin_addr[1], sin_addr[2], sin_addr[3]);
                    return Ok(
                         SocketAddr::V4(SocketAddrV4::new(ipaddr, sin.sin_port))
                    );
                },
                c::AF_INET6 => {
                    let sin6 = *(&sockaddr as *const c::sockaddr as *const c::sockaddr_in6);
                    let sin_addr = sin6.sin6_addr.in6_u.u6_addr16;
                    let ipaddr = Ipv6Addr::new(
                        sin_addr[0],
                        sin_addr[1],
                        sin_addr[2],
                        sin_addr[3],
                        sin_addr[4],
                        sin_addr[5],
                        sin_addr[6],
                        sin_addr[7]
                    );
                    return Ok(
                        SocketAddr::V6(
                            SocketAddrV6::new(
                                ipaddr,
                                sin6.sin6_port,
                                sin6.sin6_flowinfo,
                                sin6.sin6_scope_id
                            )
                        )
                    );
                },
                _ => {
                    return Err(io::Error::from_raw_os_error(-libc::EIO));
                }
            }
        }
    }

    pub fn nodelay(&self) -> io::Result<bool> {
        let mut nodelay = false;
        let mut optlen = 0;
        unsafe {
            let ret = ffi::kernel_getsockopt(
                self.inner as *mut raw::c_void,
                c::IPPROTO_TCP as i32,
                c::TCP_NODELAY as i32,
                &mut nodelay as *mut bool as *mut raw::c_char,
                &mut optlen,
            );
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            }
            return Ok(nodelay);
        }
    }

    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        unsafe {
            let ret = ffi::kernel_setsockopt(
                self.inner as *mut raw::c_void,
                c::IPPROTO_TCP as i32,
                c::TCP_NODELAY as i32,
                &nodelay as *const bool as *const raw::c_char,
                mem::size_of::<bool>() as u32,
            );
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            }
            return Ok(());
        }
    }

    pub fn ttl(&self) -> io::Result<u32> {
        let mut ttl = 0;
        let mut optlen = 0;
        unsafe {
            let ret = ffi::kernel_getsockopt(
                self.inner as *mut raw::c_void,
                c::IPPROTO_IP as i32,
                c::IP_TTL as i32,
                &mut ttl as *mut u32 as *mut raw::c_char,
                &mut optlen,
            );
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            }
            return Ok(ttl);
        }
    }

    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        unsafe {
            let ret = ffi::kernel_setsockopt(
                self.inner as *mut raw::c_void,
                c::IPPROTO_IP as i32,
                c::IP_TTL as i32,
                &ttl as *const u32 as *const raw::c_char,
                mem::size_of::<u32>() as u32,
            );
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            }
            return Ok(());
        }
    }

    pub fn read_timeout(&self) -> io::Result<Option<Duration>> {
        let mut read_timeout = c::timeval {
            tv_sec: 0,
            tv_usec: 0,
        };
        let mut optlen = 0;
        unsafe {
            let ret = ffi::kernel_getsockopt(
                self.inner as *mut raw::c_void,
                c::SOL_SOCKET as i32,
                c::SO_RCVTIMEO as i32,
                &mut read_timeout as *mut c::timeval as *mut raw::c_char,
                &mut optlen,
            );
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            }
            if read_timeout.tv_sec == 0 && read_timeout.tv_usec == 0 {
                return Ok(None);
            } else {
                let sec = read_timeout.tv_sec as u64;
                let nsec = (read_timeout.tv_usec as u32) * 1000;
                return Ok(Some(Duration::new(sec, nsec)));
            }
        }
    }

    pub fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        unsafe {
            let timeout = match dur {
                Some(dur) => {
                    if dur.as_secs() == 0 && dur.subsec_nanos() == 0 {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "cannot set a 0 duration timeout",
                        ));
                    }
                    let secs = dur.as_secs() as c::__kernel_time_t;
                    let mut timeout = c::timeval {
                        tv_sec: secs,
                        tv_usec: dur.subsec_micros() as c::__kernel_suseconds_t,
                    };
                    if timeout.tv_sec == 0 && timeout.tv_usec == 0 {
                        timeout.tv_usec = 1;
                    }
                    timeout
                },
                None => c::timeval { tv_sec: 0, tv_usec: 0 },
            };

            let ret = ffi::kernel_setsockopt(
                self.inner as *mut raw::c_void,
                c::SOL_SOCKET as i32,
                c::SO_RCVTIMEO as i32,
                &timeout as *const c::timeval as *const raw::c_char,
                mem::size_of::<c::timeval>() as u32,
            );
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            }
            return Ok(());
        }
    }

    pub fn write_timeout(&self) -> io::Result<Option<Duration>> {
        let mut write_timeout = c::timeval {
            tv_sec: 0,
            tv_usec: 0,
        };
        let mut optlen = 0;
        unsafe {
            let ret = ffi::kernel_getsockopt(
                self.inner as *mut raw::c_void,
                c::SOL_SOCKET as i32,
                c::SO_SNDTIMEO as i32,
                &mut write_timeout as *mut c::timeval as *mut raw::c_char,
                &mut optlen,
            );
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            }
            if write_timeout.tv_sec == 0 && write_timeout.tv_usec == 0 {
                return Ok(None);
            } else {
                let sec = write_timeout.tv_sec as u64;
                let nsec = (write_timeout.tv_usec as u32) * 1000;
                return Ok(Some(Duration::new(sec, nsec)));
            }
        }
    }

    pub fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        unsafe {
            let timeout = match dur {
                Some(dur) => {
                    if dur.as_secs() == 0 && dur.subsec_nanos() == 0 {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "cannot set a 0 duration timeout",
                        ));
                    }
                    let secs = dur.as_secs() as c::__kernel_time_t;
                    let mut timeout = c::timeval {
                        tv_sec: secs,
                        tv_usec: dur.subsec_micros() as c::__kernel_suseconds_t,
                    };
                    if timeout.tv_sec == 0 && timeout.tv_usec == 0 {
                        timeout.tv_usec = 1;
                    }
                    timeout
                },
                None => c::timeval { tv_sec: 0, tv_usec: 0 },
            };

            let ret = ffi::kernel_setsockopt(
                self.inner as *mut raw::c_void,
                c::SOL_SOCKET as i32,
                c::SO_SNDTIMEO as i32,
                &timeout as *const c::timeval as *const raw::c_char,
                mem::size_of::<c::timeval>() as u32,
            );
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            }
            return Ok(());
        }
    }

    fn recv_with_flags(&self, buf: &mut [u8], flags: raw::c_int) -> io::Result<usize> {
        unsafe {
            let mut msg = c::msghdr::default();
            msg.msg_flags = flags as u32;
            let mut iov = c::kvec {
                iov_base: buf.as_mut_ptr() as *mut u8 as *mut raw::c_void,
                iov_len: buf.len() as u64,
            };

            let len = ffi::kernel_recvmsg(
                self.inner as *mut raw::c_void,
                &msg as *const c::msghdr as *const raw::c_void,
                &mut iov as *mut c::kvec as *mut raw::c_void,
                1,
                buf.len() as u32,
                flags
            );
            if len < 0 {
                return Err(io::Error::from_raw_os_error(len));
            }
            return Ok(len as usize);
        }
    }

    fn send_with_flags(&self, buf: &[u8], flags: raw::c_int) -> io::Result<usize> {
        unsafe {
            let mut msg = c::msghdr::default();
            msg.msg_flags = flags as u32;
            let iov = c::kvec {
                iov_base: buf.as_ptr() as *const u8 as *const raw::c_void as *mut raw::c_void,
                iov_len: buf.len() as u64,
            };

            let len = ffi::kernel_sendmsg(
                self.inner as *mut raw::c_void,
                &msg as *const c::msghdr as *const raw::c_void,
                &iov as *const c::kvec as *const raw::c_void,
                1,
                buf.len() as u32
            );
            if len < 0 {
                return Err(io::Error::from_raw_os_error(len));
            }
            return Ok(len as usize);
        }
    }

    pub fn peek(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.recv_with_flags(buf, c::MSG_PEEK as i32)
    }

    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        let mut sk_err: isize = 0;
        let mut optlen = 0;
        unsafe {
            let ret = ffi::kernel_getsockopt(
                self.inner as *mut raw::c_void,
                c::SOL_SOCKET as i32,
                c::SO_ERROR as i32,
                &mut sk_err as *mut isize as *mut raw::c_char,
                &mut optlen,
            );
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            }
            if sk_err == 0 {
                return Ok(None);
            } else {
                return Ok(Some(io::Error::from_raw_os_error(sk_err as i32)));
            }
        }
    }

    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        unsafe {
            let enum_how = match how {
                Shutdown::Read => { c::sock_shutdown_cmd_SHUT_RD }
                Shutdown::Write => { c::sock_shutdown_cmd_SHUT_WR }
                Shutdown::Both => { c::sock_shutdown_cmd_SHUT_RDWR }
            };
            let ret = ffi::kernel_sock_shutdown(self.inner as *mut raw::c_void, enum_how);
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            } else {
                Ok(())
            }
        }
    }
}

impl io::Read for TcpStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.recv_with_flags(buf, 0)
    }
}

impl io::Write for TcpStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.send_with_flags(buf, 0)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "capnproto")]
impl capnp::io::Read for TcpStream {
    fn read(&mut self, buf: &mut [u8]) -> capnp::Result<usize> {
        self.recv_with_flags(buf, 0).map_err(|_| capnp::Error::failed("recv failed".to_string()))
    }
}

#[cfg(feature = "capnproto")]
impl capnp::io::Write for TcpStream {
    fn write_all(&mut self, buf: &[u8]) -> capnp::Result<()> {
        match self.send_with_flags(buf, 0) {
            Err(_) => Err(capnp::Error::failed("send failed".to_string())),
            Ok(_) => Ok(()),
        }
    }
}

#[cfg(feature = "capnproto")]
impl capnp::io::Read for &TcpStream {
    fn read(&mut self, buf: &mut [u8]) -> capnp::Result<usize> {
        self.recv_with_flags(buf, 0).map_err(|_| capnp::Error::failed("recv failed".to_string()))
    }
}

#[cfg(feature = "capnproto")]
impl capnp::io::Write for &TcpStream {
    fn write_all(&mut self, buf: &[u8]) -> capnp::Result<()> {
        match self.send_with_flags(buf, 0) {
            Err(_) => Err(capnp::Error::failed("send failed".to_string())),
            Ok(_) => Ok(()),
        }
    }
}

impl io::Read for &TcpStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.recv_with_flags(buf, 0)
    }
}

impl io::Write for &TcpStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.send_with_flags(buf, 0)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        unsafe {
            ffi::sock_release(self.inner as *mut raw::c_void);
        }
    }
}

impl TcpListener {
    pub fn bind(addr: SocketAddr) -> io::Result<TcpListener> {
        unsafe {
            // Create the socket
            let mut sock = 0 as *mut c::socket;
            let family = match addr {
                SocketAddr::V4(..) => c::AF_INET,
                SocketAddr::V6(..) => c::AF_INET6,
            };
            let ret = ffi::sock_create_kern(
                ffi::current_net(),
                family as i32,
                c::sock_type_SOCK_STREAM as i32,
                c::IPPROTO_TCP as i32,
                &mut sock as *mut *mut c::socket as *mut *mut raw::c_void
            );
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            }

            // Set SO_REUSEADDR
            let one: i32 = 1;
            let ret = ffi::kernel_setsockopt(
                sock as *mut raw::c_void,
                c::SOL_SOCKET as i32,
                c::SO_REUSEADDR as i32,
                &one as *const i32 as *const raw::c_char,
                mem::size_of::<i32>() as u32,
            );
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            }

            // Bind to the given address
            match addr {
                SocketAddr::V4(addrv4) => {
                    let sockaddr = addrv4.as_inner() as *const c::sockaddr_in as *const raw::c_void;
                    let sockaddr_size = mem::size_of::<c::sockaddr_in>() as i32;
                    let ret = ffi::kernel_bind(sock as *mut raw::c_void, sockaddr, sockaddr_size);
                    if ret != 0 {
                        return Err(io::Error::from_raw_os_error(ret));
                    }
                },
                SocketAddr::V6(addrv6) => {
                    let sockaddr = addrv6.as_inner() as *const c::sockaddr_in6 as *const raw::c_void;
                    let sockaddr_size = mem::size_of::<c::sockaddr_in6>() as i32;
                    let ret = ffi::kernel_bind(sock as *mut raw::c_void, sockaddr, sockaddr_size);
                    if ret != 0 {
                        return Err(io::Error::from_raw_os_error(ret));
                    }
                },
            };

            // Listen
            let ret = ffi::kernel_listen(sock as *mut raw::c_void, 128);
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            }
            Ok(Self {
                inner: sock
            })
        }
    }

    pub fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        unsafe {
            let mut sock = 0 as *mut c::socket;
            let ret = ffi::kernel_accept(
                self.inner as *mut raw::c_void,
                &mut sock as *mut *mut c::socket as *mut *mut raw::c_void,
                libc::SOCK_CLOEXEC
            );
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            }
            let mut sockaddr: c::sockaddr = c::sockaddr {
                sa_family: 0,
                sa_data: [0; 14],
            };
            let mut addrlen = 0;
            let ret = ffi::kernel_getpeername(
                sock as *mut raw::c_void,
                &mut sockaddr as *mut c::sockaddr as *mut raw::c_void,
                &mut addrlen as *mut i32
            );
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            }
            match sockaddr.sa_family as u32 {
                c::AF_INET => {
                    let sin = *(&sockaddr as *const c::sockaddr as *const c::sockaddr_in);
                    let sin_addr = sin.sin_addr.s_addr.to_be_bytes();
                    let ipaddr = Ipv4Addr::new(sin_addr[0], sin_addr[1], sin_addr[2], sin_addr[3]);
                    return Ok(
                        (TcpStream { inner: sock },
                         SocketAddr::V4(SocketAddrV4::new(ipaddr, sin.sin_port))
                         )
                    );
                },
                c::AF_INET6 => {
                    let sin6 = *(&sockaddr as *const c::sockaddr as *const c::sockaddr_in6);
                    let sin_addr = sin6.sin6_addr.in6_u.u6_addr16;
                    let ipaddr = Ipv6Addr::new(
                        sin_addr[0],
                        sin_addr[1],
                        sin_addr[2],
                        sin_addr[3],
                        sin_addr[4],
                        sin_addr[5],
                        sin_addr[6],
                        sin_addr[7]
                    );
                    return Ok(
                        (TcpStream { inner: sock },
                         SocketAddr::V6(
                            SocketAddrV6::new(
                                ipaddr,
                                sin6.sin6_port,
                                sin6.sin6_flowinfo,
                                sin6.sin6_scope_id
                                )
                             )
                         )
                    );
                },
                _ => {
                    return Err(io::Error::from_raw_os_error(-libc::EIO));
                }
            }
        }
    }

    pub fn incoming(&self) -> Incoming<'_> {
        Incoming { listener: self }
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        unsafe {
            let mut sockaddr: c::sockaddr = c::sockaddr {
                sa_family: 0,
                sa_data: [0; 14],
            };
            let mut addrlen = 0;
            let ret = ffi::kernel_getsockname(
                self.inner as *mut raw::c_void,
                &mut sockaddr as *mut c::sockaddr as *mut raw::c_void,
                &mut addrlen as *mut i32
            );
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            }
            match sockaddr.sa_family as u32 {
                c::AF_INET => {
                    let sin = *(&sockaddr as *const c::sockaddr as *const c::sockaddr_in);
                    let sin_addr = sin.sin_addr.s_addr.to_be_bytes();
                    let ipaddr = Ipv4Addr::new(sin_addr[0], sin_addr[1], sin_addr[2], sin_addr[3]);
                    return Ok(
                         SocketAddr::V4(SocketAddrV4::new(ipaddr, sin.sin_port))
                    );
                },
                c::AF_INET6 => {
                    let sin6 = *(&sockaddr as *const c::sockaddr as *const c::sockaddr_in6);
                    let sin_addr = sin6.sin6_addr.in6_u.u6_addr16;
                    let ipaddr = Ipv6Addr::new(
                        sin_addr[0],
                        sin_addr[1],
                        sin_addr[2],
                        sin_addr[3],
                        sin_addr[4],
                        sin_addr[5],
                        sin_addr[6],
                        sin_addr[7]
                    );
                    return Ok(
                        SocketAddr::V6(
                            SocketAddrV6::new(
                                ipaddr,
                                sin6.sin6_port,
                                sin6.sin6_flowinfo,
                                sin6.sin6_scope_id
                            )
                        )
                    );
                },
                _ => {
                    return Err(io::Error::from_raw_os_error(-libc::EIO));
                }
            }
        }
    }

    pub fn ttl(&self) -> io::Result<u32> {
        let mut ttl = 0;
        let mut optlen = 0;
        unsafe {
            let ret = ffi::kernel_getsockopt(
                self.inner as *mut raw::c_void,
                c::IPPROTO_IP as i32,
                c::IP_TTL as i32,
                &mut ttl as *mut u32 as *mut raw::c_char,
                &mut optlen,
            );
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            }
            return Ok(ttl);
        }
    }

    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        unsafe {
            let ret = ffi::kernel_setsockopt(
                self.inner as *mut raw::c_void,
                c::IPPROTO_IP as i32,
                c::IP_TTL as i32,
                &ttl as *const u32 as *const raw::c_char,
                mem::size_of::<u32>() as u32,
            );
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            }
            return Ok(());
        }
    }

    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        let mut sk_err: isize = 0;
        let mut optlen = 0;
        unsafe {
            let ret = ffi::kernel_getsockopt(
                self.inner as *mut raw::c_void,
                c::SOL_SOCKET as i32,
                c::SO_ERROR as i32,
                &mut sk_err as *mut isize as *mut raw::c_char,
                &mut optlen,
            );
            if ret != 0 {
                return Err(io::Error::from_raw_os_error(ret));
            }
            if sk_err == 0 {
                return Ok(None);
            } else {
                return Ok(Some(io::Error::from_raw_os_error(sk_err as i32)));
            }
        }
    }
}

impl Drop for TcpListener {
    fn drop(&mut self) {
        unsafe {
            ffi::sock_release(self.inner as *mut raw::c_void);
        }
    }
}

impl<'a> Iterator for Incoming<'a> {
    type Item = io::Result<TcpStream>;
    fn next(&mut self) -> Option<io::Result<TcpStream>> {
        Some(self.listener.accept().map(|p| p.0))
    }
}
