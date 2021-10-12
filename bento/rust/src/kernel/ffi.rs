/*
 * SPDX-License-Identifier: GPL-2.0
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 *
 */

#![macro_use]
use kernel::raw;

use crate::bindings;

pub type Condition = extern "C" fn() -> bool;

/// A macro to create a Rust wrapper around a kernel data type.
///
/// The resulting struct has one field: a pointer to a C data structure.
/// The data layout is therefore identical to a C pointer, so C functions can pass pointers to Rust
/// functions that accept these types.
///
/// Wrapper types cannot be created safely. The `from_raw` method can be used in unsafe Rust to
/// create a wrapper type given a pointer. The `get_raw` method can be used to access the pointer
/// given the wrapper type.
///
/// TODO: Maybe make this a Trait with derive instead of a macro because we can't document the
/// resulting types this way.
///
/// # Examples
///
/// ```
/// // Creates a wrapper-type for a super_block.
/// def_kernel_obj_type!(RsSuperBlock);
/// ```
#[macro_export]
macro_rules! def_kernel_obj_type {
    ($ref_name: ident) => {
        #[derive(Debug, Clone)]
        pub struct $ref_name(*const kernel::raw::c_void);
        impl $ref_name {
            pub unsafe fn from_raw(ptr: *const kernel::raw::c_void) -> $ref_name {
                $ref_name(ptr)
            }

            pub fn get_raw(&self) -> *const kernel::raw::c_void {
                self.0
            }
        }
    };
}

/// A macro for generating a getter function for a non-primitive on a wrapper type.
///
/// This will generate a Rust function that calls a C function. The name of the C function is
/// determined by the types passed into macro. The resulting C function must be defined in
/// helpers.c and exposed in the `extern` block.
///
/// For example, `def_kernel_obj_getter!(RsSuperBlock, s_bdev, super_block, RsBlockDevice);` would
/// generate a function implemented on the `RsSuperBlock` type that returns a `RsBlockDevice`. It
/// would call a C function named `rs_super_block_get_s_bdev`. A user of the `RsSuperBlock` type
/// could then call `s_bdev()` to call the getter function.
#[macro_export]
macro_rules! def_kernel_obj_getter {
    ($t_name: ty, $field_name: ident, $c_type: ident, $field_type: ident) => {
        impl $t_name {
            pub fn $field_name(&self) -> $field_type {
                use kernel::ffi::*;
                let f = concat_idents!(rs_, $c_type, _get_, $field_name);
                unsafe { $field_type(f(self.0)) }
            }
        }
    };
}

/// A macro for generating a getter function for a potentially-NULL non-primitive on a wrapper type.
///
/// This is very much like `def_kernel_obj_getter` except it is safe to use when the resulting C
/// function may return a NULL pointer or an error value cast to a pointer. This function returns
/// an `Option`, returning `None` if the returned pointer is a NULL or an error value.
#[macro_export]
macro_rules! def_kernel_nullable_obj_getter {
    ($t_name: ty, $field_name: ident, $c_type: ident, $field_type: ident) => {
        impl $t_name {
            pub fn $field_name(&self) -> Option<$field_type> {
                use crate::bindings::*;
                use kernel::ffi::*;
                let f = concat_idents!(rs_, $c_type, _get_, $field_name);
                unsafe {
                    let ptr = f(self.0);
                    match ptr.is_null() || IS_ERR(ptr as u64) {
                        true => None,
                        false => Some($field_type(ptr)),
                    }
                }
            }
        }
    };
}

/// A macro for generating a setter function for a non-primitive on a wrapper type.
///
/// This macro is very much like `def_kernel_obj_getter` except that it generates a setter function
/// instead of a getter function. The same of the setter function exposed to Rust is passed in as
/// the second argument of the macro.
#[macro_export]
macro_rules! def_kernel_obj_setter {
    ($t_name: ty, $setter_name: ident, $field_name: ident, $c_type: ident, $field_type: ident) => {
        impl $t_name {
            pub fn $setter_name(&mut self, obj: $field_type) {
                use kernel::ffi::*;
                let f = concat_idents!(rs_, $c_type, _set_, $field_name);
                unsafe {
                    f(self.0, obj.get_raw());
                }
            }
        }
    };
}

/// A macro for generating a getter function for a primitive on a wrapper type.
///
/// This function works much like `def_kernel_obj_getter` except it generates getters for primitive
/// types instead of object types.
#[macro_export]
macro_rules! def_kernel_val_getter {
    ($t_name: ty, $field_name: ident, $c_type: ident, $field_type: ty) => {
        impl $t_name {
            pub fn $field_name(&self) -> $field_type {
                use kernel::ffi::*;
                let f = concat_idents!(rs_, $c_type, _get_, $field_name);
                unsafe { f(self.0) as $field_type }
            }
        }
    };
}

/// A macro for generating a setter function for a primitive on a wrapper type.
///
/// This function works much like `def_kernel_obj_setter` except it generates setters for primitive
/// types instead of object types.
#[macro_export]
macro_rules! def_kernel_val_setter {
    ($t_name: ty, $setter_name: ident, $field_name: ident, $c_type: ident, $field_type: ty) => {
        impl $t_name {
            pub fn $setter_name(&mut self, obj: $field_type) {
                use kernel::ffi::*;
                let f = concat_idents!(rs_, $c_type, _set_, $field_name);
                unsafe {
                    f(self.0, obj);
                }
            }
        }
    };
}

/// A macro for generating both a getter and a setter for a non-primitive on a wrapper type.
///
/// This combines `def_kernel_obj_getter` and `def_kernel_obj_setter` into one macro.
#[macro_export]
macro_rules! def_kernel_obj_accessors {
    ($t_name: ty, $setter_name: ident, $field_name: ident, $c_type: ident, $field_type: ident) => {
        def_kernel_obj_getter!($t_name, $field_name, $c_type, $field_type);
        def_kernel_obj_setter!($t_name, $setter_name, $field_name, $c_type, $field_type);
    };
}

/// A macro for generating both a getter and a setter for a primitive on a wrapper type.
///
/// This combines `def_kernel_val_getter` and `def_kernel_val_setter` into one macro.
#[macro_export]
macro_rules! def_kernel_val_accessors {
    ($t_name: ty, $setter_name: ident, $field_name: ident, $c_type: ident, $field_type: ty) => {
        def_kernel_val_getter!($t_name, $field_name, $c_type, $field_type);
        def_kernel_val_setter!($t_name, $setter_name, $field_name, $c_type, $field_type);
    };
}

/// A macro for generating a mutable operation on a mutable wrapper type.
///
/// This macro will call a C-function that takes one argument (the wrapper type). The C function
/// must be defined in helpers.c and exposed in the `extern` block. This macro will expose a method
/// on the wrapper type that borrows the wrapper mutably, takes no arguments, and calls that C function.
///
/// Examples:
/// ```
/// def_kernel_obj_type!(BufferHead);
/// def_kobj_op!(BufferHead, sync_dirty_buffer, sync_dirty_buffer, i32);
///
/// // bh should be provided by C.
/// fn do_something(bh: BufferHead) {
///     // Calls sync_dirty_buffer(*const buffer_head bh) in the kernel
///     let ret: i32 = bh.sync_dirty_buffer();
///     ...
/// }
#[macro_export]
macro_rules! def_kobj_op {
    // TODO: extend this macro to variadic arguments
    // TODO: extend this macro to have return value
    // TODO: also add a version that does immutable borrow
    ($t_name: ty, $method_name: ident, $c_func_name: ident, $ret_type: ty) => {
        impl $t_name {
            pub fn $method_name(&mut self) -> $ret_type {
                unsafe { $c_func_name(self.get_raw()) }
            }
        }
    };
}

/// A macro for generating an immutable operation on an immutable wrapper type.
///
/// This macro works just like `def_kobj_op` except it generates a method that borrows the wrapper
/// immutably.
#[macro_export]
macro_rules! def_kobj_immut_op {
    // TODO: extend this macro to variadic arguments
    // TODO: extend this macro to have return value
    // TODO: also add a version that does immutable borrow
    ($t_name: ty, $method_name: ident, $c_func_name: ident, $ret_type: ty) => {
        impl $t_name {
            pub fn $method_name(&self) -> $ret_type {
                unsafe { $c_func_name(self.get_raw()) }
            }
        }
    };
}

extern "C" {
    pub fn printk(fmt: *const raw::c_char, ...) -> raw::c_int;

    // kmem
    pub fn __kmalloc(size: usize, flags: u32) -> *mut raw::c_void;
    pub fn kfree(ptr: *const raw::c_void);
    pub fn vmalloc(size: raw::c_size_t) -> *mut raw::c_void;
    pub fn vfree(ptr: *const raw::c_void);

    // mem: TODO: implement these in rust
    pub fn memchr(s: *const raw::c_void, c: i32, n: raw::c_size_t) -> *const raw::c_void;

    // block cache
    pub fn rs_sb_bread(sb: *const raw::c_void, blockno: u64) -> *const raw::c_void;
    pub fn rs_getblk(sb: *const raw::c_void, blockno: u64, size: u32) -> *const raw::c_void;
    pub fn bread_helper(ptr: *const raw::c_void, blockno: u64, size: u32) -> *const raw::c_void;
    pub fn __brelse(buf: *const raw::c_void);
    pub fn blkdev_issue_flush(
        bdev: *const raw::c_void,
        gfp_mask: usize,
        error_sector: *mut u64,
    ) -> isize;
    pub fn rs_super_block_get_s_bdev(sb: *const raw::c_void) -> *const raw::c_void;

    // fs
    pub fn rs_buffer_head_get_b_data(bh: *const raw::c_void) -> *const raw::c_void;
    pub fn rs_buffer_head_get_b_size(bh: *const raw::c_void) -> raw::c_size_t;
    pub fn rs_buffer_head_get_b_blocknr(bh: *const raw::c_void) -> raw::c_size_t;

    pub fn rs_block_device_get_bd_dev(bdev: *const raw::c_void) -> u32;

    pub fn mark_buffer_dirty(bh: *const raw::c_void);
    pub fn sync_dirty_buffer(bh: *const raw::c_void) -> i32;
    pub fn rs_set_buffer_uptodate(bh: *const raw::c_void);

    pub fn rs_get_semaphore() -> *mut raw::c_void;
    pub fn rs_put_semaphore(sem: *const raw::c_void);
    pub fn down_read(sem: *const raw::c_void);
    pub fn up_read(sem: *const raw::c_void);
    pub fn down_write(sem: *const raw::c_void);
    pub fn down_write_trylock(sem: *const raw::c_void) -> i32;
    pub fn down_read_trylock(sem: *const raw::c_void) -> i32;
    pub fn up_write(sem: *const raw::c_void);

    // journal
    pub fn rs_jbd2_journal_init_dev(bdev: *const raw::c_void, fs_dev: *const raw::c_void, 
        start: u64, len: i32, bsize: i32) -> *const raw::c_void;
    pub fn rs_jbd2_journal_load(journal: *const raw::c_void) -> i32;
    pub fn rs_jbd2_journal_destroy(journal: *const raw::c_void) -> i32; 
    pub fn rs_jbd2_journal_start(journal: *const raw::c_void, nblocks: i32) -> *const raw::c_void;
    pub fn rs_jbd2_journal_stop(handle: *const raw::c_void) -> i32;
    pub fn rs_jbd2_journal_get_write_access(handle: *const raw::c_void, bh: *const raw::c_void) -> i32;
    pub fn rs_jbd2_journal_get_create_access(handle: *const raw::c_void, bh: *const raw::c_void) -> i32;
    pub fn rs_jbd2_journal_dirty_metadata(handle: *const raw::c_void, bh: *const raw::c_void) -> i32;
    pub fn rs_jbd2_journal_force_commit(journal: *const raw::c_void) -> i32;
    pub fn rs_jbd2_journal_set_barrier(journal: *const raw::c_void);
    pub fn rs_jbd2_journal_set_async_commit(journal: *const raw::c_void);
    pub fn rs_jbd2_journal_setup(journal: *const raw::c_void);
    pub fn rs_lock_buffer(bh: *const raw::c_void);
    pub fn unlock_buffer(bh: *const raw::c_void);

    // string
    pub fn strnlen(s: *const raw::c_char, max_len: u64) -> u64;
    pub fn strcmp(s1: *const raw::c_char, s2: *const raw::c_char) -> i32;

    // debugging relaed
    pub fn rs_dump_super_block(sb: *const raw::c_void);
    pub fn msleep(msecs: u32);
    pub fn rs_ndelay(usecs: u32);

    pub fn current_kernel_time_rs() -> bindings::timespec64;

    pub fn rs_get_wait_queue_head() -> *mut raw::c_void;
    pub fn rs_put_wait_queue_head(wq_head: *const raw::c_void);
    pub fn rs_wake_up(wq_head: *const raw::c_void);
    pub fn rs_wake_up_all(wq_head: *const raw::c_void);
    pub fn rs_wait_event(wq_head: *const raw::c_void, condition: Condition);
    pub fn register_bento_fs(
        fs: *const raw::c_void,
        fs_name: *const raw::c_void,
        dispatch: *const raw::c_void,
    ) -> i32;
    pub fn reregister_bento_fs(
        fs: *const raw::c_void,
        fs_name: *const raw::c_void,
        dispatch: *const raw::c_void,
    ) -> i32;
    pub fn unregister_bento_fs(fs_name: *const raw::c_void) -> i32;
    pub fn mount() -> i32;
    pub fn lookup_bdev(dev_name: *const raw::c_char, mode: u32) -> *mut raw::c_void;
    pub fn get_bdev_helper(dev_name: *const raw::c_char, mode: u32) -> *mut raw::c_void;
    pub fn blkdev_put(bdev: *const raw::c_void, mode: u32);

    pub fn sock_create_kern(
        net: *const raw::c_void,
        family: i32,
        type_: i32,
        protocol: i32,
        socket: *mut *mut raw::c_void
    ) -> i32;
    pub fn sock_release(socket: *mut raw::c_void);
    pub fn current_net() -> *const raw::c_void;
    pub fn kernel_bind(sock: *mut raw::c_void, addr: *const raw::c_void, addrlen: i32) -> i32;
    pub fn kernel_listen(sock: *mut raw::c_void, backlog: i32) -> i32;
    pub fn kernel_getsockopt(
        sock: *mut raw::c_void,
        level: i32,
        optname: i32,
        optval: *mut raw::c_char,
        optlen: *mut i32
    ) -> i32;
    pub fn kernel_setsockopt(
        sock: *mut raw::c_void,
        level: i32,
        optname: i32,
        optval: *const raw::c_char,
        optlen: u32
    ) -> i32;
    pub fn kernel_accept(sock: *mut raw::c_void, newsock: *mut *mut raw::c_void, flags: i32) -> i32;
    pub fn kernel_connect(sock: *mut raw::c_void, sockaddr: *const raw::c_void, addrlen: i32, flags: i32) -> i32;
    pub fn kernel_getpeername(sock: *mut raw::c_void, sockaddr: *mut raw::c_void, addrlen: *mut i32) -> i32;
    pub fn kernel_getsockname(sock: *mut raw::c_void, sockaddr: *mut raw::c_void, addrlen: *mut i32) -> i32;
    pub fn kernel_recvmsg(
        sock: *mut raw::c_void,
        msg: *const raw::c_void,
        kvec: *mut raw::c_void,
        num: u32,
        len: u32,
        flags: i32,
    ) -> i32;
    pub fn kernel_sendmsg(
        sock: *mut raw::c_void,
        msg: *const raw::c_void,
        kvec: *const raw::c_void,
        num: u32,
        len: u32,
    ) -> i32;
    pub fn kernel_sock_shutdown(socket: *mut raw::c_void, how: bindings::sock_shutdown_cmd) -> i32;
    pub fn kthread_run_helper(
        threadfn: *const raw::c_void,
        data: *mut raw::c_void,
        namefmt: *const raw::c_void
    ) -> *mut raw::c_void;
    pub fn kthread_stop(task_struct: *mut raw::c_void) -> i32;
    pub fn kthread_should_stop() -> bool;
    pub fn wait_a_bit();
    pub fn wait_for_interrupt();

    // TCP Stuff
    pub fn proto_register(prot: *mut bindings::proto, alloc_slab: i32) -> i32;
    pub fn proto_unregister(prot: *mut bindings::proto);
    pub fn sock_register(prot: *const bindings::net_proto_family) -> i32;
    pub fn sock_unregister(family: i32);
    pub fn rs_sk_mem_reclaim(sk: *mut bindings::sock);
    pub fn rs_release_dst_cache(sk: *mut bindings::sock);
    pub fn rs_sk_refcnt_debug_inc(sk: *mut bindings::sock);
    pub fn rs_sk_refcnt_debug_dec(sk: *mut bindings::sock);
    pub fn rs_sk_refcnt_debug_release(sk: *mut bindings::sock);
    pub fn rs_sock_flag(sk: *const bindings::sock, flag: bindings::sock_flags) -> bool;
    pub fn rs_sock_set_flag(sk: *mut bindings::sock, flag: bindings::sock_flags);
    pub fn rs_sock_reset_flag(sk: *mut bindings::sock, flag: bindings::sock_flags);
    pub fn rs_sock_hold(sk: *mut bindings::sock);
    pub fn rs_sock_orphan(sk: *mut bindings::sock);
    pub fn rs_sock_put(sk: *mut bindings::sock);
    pub fn current_flags() -> u32;
    pub fn rs_local_bh_enable();
    pub fn rs_local_bh_disable();
    pub fn rs_bh_lock_sock(sk: *mut bindings::sock);
    pub fn rs_bh_lock_sock_nested(sk: *mut bindings::sock);
    pub fn rs_bh_unlock_sock(sk: *mut bindings::sock);
    pub fn rs_inc_orphan(sk: *mut bindings::sock);
    pub fn rs_dec_orphan(sk: *mut bindings::sock);
    pub fn rs_xfrm_sk_free_policy(sk: *mut bindings::sock);
    pub fn rs_sock_alloc_dec(sk: *mut bindings::sock);
    pub fn rs_ntohs(sin: u16) -> u16;
    pub fn rs_htons(sin: u16) -> u16;
    pub fn rs_htonl(sin: u32) -> u32;
    pub fn rs_ntohl(sin: u32) -> u32;
    pub fn rs_inet_port_requires_bind_service(net: *mut bindings::net, port: u16) -> bool;
    pub fn rs_sock_net(sk: *const bindings::sock) -> *mut bindings::net;
    pub fn rs_lock_sock(sk: *mut bindings::sock);
    pub fn rs_sk_dst_reset(sk: *mut bindings::sock);
    pub fn rs_reqsk_queue_alloc(queue: *mut bindings::request_sock_queue);
    pub fn rs_inet_csk_delack_init(sk: *mut bindings::sock);
    pub fn rs_smp_store_release(p: *mut u8, v: u8);
    pub fn rs_sock_prot_inuse_add(net: *mut bindings::net, prot: *mut bindings::proto, inc: i32);
    pub fn rs_sock_prot_inuse_get(net: *mut bindings::net, prot: *mut bindings::proto) -> i32;
    pub fn rs_sock_graft(sk: *mut bindings::sock, parent: *mut bindings::socket);
    pub fn rs_reqsk_queue_empty(queue: *const bindings::request_sock_queue) -> bool;
    pub fn rs_sock_rcvtimeo(sk: *mut bindings::sock, noblock: bool) -> i64;
    pub fn rs_sock_sndtimeo(sk: *mut bindings::sock, noblock: bool) -> i64;
    pub fn rs_define_wait() -> bindings::wait_queue_entry;
    pub fn rs_define_wait_func(func: bindings::wait_queue_func_t) -> bindings::wait_queue_entry;
    pub fn rs_sk_sleep(sk: *mut bindings::sock) -> *mut bindings::wait_queue_head_t;
    pub fn rs_sched_annotate_sleep();
    pub fn rs_sock_intr_errno(timeo: i64) -> i32;
    pub fn rs_signal_pending() -> i32;
    pub fn rs_sock_error(sk: *mut bindings::sock) -> i32;
    pub fn rs_get_inet_opt(
        inet: *mut bindings::inet_sock,
        sk: *mut bindings::sock
    ) -> *mut bindings::ip_options_rcu;
    pub fn rs_sk_mem_pages(amt: i32) -> i32;
    pub fn rs_sk_memory_allocated_add(sk: *mut bindings::sock, amt: i32) -> i64;
    pub fn rs_sk_wmem_schedule(sk: *mut bindings::sock, size: i32) -> bool;
    pub fn rs_init_list_head(list: *mut bindings::list_head);
    pub fn rs__skb_header_release(skb: *mut bindings::sk_buff);
    pub fn rs_sk_wmem_queued_add(sk: *mut bindings::sock, val: i32);
    pub fn rs_sk_mem_charge(sk: *mut bindings::sock, size: i32);
    pub fn rs_ktime_get_ns() -> u64;
    pub fn rs_skb_cloned(skb: *const bindings::sk_buff) -> i32;
    pub fn rs_pskb_copy(skb: *mut bindings::sk_buff, gfp_mask: bindings::gfp_t) -> *mut bindings::sk_buff;
    pub fn rs_skb_orphan(skb: *mut bindings::sk_buff);
    pub fn rs_refcount_sub_and_test(i: i32, r: *mut bindings::refcount_t) -> bool;
    pub fn rs_refcount_add(i: i32, r: *mut bindings::refcount_t);
    pub fn rs_csum_tcpudp_magic(saddr: u32, daddr: u32, len: u32, proto: u8, sum: u32) -> u16;
    pub fn rs_rcu_read_lock_bh();
    pub fn rs_rcu_read_unlock_bh();
    pub fn rs_cpu_to_be16(i: u16) -> u16;
    pub fn rs_skb_orphan_frags_rx(skb: *mut bindings::sk_buff, gfp_mask: bindings::gfp_t) -> i32;
    pub fn rs_gfp_atomic() -> i32;
    pub fn rs_skb_zcopy(skb: *mut bindings::sk_buff) -> *mut bindings::ubuf_info;
    pub fn rs_check_skb(skb: *mut bindings::sk_buff);
    pub fn rs_dev_hard_header(skb: *mut bindings::sk_buff, dev: *mut bindings::net_device,
                              type_: u16, daddr: *mut raw::c_void, saddr: *mut raw::c_void,
                              len: u32) -> i32;
    pub fn rs_secure_tcp_seq(saddr: u32, daddr: u32, sport: u16, dport: u16) -> u32;
    pub fn rs_skb_share_check(skb: *mut bindings::sk_buff) -> *mut bindings::sk_buff;
    pub fn rs_pskb_may_pull(skb: *mut bindings::sk_buff, len: u32) -> bool;
    pub fn rs_ip_fast_csum(iph: *const raw::c_char, len: u32) -> u16;
    pub fn rs_pskb_trim_rcsum(skb: *mut bindings::sk_buff, len: u32) -> i32;
    pub fn rs_dev_net(dev: *mut bindings::net_device) -> *mut bindings::net;
    pub fn rs___skb_pull(skb: *mut bindings::sk_buff, len: u32) -> *mut raw::c_void;
    pub fn rs_rcu_read_lock();
    pub fn rs_rcu_read_unlock();
    pub fn rs_skb_checksum_init(skb: *mut bindings::sk_buff, proto: i32) -> u16;
    pub fn rs_sk_incoming_cpu_update(sk: *mut bindings::sock);
    pub fn rs_skb_csum_unnecessary(skb: *const bindings::sk_buff) -> i32;
    pub fn rs_reqsk_alloc(
        ops: *const bindings::request_sock_ops,
        sk_listener: *mut bindings::sock,
        attach_listener: bool
    ) -> *mut bindings::request_sock;
    pub fn rs_skb_set_owner_w(skb: *mut bindings::sk_buff, sk: *mut bindings::sock);
    pub fn rs_refcount_set(r: *mut bindings::refcount_t, n: i32);
    pub fn rs_net_xmit_eval(e: i32) -> i32;
    pub fn rs_write_pnet(pnet: *mut bindings::possible_net_t, net: *mut bindings::net);
    pub fn rs_read_pnet(pnet: *mut bindings::possible_net_t) -> *mut bindings::net;
    pub fn rs_reqsk_put(req: *mut bindings::request_sock);
    pub fn rs__skb_queue_tail(list: *mut bindings::sk_buff_head, newsk: *mut bindings::sk_buff);
    pub fn rs_skb_peek(list: *mut bindings::sk_buff_head) -> *mut bindings::sk_buff;
    pub fn rs__skb_unlink(skb: *mut bindings::sk_buff, list: *mut bindings::sk_buff_head);
    pub fn rs__skb_pull(skb: *mut bindings::sk_buff, len: u32) -> *mut raw::c_void;
    pub fn rs_timer_setup(
        timer: *mut bindings::timer_list, 
        func: unsafe extern "C" fn(*mut bindings::timer_list),
        flags: u32
    );
    pub fn rs_get_jiffies_64() -> u64;
    pub fn rs_proto_unregister_mod(prot: *mut bindings::proto);
    pub fn mod_print_stats(module: *mut bindings::module);
}

pub unsafe fn sb_bread(sb: *const raw::c_void, blockno: u64) -> *const raw::c_void {
    rs_sb_bread(sb, blockno)
}
