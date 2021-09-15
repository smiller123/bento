use core::ops::{Deref, DerefMut};

use bindings as c;
use kernel::ffi;
use libc;

pub struct Socket {
    pub inner: *mut c::sock,
}

impl Socket {
    pub unsafe fn alloc(net: &c::net, family: i32, priority: c::gfp_t,
                        prot: *mut c::proto, kern: i32) -> Result<Self, i32> {
        // Doesn't actually mutate net
        let sk = c::sk_alloc(net as *const c::net as *mut c::net, family, priority, prot, kern);
        if sk.is_null() {
            Err(libc::EIO)
        } else {
            Ok(Self{inner: sk})
        }
    }

    pub fn init_data(&mut self, sock: &c::socket) {
        // Doesn't actually mutate sock
        unsafe {
            c::sock_init_data(sock as *const c::socket as *mut c::socket, self.inner);
        }
    }

    pub fn refcnt_debug_inc(&mut self) {
        unsafe {
            ffi::rs_sk_refcnt_debug_inc(self.inner as *mut c::sock);
        }
    }

    pub fn get_prot(&self) -> Option<&c::proto> {
        unsafe {
            let maybe_prot = (*self.inner).__sk_common.skc_prot;
            if maybe_prot.is_null() {
                None
            } else {
                Some(&*maybe_prot)
            }
        }
    }

    pub unsafe fn from_raw_sock(sock: *mut c::sock) -> Option<Self> {
        if sock.is_null() {
            None
        } else {
            Self { inner: sock }
        }
    }
}

impl Deref for Socket {
    type Target = c::sock;

    fn deref(&self) -> &c::sock {
        unsafe { &*self.inner }
    }
}

impl DerefMut for Socket {
    fn deref_mut(&mut self) -> &mut c::sock {
        unsafe { &mut *self.inner }
    }
}

