pub mod ringbuffer;
pub mod hrtick;
pub mod sched_core;
pub mod rbtree;

use alloc::boxed::Box;
use libc::ENOSYS;
use libc;
use kernel::ffi;
use core::marker::PhantomData;

use time::Timespec;

use bindings as c;
use bindings::{register_enoki_sched,unregister_enoki_sched,reregister_enoki_sched};
use kernel::raw;

use self::ringbuffer::RingBuffer;

use core::fmt::Debug;
use core::convert::TryInto;

use serde::{Serialize, Deserialize};

use kernel::time::Timespec64;
use kernel::time::getnstimeofday64_rs;
use kernel::time::diff_ns;

pub const BENTO_KERNEL_VERSION: u32 = 1;
pub const BENTO_KERNEL_MINOR_VERSION: u32 = 0;

pub struct RQLockGuard {
    random_data: PhantomData<i32>,
}

#[derive(Default, Debug)]
pub struct Schedulable {
    pid: u64,
    cpu: u32
}

impl Schedulable {
    pub fn get_cpu(&self) -> u32 {
        self.cpu
    }

    pub fn get_pid(&self) -> u64 {
        self.pid
    }
}

pub fn parse_message<'a, 'b, TransferIn: Send, TransferOut: Send,
    UserMessage: Send+ Copy + Serialize + Deserialize<'a>,
    RevMessage: Send + Copy + Serialize + Deserialize<'b>,
    T: BentoScheduler<'a, 'b, TransferIn, TransferOut, UserMessage, RevMessage>>(
    sched: &mut T,
    type_: i32,
    msglen: i32,
    barrier: u32,
    payload: *mut raw::c_void,
    payload_size: i32,
    retval: *mut i32)
{
    unsafe {
        match type_ as u32 {
            c::MSG_PNT => {
                let payload_data = payload as *mut c::enoki_msg_payload_pnt;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("pnt: {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                let curr_sched = if (*payload_data).is_curr {
                    let sched = Schedulable {
                        cpu: (*payload_data).cpu as u32,
                        pid: (*payload_data).curr_pid,
                    };
                    Some(sched)
                } else {
                    None
                };
                let curr_runtime = if (*payload_data).is_curr {
                    Some((*payload_data).curr_runtime)
                } else {
                    None
                };
                let guard = RQLockGuard{random_data: PhantomData};
                let next_task = sched.pick_next_task((*payload_data).cpu, curr_sched, curr_runtime, guard);
                if next_task.is_none() {
                    (*payload_data).pick_task = false;
                    (*payload_data).ret_pid = 0;
                    #[cfg(feature = "record")]
                    {
                        let curr = unsafe {
                            ffi::rs_current()
                        };
                        let mut write_str = alloc::format!("pnt_ret: {:?} {} {:?}\n\0",
                                                           curr, (*payload_data).cpu, next_task);
                        c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                    }
                } else if let Some(ref ret_sched) = next_task &&
                    (ret_sched.cpu == (*payload_data).cpu as u32 || ret_sched.cpu == u32::MAX) {
                    let ret_cpu = ret_sched.get_cpu();
                    let ret_pid = ret_sched.get_pid();
                    (*payload_data).pick_task = next_task.is_some();
                    (*payload_data).ret_pid = next_task.unwrap_or_default().get_pid();
                    #[cfg(feature = "record")]
                    {
                        let curr = unsafe {
                            ffi::rs_current()
                        };
                        let rec_sched = Some(Schedulable{cpu: ret_cpu, pid: ret_pid});
                        let mut write_str = alloc::format!("pnt_ret: {:?} {} {:?}\n\0",
                                                           curr, (*payload_data).cpu, rec_sched);
                        c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                    }
                } else {
                    // The process can't be scheduled on this cpu

                    let sched = next_task.unwrap();
                    (*payload_data).pick_task = false;
                    let guard = RQLockGuard{random_data: PhantomData};
                    let ret_cpu = sched.get_cpu();
                    let ret_pid = sched.get_pid();
                    sched.pnt_err(sched.get_cpu() as i32, sched.get_pid(), 2, Some(sched), guard);
                    #[cfg(feature = "record")]
                    {
                        let curr = unsafe {
                            ffi::rs_current()
                        };
                        let ret_sched = Some(Schedulable{cpu: ret_cpu, pid: ret_pid});
                        let mut write_str = alloc::format!("pnt_ret: {:?} {} {:?} Error\n\0",
                                                           curr, (*payload_data).cpu, ret_sched);
                        c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                    }
                }
            }
            c::MSG_PNT_ERR => {
                let payload_data = payload as *mut c::enoki_msg_payload_pnt_err;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("pnt_err: {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                let guard = RQLockGuard{random_data: PhantomData};
                sched.pnt_err((*payload_data).cpu, (*payload_data).pid, (*payload_data).err, None, guard);
            }
            c::MSG_BALANCE_ERR => {
                let payload_data = payload as *mut c::enoki_msg_payload_balance_err;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("balance_err: {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                let guard = RQLockGuard{random_data: PhantomData};
                sched.balance_err((*payload_data).cpu, (*payload_data).pid, (*payload_data).err, None, guard);
            }
            c::MSG_TASK_DEAD => {
                let payload_data = payload as *const c::enoki_msg_payload_task_dead;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("dead: {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                let guard = RQLockGuard{random_data: PhantomData};
                sched.task_dead((*payload_data).pid, guard);
            }
            c::MSG_TASK_BLOCKED => {
                let payload_data = payload as *const c::enoki_msg_payload_task_blocked;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("blocked: {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                let guard = RQLockGuard{random_data: PhantomData};
                sched.task_blocked((*payload_data).pid, (*payload_data).runtime,
                    (*payload_data).cpu_seqnum,
                    (*payload_data).cpu, (*payload_data).from_switchto, guard);
            }
            c::MSG_TASK_WAKEUP => {
                let payload_data = payload as *const c::enoki_msg_payload_task_wakeup;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("wakeup: {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                // Maybe it's ok to have this for other cpus?
                let sched = Schedulable {
                    cpu: (*payload_data).wake_up_cpu as u32,
                    pid: (*payload_data).pid,
                };
                let guard = RQLockGuard{random_data: PhantomData};
                sched.task_wakeup((*payload_data).pid,
                    (*payload_data).deferrable > 0, (*payload_data).last_ran_cpu,
                    (*payload_data).wake_up_cpu, (*payload_data).waker_cpu,
                    sched, guard);
            }
            c::MSG_TASK_NEW => {
                let payload_data = payload as *const c::enoki_msg_payload_task_new;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("new: {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                // Tasks moved onto the scheduler can be scheduled anywhere
                let cpu = if (*payload_data).wake_up_cpu == -1 {
                    u32::MAX
                } else {
                    (*payload_data).wake_up_cpu as u32
                };
                let sched = Schedulable {
                    pid: (*payload_data).pid,
                    cpu: cpu,
                };
                let guard = RQLockGuard{random_data: PhantomData};
                sched.task_new((*payload_data).pid, (*payload_data).tgid, (*payload_data).runtime,
                    (*payload_data).runnable, (*payload_data).prio, sched, guard);
            }
            c::MSG_TASK_PREEMPT => {
                let payload_data = payload as *const c::enoki_msg_payload_task_preempt;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("preempt: {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                let sched = Schedulable {
                    cpu: (*payload_data).cpu as u32,
                    pid: (*payload_data).pid,
                };
                let guard = RQLockGuard{random_data: PhantomData};
                sched.task_preempt((*payload_data).pid, (*payload_data).runtime,
                    (*payload_data).cpu_seqnum, (*payload_data).cpu,
                    (*payload_data).from_switchto, (*payload_data).was_latched, sched, guard);
            }
            c::MSG_TASK_YIELD => {
                let payload_data = payload as *const c::enoki_msg_payload_task_yield;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("yield: {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                let sched = Schedulable {
                    cpu: (*payload_data).cpu as u32,
                    pid: (*payload_data).pid,
                };
                let guard = RQLockGuard{random_data: PhantomData};
                sched.task_yield((*payload_data).pid, (*payload_data).runtime,
                    (*payload_data).cpu_seqnum, (*payload_data).cpu,
                    (*payload_data).from_switchto, sched, guard);
            }
            c::MSG_TASK_DEPARTED => {
                let payload_data = payload as *const c::enoki_msg_payload_task_departed;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("departed: {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                let guard = RQLockGuard{random_data: PhantomData};
                sched.task_departed((*payload_data).pid, (*payload_data).cpu_seqnum,
                    (*payload_data).cpu, (*payload_data).from_switchto,
                    (*payload_data).was_current, guard);
            }
            c::MSG_TASK_SWITCHTO => {
                let payload_data = payload as *const c::enoki_msg_payload_task_switchto;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("switchto: {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                sched.task_switchto((*payload_data).pid, (*payload_data).runtime,
                    (*payload_data).cpu_seqnum, (*payload_data).cpu);
            }
            c::MSG_TASK_AFFINITY_CHANGED => {
                let payload_data = payload as *const c::enoki_msg_payload_task_affinity_changed;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("affinity: {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                sched.task_affinity_changed((*payload_data).pid, (*payload_data).cpumask);
            }
            c::MSG_TASK_LATCHED => {
                let payload_data = payload as *const c::enoki_msg_payload_task_latched;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("latched: {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                sched.task_latched((*payload_data).pid, (*payload_data).commit_time,
                    (*payload_data).cpu_seqnum, (*payload_data).cpu,
                    (*payload_data).latched_preempt);
            }
            c::MSG_TASK_PRIO_CHANGED => {
                let payload_data = payload as *const c::enoki_msg_payload_task_prio_changed;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("prio_changed: {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                // Tasks moved onto the scheduler can be scheduled anywhere
                let guard = RQLockGuard{random_data: PhantomData};
                sched.task_prio_changed((*payload_data).pid, (*payload_data).prio, guard);
            }
            c::MSG_CPU_TICK => {
                let payload_data = payload as *const c::enoki_msg_payload_cpu_tick;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("tick: {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                let guard = RQLockGuard{random_data: PhantomData};
                sched.task_tick((*payload_data).cpu, (*payload_data).queued != 0, guard);
            }
            c::MSG_CPU_NOT_IDLE => {
                let payload_data = payload as *const c::enoki_msg_payload_cpu_not_idle;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("not_idle: {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                sched.cpu_not_idle((*payload_data).cpu, (*payload_data).next_pid);
            }
            c::MSG_TASK_SELECT_RQ => {
                let payload_data = payload as *mut c::enoki_msg_payload_select_task_rq;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("select_rq: {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                let cpu = sched.select_task_rq((*payload_data).pid, (*payload_data).waker_cpu, (*payload_data).prev_cpu);
                let sched = Schedulable {
                    cpu: cpu as u32,
                    pid: (*payload_data).pid,
                };
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("select_rq_ret: {:?} {} {}\n\0",
                                                       curr, (*payload_data).pid, cpu);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                sched.selected_task_rq(sched);
                (*payload_data).ret_cpu = cpu;
            }
            c::MSG_TASK_MIGRATE_RQ => {
                let payload_data = payload as *const c::enoki_msg_payload_migrate_task_rq;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("migrate_rq: {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                let sched = Schedulable {
                    pid: (*payload_data).pid,
                    cpu: (*payload_data).new_cpu as u32,
                };
                let guard = RQLockGuard{random_data: PhantomData};
                sched.migrate_task_rq((*payload_data).pid, sched, guard);
            }
            c::MSG_BALANCE => {
                let payload_data = payload as *mut c::enoki_msg_payload_balance;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("balance: {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                let guard = RQLockGuard{random_data: PhantomData};
                let next_pid = sched.balance((*payload_data).cpu, guard);
                (*payload_data).do_move = next_pid.is_some();
                (*payload_data).move_pid = next_pid.unwrap_or_default();
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("balance_ret: {:?} {} {:?}\n\0",
                                                       curr, (*payload_data).cpu, next_pid);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
            }
            c::MSG_REREGISTER_PREPARE => {
                let payload_data = payload as *mut c::enoki_msg_payload_rereg_prep;
                let data = sched.reregister_prepare();
                (*payload_data).data = Box::into_raw(Box::new(data)) as *mut _ as *mut raw::c_void;
            }
            c::MSG_REREGISTER_INIT => {
                let payload_data = payload as *const c::enoki_msg_payload_rereg_init;
                let data = if (*payload_data).data.is_null() {
                    None
                } else {
                    unsafe {
                        Some(*Box::from_raw((*payload_data).data as *mut TransferIn))
                    }
                };
                sched.reregister_init(data);
            }
            c::MSG_MSG_SIZE => {
                let payload_data = payload as *mut c::enoki_msg_payload_msg_size;
                (*payload_data).msg_size = core::mem::size_of::<UserMessage>() as u32;
            }
            c::MSG_REV_MSG_SIZE => {
                let payload_data = payload as *mut c::enoki_msg_payload_msg_size;
                (*payload_data).msg_size = core::mem::size_of::<RevMessage>() as u32;
            }
            c::MSG_CREATE_QUEUE => {
                let payload_data = payload as *mut c::enoki_msg_payload_create_queue;
                let q = unsafe { RingBuffer::from_raw((*payload_data).q, sched.get_policy()) };
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("create_queue {:?}\n\0", curr);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                let id = sched.register_queue((*payload_data).pid, q);
                (*payload_data).id = id;
            }
            c::MSG_CREATE_REV_QUEUE => {
                let payload_data = payload as *mut c::enoki_msg_payload_create_queue;
                let q = unsafe { RingBuffer::from_raw((*payload_data).q, sched.get_policy()) };
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("create_reverse_queue {:?}\n\0", curr);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                let id = sched.register_reverse_queue((*payload_data).pid, q);
                (*payload_data).id = id;
            }
            c::MSG_ENTER_QUEUE => {
                let payload_data = payload as *const c::enoki_msg_payload_enter_queue;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("enter_queue: {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                sched.enter_queue((*payload_data).id, (*payload_data).entries);
            }
            c::MSG_UNREGISTER_QUEUE => {
                // I'm like 60% sure this won't try to free the queue and will let linux do it.
                let payload_data = payload as *const c::enoki_msg_payload_unreg_queue;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("unregister_queue {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                sched.unregister_queue((*payload_data).id);
            }
            c::MSG_UNREGISTER_REV_QUEUE => {
                // I'm like 60% sure this won't try to free the queue and will let linux do it.
                let payload_data = payload as *const c::enoki_msg_payload_unreg_queue;
                #[cfg(feature = "record")]
                {
                    let curr = unsafe {
                        ffi::rs_current()
                    };
                    let mut write_str = alloc::format!("unregister_reverse_queue {:?} {:?}\n\0", curr, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                sched.unregister_rev_queue((*payload_data).id);
            }
            c::MSG_SEND_HINT => {
                let payload_data = payload as *const c::enoki_msg_payload_send_hint;
                let arg = (*payload_data).arg as *const UserMessage;
                #[cfg(feature = "record")]
                {
                    print_hint(*arg, sched.get_policy());
                }
                sched.parse_hint(*arg);
            }
            _ => {
                println!("Unsupported message type");
            }
        }
    }
}

unsafe fn print_hint<T: Serialize>(arg: T, policy: i32) {
            let pid = unsafe {
                ffi::current_pid()
            };
            let mut buf = [0u8; 128];
            postcard::to_slice(&arg, &mut buf);
            let num1 = u128::from_be_bytes(buf[0..16].try_into().unwrap());
            let num2 = u128::from_be_bytes(buf[16..32].try_into().unwrap());
            let num3 = u128::from_be_bytes(buf[32..48].try_into().unwrap());
            let num4 = u128::from_be_bytes(buf[48..64].try_into().unwrap());
            let num5 = u128::from_be_bytes(buf[64..80].try_into().unwrap());
            let num6 = u128::from_be_bytes(buf[80..96].try_into().unwrap());
            let num7 = u128::from_be_bytes(buf[96..112].try_into().unwrap());
            let num8 = u128::from_be_bytes(buf[112..128].try_into().unwrap());
            let mut write_str = alloc::format!("dequeue2: {} {} {} {} {} {} {} {} {}\n\0",
                                               pid, num1, num2, num3, num4, num5, num6,
                                               num7, num8);
            c::printk_deferred(write_str.as_ptr() as *const i8);
            c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
}

/// BentoScheduler trait
///
/// This trait is derived from the Filesystem trait from the fuse Rust crate.
///
/// This trait must be implemented to provide a Bento scheduler.
pub trait BentoScheduler<'a, 'b, TransferIn: Send, TransferOut: Send, UserMessage: Send + Copy + Serialize + Deserialize<'a>,
    RevMessage: Send + Copy + Serialize + Deserialize<'b>> {
    fn get_policy(&self) -> i32;
    /// Register the filesystem with Bento.
    ///
    /// This should be called when the filesystem module is inserted and before
    /// a filesystem is mounted.
    fn register(&self) -> i32
    where
        Self: core::marker::Sized,
    {
        let mut path = c::path::default();
        unsafe {
            let ret = register_enoki_sched(
                self as *const Self as *const raw::c_void,
                self.get_policy(),
                parse_message::<TransferIn, TransferOut, UserMessage, RevMessage, Self> as *const raw::c_void
            );
            return ret;
        }
    }

    fn reregister(&self) -> i32
    where
        Self: core::marker::Sized,
    {
        return unsafe {
            reregister_enoki_sched(
                self as *const Self as *const raw::c_void,
                self.get_policy(),
                parse_message::<TransferIn, TransferOut, UserMessage, RevMessage, Self> as *const raw::c_void
            )
        };
    }

    fn unregister(&self) -> i32 {
        return unsafe {
            unregister_enoki_sched(self as *const Self as *const raw::c_void)
        };
    }

    fn pick_next_task(
        &self,
        _cpu: i32,
        _curr_sched: Option<Schedulable>,
        _curr_runtime: Option<u64>,
        _guard: RQLockGuard
    ) -> Option<Schedulable> {
        None
    }

    fn pnt_err(
        &self,
        _cpu: i32,
        _pid: u64,
        _err: i32,
        _sched: Option<Schedulable>,
        _guard: RQLockGuard
    ) {}

    fn balance_err(
        &self,
        _cpu: i32,
        _pid: u64,
        _err: i32,
        _sched: Option<Schedulable>,
        _guard: RQLockGuard
    ) {}

    fn task_dead(&self, _pid: u64, _guard: RQLockGuard) {}

    fn task_blocked(
        &self,
        _pid: u64,
        _runtime: u64,
        _cpu_seqnum: u64,
        _cpu: i32,
        _from_switchto: i8,
        _guard: RQLockGuard
    ) {}

    fn task_wakeup(
        &self,
        _pid: u64,
        _deferrable: bool,
        _last_run_cpu: i32,
        _wake_up_cpu: i32,
        _waker_cpu: i32,
        _sched: Schedulable,
        _guard: RQLockGuard
    ) {}

    fn task_new(
        &self,
        _pid: u64,
        _tgid: u64,
        _runtime: u64,
        _runnable: u16,
        _prio: i32,
        _sched: Schedulable,
        _guard: RQLockGuard
    ) {}

    fn task_preempt(
        &self,
        _pid: u64,
        _runtime: u64,
        _cpu_seqnum: u64,
        _cpu: i32,
        _from_switchto: i8,
        _was_latched: i8,
        _sched: Schedulable,
        _guard: RQLockGuard
    ) {}

    fn task_yield(
        &self,
        _pid: u64,
        _runtime: u64,
        _cpu_seqnum: u64,
        _cpu: i32,
        _from_switchto: i8,
        _sched: Schedulable,
        _guard: RQLockGuard
    ) {}

    fn task_departed(
        &self,
        _pid: u64,
        _cpu_seqnum: u64,
        _cpu: i32,
        _from_switchto: i8,
        _was_current: i8,
        _guard: RQLockGuard
    ) -> Schedulable;

    fn task_switchto(
        &self,
        _pid: u64,
        _runtime: u64,
        _cpu_seqnum: u64,
        _cpu: i32,
    ) {}

    fn task_affinity_changed(&self, _pid: u64, _cpumask: u64) {}

    fn task_latched(
        &self,
        _pid: u64,
        _commit_time: u64,
        _cpu_seqnum: u64,
        _cpu: i32,
        _latched_preempt: i8
    ) {}

    fn task_prio_changed(
        &self,
        _pid: u64,
        _prio: i32,
        _guard: RQLockGuard
    ) {}

    fn task_tick(&self, _cpu: i32, _queued: bool, _guard: RQLockGuard) {}

    fn cpu_not_idle(&self, _cpu: i32, _next_pid: u64) {}

    fn select_task_rq(&self, _pid: u64, _waker_cpu: i32, _prev_cpu: i32) -> i32 { 0 }

    fn selected_task_rq(&self, _sched: Schedulable) {}
    
    fn migrate_task_rq(&self, _pid: u64, _sched: Schedulable, _guard: RQLockGuard) -> Schedulable;

    fn balance(&self, _cpu: i32, _guard: RQLockGuard) -> Option<u64> { None }

    fn reregister_prepare(&mut self) -> Option<TransferOut> {
        None
    }

    fn reregister_init(&mut self, Option<TransferIn>) {}

    fn register_queue(&self, pid: u64, RingBuffer<UserMessage>) -> i32;

    fn register_reverse_queue(&self, pid: u64, RingBuffer<RevMessage>) -> i32;

    fn enter_queue(&self, id: i32, _entries: u32) {}

    fn unregister_queue(&self, id: i32) -> RingBuffer<UserMessage>;

    fn unregister_rev_queue(&self, id: i32) -> RingBuffer<RevMessage>;

    fn parse_hint(&self, hint: UserMessage) {}
}
