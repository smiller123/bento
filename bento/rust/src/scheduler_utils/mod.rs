pub mod ringbuffer;
pub mod hrtick;

use alloc::boxed::Box;
use libc::ENOSYS;
use libc;
use kernel::ffi;

use time::Timespec;

use bindings as c;
use bindings::{register_ghost_agent,unregister_ghost_agent,reregister_ghost_agent};
use kernel::raw;

use self::ringbuffer::RingBuffer;

use core::fmt::Debug;
use core::convert::TryInto;

use serde::{Serialize, Deserialize};

pub const BENTO_KERNEL_VERSION: u32 = 1;
pub const BENTO_KERNEL_MINOR_VERSION: u32 = 0;

#[derive(Clone, Copy, Default, Debug)]
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
    UserMessage: Copy + Serialize + Deserialize<'a>,
    RevMessage: Copy + Serialize + Deserialize<'b>,
    T: BentoScheduler<'a, 'b, TransferIn, TransferOut, UserMessage, RevMessage>>(
//pub extern "C" fn parse_message<T: BentoScheduler> (
    agent: &mut T,
    type_: i32,
    msglen: i32,
    barrier: u32,
    payload: *mut raw::c_void,
    payload_size: i32,
    retval: *mut i32)
{
    unsafe {
        //let write_str = alloc::format!("create {}\0", id);
        //let mut write_ptr: i64 = 0;
        //let ret = unsafe {
        //    c::kernel_write(file, write_str.as_ptr() as *const raw::c_void, write_str.as_bytes().len(), &mut write_ptr as *mut i64)
        //};
        match type_ as u32 {
            c::MSG_PNT => {
                let payload_data = payload as *mut c::ghost_msg_payload_pnt;
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("pnt: {} {:?}\n\0", pid, *payload_data);
                    //c::printk_deferred(write_str.as_ptr() as *const i8);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                let next_task = agent.pick_next_task((*payload_data).cpu);
                if (next_task.is_none() ||
                    next_task.unwrap().cpu == (*payload_data).cpu as u32 ||
                    next_task.unwrap().cpu == u32::MAX) {
                    (*payload_data).pick_task = next_task.is_some();
                    (*payload_data).ret_pid = next_task.unwrap_or_default().get_pid();
                    #[cfg(feature = "record")]
                    {
                        let pid = unsafe {
                            ffi::current_pid()
                        };
                        let mut write_str = alloc::format!("pnt_ret: {} {} {:?}\n\0",
                                                           pid, (*payload_data).cpu, next_task);
                        c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                    }
                } else {
                    // The process can't be scheduled on this cpu

                    let sched = next_task.unwrap();
                    (*payload_data).pick_task = false;
                    agent.pnt_err(sched);
                    #[cfg(feature = "record")]
                    {
                        let pid = unsafe {
                            ffi::current_pid()
                        };
                        let mut write_str = alloc::format!("pnt_ret: {} {} {:?} Error\n\0",
                                                           pid, (*payload_data).cpu, next_task);
                        c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                    }
                }
            }
            c::MSG_TASK_DEAD => {
                let payload_data = payload as *const c::ghost_msg_payload_task_dead;
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("dead: {} {:?}\n\0", pid, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                agent.task_dead((*payload_data).pid);
            }
            c::MSG_TASK_BLOCKED => {
                let payload_data = payload as *const c::ghost_msg_payload_task_blocked;
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("blocked: {} {:?}\n\0", pid, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                let sched = Schedulable {
                    cpu: (*payload_data).cpu as u32,
                    pid: (*payload_data).pid,
                };
                agent.task_blocked((*payload_data).pid, (*payload_data).runtime,
                    (*payload_data).cpu_seqnum,
                    (*payload_data).cpu, (*payload_data).from_switchto, sched);
            }
            c::MSG_TASK_WAKEUP => {
                let payload_data = payload as *const c::ghost_msg_payload_task_wakeup;
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("wakeup: {} {:?}\n\0", pid, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                // Maybe it's ok to have this for other cpus?
                let sched = Schedulable {
                    cpu: (*payload_data).wake_up_cpu as u32,
                    pid: (*payload_data).pid,
                };
                agent.task_wakeup((*payload_data).pid, (*payload_data).agent_data,
                    (*payload_data).deferrable > 0, (*payload_data).last_ran_cpu,
                    (*payload_data).wake_up_cpu, (*payload_data).waker_cpu,
                    sched);
            }
            c::MSG_TASK_NEW => {
                let payload_data = payload as *const c::ghost_msg_payload_task_new;
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("new: {} {:?}\n\0", pid, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                // Tasks moved onto the scheduler can be scheduled anywhere
                let sched = Schedulable {
                    pid: (*payload_data).pid,
                    cpu: u32::MAX,
                };
                agent.task_new((*payload_data).pid, (*payload_data).runtime, (*payload_data).runnable, sched);
            }
            c::MSG_TASK_PREEMPT => {
                let payload_data = payload as *const c::ghost_msg_payload_task_preempt;
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("preempt: {} {:?}\n\0", pid, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                let sched = Schedulable {
                    cpu: (*payload_data).cpu as u32,
                    pid: (*payload_data).pid,
                };
                agent.task_preempt((*payload_data).pid, (*payload_data).runtime,
                    (*payload_data).cpu_seqnum, (*payload_data).cpu,
                    (*payload_data).from_switchto, (*payload_data).was_latched, sched);
            }
            c::MSG_TASK_YIELD => {
                let payload_data = payload as *const c::ghost_msg_payload_task_yield;
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("yield: {} {:?}\n\0", pid, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                agent.task_yield((*payload_data).pid, (*payload_data).runtime,
                    (*payload_data).cpu_seqnum, (*payload_data).cpu,
                    (*payload_data).from_switchto);
            }
            c::MSG_TASK_DEPARTED => {
                let payload_data = payload as *const c::ghost_msg_payload_task_departed;
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("departed: {} {:?}\n\0", pid, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                agent.task_departed((*payload_data).pid, (*payload_data).cpu_seqnum,
                    (*payload_data).cpu, (*payload_data).from_switchto,
                    (*payload_data).was_current);
            }
            c::MSG_TASK_SWITCHTO => {
                let payload_data = payload as *const c::ghost_msg_payload_task_switchto;
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("switchto: {} {:?}\n\0", pid, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                agent.task_switchto((*payload_data).pid, (*payload_data).runtime,
                    (*payload_data).cpu_seqnum, (*payload_data).cpu);
            }
            c::MSG_TASK_AFFINITY_CHANGED => {
                let payload_data = payload as *const c::ghost_msg_payload_task_affinity_changed;
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("affinity: {} {:?}\n\0", pid, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                agent.task_affinity_changed((*payload_data).pid);
            }
            c::MSG_TASK_LATCHED => {
                let payload_data = payload as *const c::ghost_msg_payload_task_latched;
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("latched: {} {:?}\n\0", pid, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                agent.task_latched((*payload_data).pid, (*payload_data).commit_time,
                    (*payload_data).cpu_seqnum, (*payload_data).cpu,
                    (*payload_data).latched_preempt);
            }
            c::MSG_CPU_TICK => {
                let payload_data = payload as *const c::ghost_msg_payload_cpu_tick;
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("tick: {} {:?}\n\0", pid, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                agent.task_tick((*payload_data).cpu, (*payload_data).queued != 0);
            }
            c::MSG_CPU_NOT_IDLE => {
                let payload_data = payload as *const c::ghost_msg_payload_cpu_not_idle;
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("not_idle: {} {:?}\n\0", pid, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                agent.cpu_not_idle((*payload_data).cpu, (*payload_data).next_pid);
            }
            c::MSG_TASK_SELECT_RQ => {
                let payload_data = payload as *mut c::ghost_msg_payload_select_task_rq;
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("select_rq: {} {:?}\n\0", pid, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                let cpu = agent.select_task_rq((*payload_data).pid);
                let sched = Schedulable {
                    cpu: cpu as u32,
                    pid: (*payload_data).pid,
                };
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("select_rq_ret: {} {} {}\n\0",
                                                       pid, (*payload_data).pid, cpu);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                agent.selected_task_rq(sched);
                (*payload_data).ret_cpu = cpu;
            }
            c::MSG_TASK_MIGRATE_RQ => {
                let payload_data = payload as *const c::ghost_msg_payload_migrate_task_rq;
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("migrate_rq: {} {:?}\n\0", pid, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                let sched = Schedulable {
                    pid: (*payload_data).pid,
                    cpu: (*payload_data).new_cpu as u32,
                };
                //agent.migrate_task_rq((*payload_data).pid, (*payload_data).new_cpu);
                agent.migrate_task_rq((*payload_data).pid, sched);
            }
            c::MSG_BALANCE => {
                let payload_data = payload as *mut c::ghost_msg_payload_balance;
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("balance: {} {:?}\n\0", pid, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                let next_pid = agent.balance((*payload_data).cpu);
                (*payload_data).do_move = next_pid.is_some();
                (*payload_data).move_pid = next_pid.unwrap_or_default();
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("balance_ret: {} {} {:?}\n\0",
                                                       pid, (*payload_data).cpu, next_pid);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
            }
            c::MSG_REREGISTER_PREPARE => {
                let payload_data = payload as *mut c::ghost_msg_payload_rereg_prep;
                let data = agent.reregister_prepare();
                (*payload_data).data = Box::into_raw(Box::new(data)) as *mut _ as *mut raw::c_void;
            }
            c::MSG_REREGISTER_INIT => {
                let payload_data = payload as *const c::ghost_msg_payload_rereg_init;
                let data = if (*payload_data).data.is_null() {
                    None
                } else {
                    unsafe {
                        Some(*Box::from_raw((*payload_data).data as *mut TransferIn))
                    }
                };
                agent.reregister_init(data);
            }
            c::MSG_MSG_SIZE => {
                let payload_data = payload as *mut c::ghost_msg_payload_msg_size;
                //let next_pid = agent.balance((*payload_data).cpu);
                (*payload_data).msg_size = core::mem::size_of::<UserMessage>() as u32;
                //(*payload_data).move_pid = next_pid.unwrap_or_default();
            }
            c::MSG_REV_MSG_SIZE => {
                let payload_data = payload as *mut c::ghost_msg_payload_msg_size;
                //let next_pid = agent.balance((*payload_data).cpu);
                (*payload_data).msg_size = core::mem::size_of::<RevMessage>() as u32;
                //(*payload_data).move_pid = next_pid.unwrap_or_default();
            }
            c::MSG_CREATE_QUEUE => {
                let payload_data = payload as *const c::ghost_msg_payload_create_queue;
                //println!("q ptr {:?}", (*payload_data).q);
                let q = unsafe { RingBuffer::from_raw((*payload_data).q, agent.get_policy()) };
                //let q = unsafe { &mut*((*payload_data).q as *mut RingBuffer<UserMessage>) };
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("create_queue {}\n\0", pid);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                agent.register_queue(q);
            }
            c::MSG_CREATE_REV_QUEUE => {
                let payload_data = payload as *const c::ghost_msg_payload_create_queue;
                //println!("q ptr {:?}", (*payload_data).q);
                let q = unsafe { RingBuffer::from_raw((*payload_data).q, agent.get_policy()) };
                //let q = unsafe { &mut*((*payload_data).q as *mut RingBuffer<UserMessage>) };
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("create_reverse_queue {}\n\0", pid);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                agent.register_reverse_queue(q);
            }
            c::MSG_ENTER_QUEUE => {
                let payload_data = payload as *const c::ghost_msg_payload_enter_queue;
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("enter_queue: {} {:?}\n\0", pid, *payload_data);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                agent.enter_queue((*payload_data).entries);
            }
            c::MSG_UNREGISTER_QUEUE => {
                //let payload_data = payload as *const c::ghost_msg_payload_enter_queue;
                // I'm like 60% sure this won't try to free the queue and will let linux do it.
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("unregister_queue {}\n\0", pid);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                agent.unregister_queue();
            }
            c::MSG_UNREGISTER_REV_QUEUE => {
                //let payload_data = payload as *const c::ghost_msg_payload_enter_queue;
                // I'm like 60% sure this won't try to free the queue and will let linux do it.
                #[cfg(feature = "record")]
                {
                    let pid = unsafe {
                        ffi::current_pid()
                    };
                    let mut write_str = alloc::format!("unregister_reverse_queue {}\n\0", pid);
                    c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                }
                agent.unregister_rev_queue();
            }
            c::MSG_SEND_HINT => {
                let payload_data = payload as *const c::ghost_msg_payload_send_hint;
                let arg = (*payload_data).arg as *const UserMessage;
                #[cfg(feature = "record")]
                {
                    print_hint(*arg, agent.get_policy());
                }
                //let mut write_str = alloc::format!("send_hint: {:?}\n\0", *arg);
                //c::printk_deferred(write_str.as_ptr() as *const i8);
                //c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
                agent.parse_hint(*arg);
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
pub trait BentoScheduler<'a, 'b, TransferIn: Send, TransferOut: Send, UserMessage: Copy + Serialize + Deserialize<'a>,
    RevMessage: Copy + Serialize + Deserialize<'b>> {
    fn get_policy(&self) -> i32;
    /// Register the filesystem with Bento.
    ///
    /// This should be called when the filesystem module is inserted and before
    /// a filesystem is mounted.
    fn register(&self) -> i32
    where
        Self: core::marker::Sized,
    {
        //let name = "/sys/fs/ghost/ctl\0";
        let mut path = c::path::default();
        unsafe {
            //ffi::rs_kern_path(record_file, libc::O_WRONLY as u32, &mut path as *mut c::path);
            //let file = c::dentry_open(&path, libc::O_WRONLY | libc::O_APPEND | libc::O_NONBLOCK, ffi::rs_current_cred());
            //        let write_str = "hi\n\0";
            //        let mut write_ptr: i64 = (*file).f_pos;
            //        println!("write ptr {:?}", write_ptr);
            //        println!("inode {:?}", (*file).f_inode);
            //        println!("path {:?}", (*file).f_path);
            //        println!("write str {:?}", write_str);
            //        println!("write str len {:?}", write_str.as_bytes().len());
            //        let ret =
            //            c::kernel_write(file, write_str.as_ptr() as *const raw::c_void,
            //                write_str.as_bytes().len() - 1, &mut write_ptr as *mut i64);
            //println!("record file ptr {:?}", file);
            let ret = register_ghost_agent(
                self as *const Self as *const raw::c_void,
                self.get_policy(),
                parse_message::<TransferIn, TransferOut, UserMessage, RevMessage, Self> as *const raw::c_void
            );
            let mut write_str = alloc::format!("loading\n\0");
            c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
            return ret;
        }
    }

    fn reregister(&self) -> i32
    where
        Self: core::marker::Sized,
    {
        return unsafe {
            reregister_ghost_agent(
                self as *const Self as *const raw::c_void,
                self.get_policy(),
                parse_message::<TransferIn, TransferOut, UserMessage, RevMessage, Self> as *const raw::c_void
            )
        };
    }

    fn unregister(&self) -> i32 {
        return unsafe {
            unregister_ghost_agent(self as *const Self as *const raw::c_void)
        };
    }

    //fn bento_update_prepare(&mut self) -> Option<TransferOut> {
    ////fn bento_update_prepare(&mut self) -> Option<*const raw::c_void> {
    //    None
    //}

    //fn bento_update_transfer(&mut self, Option<TransferIn>) { }
    ////fn bento_update_transfer(&mut self, Option<*const raw::c_void>) { }

    /// Initialize the file system and fill in initialization flags.
    ///
    /// Possible initialization flags are defined /include/uapi/linux/fuse.h.
    /// No support is provided for readdirplus and async DIO.
    ///
    /// Arguments:
    /// * `req: &Request` - Request data structure.
    /// * `devname: &OsStr` - Name of the backing device file.
    /// * `fc_info: &mut FuseConnInfo` - Connection information used to pass initialization
    /// arguments to Bento.
    //fn init(
    //    &mut self,
    //    _req: &Request,
    //    _devname: &OsStr,
    //    _fc_info: &mut FuseConnInfo,
    //) -> Result<(), i32> {
    //    return Err(ENOSYS);
    //}

    ///// Perform any necessary cleanup on the file system.
    /////
    ///// Arguments:
    ///// * `req: &Request` - Request data structure.
    //fn bento_destroy(&mut self, _req: &Request) {}
    
    fn pick_next_task(
        &self,
        _cpu: i32,
    ) -> Option<Schedulable> {
        None
    }

    fn pnt_err(&self, _sched: Schedulable) {}

    fn task_dead(&self, _pid: u64) {}

    fn task_blocked(
        &self,
        _pid: u64,
        _runtime: u64,
        _cpu_seqnum: u64,
        _cpu: i32,
        _from_switchto: i8,
        _sched: Schedulable,
    ) {}

    fn task_wakeup(
        &self,
        _pid: u64,
        _agent_data: u64,
        _deferrable: bool,
        _last_run_cpu: i32,
        _wake_up_cpu: i32,
        _waker_cpu: i32,
        _sched: Schedulable,
    ) {}

    fn task_new(
        &self,
        _pid: u64,
        _runtime: u64,
        _runnable: u16,
        _sched: Schedulable,
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
    ) {}

    fn task_yield(
        &self,
        _pid: u64,
        _runtime: u64,
        _cpu_seqnum: u64,
        _cpu: i32,
        _from_switchto: i8
    ) {}

    fn task_departed(
        &self,
        _pid: u64,
        _cpu_seqnum: u64,
        _cpu: i32,
        _from_switchto: i8,
        _was_current: i8
    ) {}

    fn task_switchto(
        &self,
        _pid: u64,
        _runtime: u64,
        _cpu_seqnum: u64,
        _cpu: i32,
    ) {}

    fn task_affinity_changed(&self, _pid: u64) {}

    fn task_latched(
        &self,
        _pid: u64,
        _commit_time: u64,
        _cpu_seqnum: u64,
        _cpu: i32,
        _latched_preempt: i8
    ) {}

    fn task_tick(&self, _cpu: i32, _queued: bool) {}

    fn cpu_not_idle(&self, _cpu: i32, _next_pid: u64) {}

    fn select_task_rq(&self, _pid: u64) -> i32 { 0 }

    fn selected_task_rq(&self, _sched: Schedulable) {}
    
    //fn migrate_task_rq(&self, _pid: u64, _new_cpu: i32) {}
    fn migrate_task_rq(&self, _pid: u64, _sched: Schedulable) {}

    fn balance(&self, _cpu: i32) -> Option<u64> { None }

    //fn bento_update_prepare(&mut self) -> Option<TransferOut> {
    fn reregister_prepare(&mut self) -> Option<TransferOut> {
        None
    }

    fn reregister_init(&mut self, Option<TransferIn>) {}

    fn register_queue(&self, RingBuffer<UserMessage>) {}

    fn register_reverse_queue(&self, RingBuffer<RevMessage>) {}

    fn enter_queue(&self, _entries: u32) {}

    fn unregister_queue(&self) -> RingBuffer<UserMessage>;

    fn unregister_rev_queue(&self) -> RingBuffer<RevMessage>;

    fn parse_hint(&self, hint: UserMessage) {}
}
