use core::ops::{Deref, DerefMut};

use bindings as c;
use kernel::ffi;
use libc;

pub struct Socket {
    pub inner: *mut c::sock,
}

pub struct SocketLockGuard<'a> {
    sock: &'a mut Socket
}

impl Drop for SocketLockGuard<'_> {
    fn drop(&mut self) {
        unsafe {
            c::release_sock(self.sock.inner)
        }
    }
}

impl Deref for SocketLockGuard<'_> {
    type Target = Socket;

    fn deref(&self) -> &Socket {
        unsafe { &*self.sock }
    }
}

impl DerefMut for SocketLockGuard<'_> {
    fn deref_mut(&mut self) -> &mut Socket {
        unsafe { &mut *self.sock }
    }
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
            Some( Self { inner: sock } )
        }
    }

    pub fn lock(&mut self) -> SocketLockGuard {
        unsafe {
            ffi::rs_lock_sock(self.inner);
        }
        SocketLockGuard { sock: self }
    }

    pub fn state(&self) -> u8 {
        unsafe {
            (*self.inner).__sk_common.skc_state
        }
    }

    pub fn host_port(&self) -> u16 {
        unsafe {
            (*self.inner).__sk_common.__bindgen_anon_3.__bindgen_anon_1.skc_num
        }
    }

    pub fn source_addr(&self) -> u32 {
        unsafe {
            (*self.inner).__sk_common.__bindgen_anon_1.__bindgen_anon_1.skc_rcv_saddr
        }
    }

    pub fn set_source_addr(&mut self, saddr: u32) {
        unsafe {
            (*self.inner).__sk_common.__bindgen_anon_1.__bindgen_anon_1.skc_rcv_saddr = saddr;
        }
    }

    pub fn set_dest_addr(&mut self, daddr: u32) {
        unsafe {
            (*self.inner).__sk_common.__bindgen_anon_1.__bindgen_anon_1.skc_daddr = daddr;
        }
    }

    pub fn set_dest_port(&mut self, dport: u16) {
        unsafe {
            (*self.inner).__sk_common.__bindgen_anon_3.__bindgen_anon_1.skc_dport = dport;
        }
    }

    pub fn dst_reset(&mut self) {
        unsafe {
            ffi::rs_sk_dst_reset(self.inner);
        }
    }

    pub fn set_backlog(&mut self, backlog: u32) {
        unsafe {
            core::ptr::write_volatile(&mut (*self.inner).sk_max_ack_backlog as *mut u32, backlog);
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

