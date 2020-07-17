use crate::bindings::*;

use crate::std::sys_common::AsInner;

#[derive(Copy, Clone)]
pub enum IpAddr {
    V4(Ipv4Addr),
    V6(Ipv6Addr),
}

#[derive(Copy, Clone)]
pub struct Ipv4Addr {
    inner: in_addr,
}

#[derive(Copy, Clone)]
pub struct Ipv6Addr {
    inner: in6_addr,
}

impl Ipv4Addr {
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Ipv4Addr {
        Ipv4Addr {
            inner: in_addr {
                s_addr: u32::to_be(
                    ((a as u32) << 24) | ((b as u32) << 16) | ((c as u32) << 8) | (d as u32),
                ),
            },
        }
    }

    pub const LOCALHOST: Self = Ipv4Addr::new(127, 0, 0, 1);

    pub const UNSPECIFIED: Self = Ipv4Addr::new(0, 0, 0, 0);

    pub const BROADCAST: Self = Ipv4Addr::new(255, 255, 255, 255);

    pub fn octets(&self) -> [u8; 4] {
        self.inner.s_addr.to_ne_bytes()
    }
}

impl AsInner<in_addr> for Ipv4Addr {
    fn as_inner(&self) -> &in_addr {
        &self.inner
    }
}

impl From<Ipv4Addr> for IpAddr {
    fn from(ipv4: Ipv4Addr) -> IpAddr {
        IpAddr::V4(ipv4)
    }
}

impl Ipv6Addr {
    pub const fn new(a: u16, b: u16, c: u16, d: u16, e: u16, f: u16, g: u16, h: u16) -> Ipv6Addr {
        Ipv6Addr {
            inner: in6_addr {
                in6_u: in6_addr__bindgen_ty_1 {
                    u6_addr8: [
                        (a >> 8) as u8,
                        a as u8,
                        (b >> 8) as u8,
                        b as u8,
                        (c >> 8) as u8,
                        c as u8,
                        (d >> 8) as u8,
                        d as u8,
                        (e >> 8) as u8,
                        e as u8,
                        (f >> 8) as u8,
                        f as u8,
                        (g >> 8) as u8,
                        g as u8,
                        (h >> 8) as u8,
                        h as u8,
                    ],
                },
            },
        }
    }

    pub const LOCALHOST: Self = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1);

    pub const UNSPECIFIED: Self = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0);

    pub const fn octets(&self) -> [u8; 16] {
        unsafe { self.inner.in6_u.u6_addr8 }
    }

    pub fn segments(&self) -> [u16; 8] {
        unsafe { self.inner.in6_u.u6_addr16 }
    }
}

impl From<Ipv6Addr> for IpAddr {
    fn from(ipv6: Ipv6Addr) -> IpAddr {
        IpAddr::V6(ipv6)
    }
}

impl AsInner<in6_addr> for Ipv6Addr {
    fn as_inner(&self) -> &in6_addr {
        &self.inner
    }
}
