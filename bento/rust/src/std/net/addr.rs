use crate::bindings::*;

use core::mem;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use std::sys_common::AsInner;

#[derive(Copy, Clone)]
pub enum SocketAddr {
    V4(SocketAddrV4),
    V6(SocketAddrV6),
}

#[derive(Copy, Clone)]
pub struct SocketAddrV4 {
    inner: sockaddr_in,
}

#[derive(Copy, Clone)]
pub struct SocketAddrV6 {
    inner: sockaddr_in6,
}

impl SocketAddr {
    pub fn new(ip: IpAddr, port: u16) -> SocketAddr {
        match ip {
            IpAddr::V4(a) => SocketAddr::V4(SocketAddrV4::new(a, port)),
            IpAddr::V6(a) => SocketAddr::V6(SocketAddrV6::new(a, port, 0, 0)),
        }
    }

    pub fn ip(&self) -> IpAddr {
        match *self {
            SocketAddr::V4(ref a) => IpAddr::V4(*a.ip()),
            SocketAddr::V6(ref a) => IpAddr::V6(*a.ip()),
        }
    }

    pub fn port(&self) -> u16 {
        match *self {
            SocketAddr::V4(ref a) => a.port(),
            SocketAddr::V6(ref a) => a.port(),
        }
    }

    pub fn set_ip(&mut self, new_ip: IpAddr) {
        match (self, new_ip) {
            (&mut SocketAddr::V4(ref mut a), IpAddr::V4(new_ip)) => a.set_ip(new_ip),
            (&mut SocketAddr::V6(ref mut a), IpAddr::V6(new_ip)) => a.set_ip(new_ip),
            (self_, new_ip) => *self_ = Self::new(new_ip, self_.port()),
        }
    }

    pub fn set_port(&mut self, new_port: u16) {
        match *self {
            SocketAddr::V4(ref mut a) => a.set_port(new_port),
            SocketAddr::V6(ref mut a) => a.set_port(new_port),
        }
    }
}

impl SocketAddrV4 {
    pub fn new(ip: Ipv4Addr, port: u16) -> SocketAddrV4 {
        SocketAddrV4 {
            inner: sockaddr_in {
                sin_family: AF_INET as sa_family_t,
                sin_port: port.to_be(),
                sin_addr: *ip.as_inner(),
                ..unsafe { mem::zeroed() }
            },
        }
    }

    pub fn as_inner(&self) -> &sockaddr_in {
        &self.inner
    }

    pub fn ip(&self) -> &Ipv4Addr {
        unsafe { &*(&self.inner.sin_addr as *const in_addr as *const Ipv4Addr) }    
    }

    pub fn port(&self) -> u16 {
        u16::from_be(self.inner.sin_port)
    }

    pub fn set_ip(&mut self, new_ip: Ipv4Addr) {
        self.inner.sin_addr = *new_ip.as_inner()
    }

    pub fn set_port(&mut self, new_port: u16) {
        self.inner.sin_port = new_port.to_be()
    }
}

impl SocketAddrV6 {
    pub fn new(ip: Ipv6Addr, port: u16, flowinfo: u32, scope_id: u32) -> SocketAddrV6 {
        SocketAddrV6 {
            inner: sockaddr_in6 {
                sin6_family: AF_INET6 as sa_family_t,
                sin6_port: port.to_be(),
                sin6_addr: *ip.as_inner(),
                sin6_flowinfo: flowinfo,
                sin6_scope_id: scope_id,
                ..unsafe { mem::zeroed() }
            },
        }
    }

    pub fn as_inner(&self) -> &sockaddr_in6 {
        &self.inner
    }

    pub fn ip(&self) -> &Ipv6Addr {
        unsafe { &*(&self.inner.sin6_addr as *const in6_addr as *const Ipv6Addr) }    
    }

    pub fn port(&self) -> u16 {
        u16::from_be(self.inner.sin6_port)
    }

    pub fn flowinfo(&self) -> u32 {
        self.inner.sin6_flowinfo
    }

    pub fn scope_id(&self) -> u32 {
        self.inner.sin6_scope_id
    }

    pub fn set_ip(&mut self, new_ip: Ipv6Addr) {
        self.inner.sin6_addr = *new_ip.as_inner()
    }

    pub fn set_port(&mut self, new_port: u16) {
        self.inner.sin6_port = new_port.to_be()
    }

    pub fn set_flowinfo(&mut self, new_flowinfo: u32) {
        self.inner.sin6_flowinfo = new_flowinfo
    }

    pub fn set_scope_id(&mut self, new_scope_id: u32) {
        self.inner.sin6_scope_id = new_scope_id
    }
}
