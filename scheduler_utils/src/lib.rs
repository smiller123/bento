extern crate core;
extern crate alloc;
extern crate serde;
extern crate once_cell;
extern crate num_cpus;

pub mod ringbuffer;
pub mod spin_rs;
pub mod hrtick;
pub mod sched_core;
pub mod cpu;

use serde::{Serialize, Deserialize};

//use alloc::boxed::Box;
//use libc::ENOSYS;
//use libc;
//use kernel::ffi;
//
//use time::Timespec;
//
//use bindings as c;
//use bindings::{register_ghost_agent,unregister_ghost_agent,reregister_ghost_agent};
//use kernel::raw;

use self::ringbuffer::RingBuffer;

use core::fmt::Debug;
use core::marker::PhantomData;

pub struct RQLockGuard {
    pub random_data: PhantomData<i32>,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Schedulable {
    pub pid: u64,
    pub cpu: u32
}

impl Schedulable {
    pub fn get_cpu(&self) -> u32 {
        self.cpu
    }

    pub fn get_pid(&self) -> u64 {
        self.pid
    }
}


//pub const BENTO_KERNEL_VERSION: u32 = 1;
//pub const BENTO_KERNEL_MINOR_VERSION: u32 = 0;

//pub fn parse_message<TransferIn: Send, TransferOut: Send, UserMessage: Copy + Debug,
//    T: BentoScheduler<TransferIn, TransferOut, UserMessage>>(
//    agent: &mut T,
//    type_: i32,
//    msglen: i32,
//    barrier: u32,
//    payload: *mut raw::c_void,
//    payload_size: i32,
//    retval: *mut i32)
//{
//    unsafe {
//        match type_ as u32 {
//            c::MSG_PNT => {
//                let payload_data = payload as *mut c::ghost_msg_payload_pnt;
//                let mut write_str = alloc::format!("pnt: {:?}\n\0", *payload_data);
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//                let next_task = agent.pick_next_task((*payload_data).cpu);
//                (*payload_data).pick_task = next_task.is_some();
//                (*payload_data).ret_pid = next_task.unwrap_or_default();
//                let mut write_str = alloc::format!("pnt ret: {:?}\n\0", next_task);
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//            }
//            c::MSG_TASK_DEAD => {
//                let payload_data = payload as *const c::ghost_msg_payload_task_dead;
//                let mut write_str = alloc::format!("dead: {:?}\n\0", *payload_data);
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//                agent.task_dead((*payload_data).pid);
//            }
//            c::MSG_TASK_BLOCKED => {
//                let payload_data = payload as *const c::ghost_msg_payload_task_blocked;
//                let mut write_str = alloc::format!("blocked: {:?}\n\0", *payload_data);
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//                agent.task_blocked((*payload_data).pid, (*payload_data).runtime,
//                    (*payload_data).cpu_seqnum,
//                    (*payload_data).cpu, (*payload_data).from_switchto);
//            }
//            c::MSG_TASK_WAKEUP => {
//                let payload_data = payload as *const c::ghost_msg_payload_task_wakeup;
//                let mut write_str = alloc::format!("wakeup: {:?}\n\0", *payload_data);
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//                agent.task_wakeup((*payload_data).pid, (*payload_data).agent_data,
//                    (*payload_data).deferrable > 0, (*payload_data).last_ran_cpu,
//                    (*payload_data).wake_up_cpu, (*payload_data).waker_cpu);
//            }
//            c::MSG_TASK_NEW => {
//                let payload_data = payload as *const c::ghost_msg_payload_task_new;
//                let mut write_str = alloc::format!("new: {:?}\n\0", *payload_data);
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//                agent.task_new((*payload_data).pid, (*payload_data).runtime, (*payload_data).runnable);
//            }
//            c::MSG_TASK_PREEMPT => {
//                let payload_data = payload as *const c::ghost_msg_payload_task_preempt;
//                let mut write_str = alloc::format!("preempt: {:?}\n\0", *payload_data);
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//                agent.task_preempt((*payload_data).pid, (*payload_data).runtime,
//                    (*payload_data).cpu_seqnum, (*payload_data).cpu,
//                    (*payload_data).from_switchto, (*payload_data).was_latched);
//            }
//            c::MSG_TASK_YIELD => {
//                let payload_data = payload as *const c::ghost_msg_payload_task_yield;
//                let mut write_str = alloc::format!("yield: {:?}\n\0", *payload_data);
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//                agent.task_yield((*payload_data).pid, (*payload_data).runtime,
//                    (*payload_data).cpu_seqnum, (*payload_data).cpu,
//                    (*payload_data).from_switchto);
//            }
//            c::MSG_TASK_DEPARTED => {
//                let payload_data = payload as *const c::ghost_msg_payload_task_departed;
//                let mut write_str = alloc::format!("departed: {:?}\n\0", *payload_data);
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//                agent.task_departed((*payload_data).pid, (*payload_data).cpu_seqnum,
//                    (*payload_data).cpu, (*payload_data).from_switchto,
//                    (*payload_data).was_current);
//            }
//            c::MSG_TASK_SWITCHTO => {
//                let payload_data = payload as *const c::ghost_msg_payload_task_switchto;
//                let mut write_str = alloc::format!("switchto: {:?}\n\0", *payload_data);
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//                agent.task_switchto((*payload_data).pid, (*payload_data).runtime,
//                    (*payload_data).cpu_seqnum, (*payload_data).cpu);
//            }
//            c::MSG_TASK_AFFINITY_CHANGED => {
//                let payload_data = payload as *const c::ghost_msg_payload_task_affinity_changed;
//                let mut write_str = alloc::format!("affinity: {:?}\n\0", *payload_data);
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//                agent.task_affinity_changed((*payload_data).pid);
//            }
//            c::MSG_TASK_LATCHED => {
//                let payload_data = payload as *const c::ghost_msg_payload_task_latched;
//                let mut write_str = alloc::format!("latched: {:?}\n\0", *payload_data);
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//                agent.task_latched((*payload_data).pid, (*payload_data).commit_time,
//                    (*payload_data).cpu_seqnum, (*payload_data).cpu,
//                    (*payload_data).latched_preempt);
//            }
//            c::MSG_CPU_TICK => {
//                let payload_data = payload as *const c::ghost_msg_payload_cpu_tick;
//                let mut write_str = alloc::format!("tick: {:?}\n\0", *payload_data);
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//                agent.cpu_tick((*payload_data).cpu);
//            }
//            c::MSG_CPU_NOT_IDLE => {
//                let payload_data = payload as *const c::ghost_msg_payload_cpu_not_idle;
//                let mut write_str = alloc::format!("not idle: {:?}\n\0", *payload_data);
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//                agent.cpu_not_idle((*payload_data).cpu, (*payload_data).next_pid);
//            }
//            c::MSG_TASK_SELECT_RQ => {
//                let payload_data = payload as *mut c::ghost_msg_payload_select_task_rq;
//                let mut write_str = alloc::format!("select rq: {:?}\n\0", *payload_data);
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//                let cpu = agent.select_task_rq((*payload_data).pid);
//                (*payload_data).ret_cpu = cpu;
//                let mut write_str = alloc::format!("select rq ret: {:?}\n\0", cpu);
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//            }
//            c::MSG_TASK_MIGRATE_RQ => {
//                let payload_data = payload as *const c::ghost_msg_payload_migrate_task_rq;
//                let mut write_str = alloc::format!("migrate rq: {:?}\n\0", *payload_data);
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//                agent.migrate_task_rq((*payload_data).pid, (*payload_data).new_cpu);
//            }
//            c::MSG_BALANCE => {
//                let payload_data = payload as *mut c::ghost_msg_payload_balance;
//                let mut write_str = alloc::format!("balance: {:?}\n\0", *payload_data);
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//                let next_pid = agent.balance((*payload_data).cpu);
//                (*payload_data).do_move = next_pid.is_some();
//                (*payload_data).move_pid = next_pid.unwrap_or_default();
//                let mut write_str = alloc::format!("balance ret: {:?}\n\0", next_pid);
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//            }
//            c::MSG_REREGISTER_PREPARE => {
//                let payload_data = payload as *mut c::ghost_msg_payload_rereg_prep;
//                let data = agent.reregister_prepare();
//                (*payload_data).data = Box::into_raw(Box::new(data)) as *mut _ as *mut raw::c_void;
//            }
//            c::MSG_REREGISTER_INIT => {
//                let payload_data = payload as *const c::ghost_msg_payload_rereg_init;
//                let data = if (*payload_data).data.is_null() {
//                    None
//                } else {
//                    unsafe { Some(*Box::from_raw((*payload_data).data as *mut TransferIn)) }
//                };
//                agent.reregister_init(data);
//            }
//            c::MSG_MSG_SIZE => {
//                let payload_data = payload as *mut c::ghost_msg_payload_msg_size;
//                //let next_pid = agent.balance((*payload_data).cpu);
//                (*payload_data).msg_size = core::mem::size_of::<UserMessage>() as u32;
//                //(*payload_data).move_pid = next_pid.unwrap_or_default();
//            }
//            c::MSG_CREATE_QUEUE => {
//                let payload_data = payload as *const c::ghost_msg_payload_create_queue;
//                println!("q ptr {:?}", (*payload_data).q);
//                let q = unsafe { RingBuffer::from_raw((*payload_data).q) };
//                //let q = unsafe { &mut*((*payload_data).q as *mut RingBuffer<UserMessage>) };
//                let mut write_str = alloc::format!("create queue\n\0");
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//                agent.register_queue(q);
//            }
//            c::MSG_ENTER_QUEUE => {
//                let payload_data = payload as *const c::ghost_msg_payload_enter_queue;
//                let mut write_str = alloc::format!("enter queue: {:?}\n\0", *payload_data);
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//                agent.enter_queue((*payload_data).entries);
//            }
//            c::MSG_UNREGISTER_QUEUE => {
//                //let payload_data = payload as *const c::ghost_msg_payload_enter_queue;
//                // I'm like 60% sure this won't try to free the queue and will let linux do it.
//                let mut write_str = alloc::format!("unregister queue\n\0");
//                c::printk_deferred(write_str.as_ptr() as *const i8);
//                c::file_write_deferred(agent.get_policy(), write_str.as_mut_ptr() as *mut i8);
//                agent.unregister_queue();
//            }
//            _ => {
//                println!("Unsupported message type");
//            }
//        }
//    }
//}
pub trait BentoScheduler<'a, 'b, TransferIn: Send, TransferOut: Send, UserMessage: Send + Copy + Serialize + Deserialize<'a>,
    RevMessage: Send + Copy + Serialize + Deserialize<'b>> {
    fn get_policy(&self) -> i32;
    /// Register the filesystem with Bento.
    ///
    /// This should be called when the filesystem module is inserted and before
    /// a filesystem is mounted.
    //fn register(&self) -> i32
    //where
    //    Self: core::marker::Sized,
    //{
    //    //let name = "/sys/fs/ghost/ctl\0";
    //    let mut path = c::path::default();
    //    unsafe {
    //        //ffi::rs_kern_path(record_file, libc::O_WRONLY as u32, &mut path as *mut c::path);
    //        //let file = c::dentry_open(&path, libc::O_WRONLY | libc::O_APPEND | libc::O_NONBLOCK, ffi::rs_current_cred());
    //        //        let write_str = "hi\n\0";
    //        //        let mut write_ptr: i64 = (*file).f_pos;
    //        //        println!("write ptr {:?}", write_ptr);
    //        //        println!("inode {:?}", (*file).f_inode);
    //        //        println!("path {:?}", (*file).f_path);
    //        //        println!("write str {:?}", write_str);
    //        //        println!("write str len {:?}", write_str.as_bytes().len());
    //        //        let ret =
    //        //            c::kernel_write(file, write_str.as_ptr() as *const raw::c_void,
    //        //                write_str.as_bytes().len() - 1, &mut write_ptr as *mut i64);
    //        //println!("record file ptr {:?}", file);
    //        let ret = register_ghost_agent(
    //            self as *const Self as *const raw::c_void,
    //            self.get_policy(),
    //            parse_message::<TransferIn, TransferOut, UserMessage, RevMessage, Self> as *const raw::c_void
    //        );
    //        //let mut write_str = alloc::format!("loading\n\0");
    //        //c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
    //        return ret;
    //    }
    //}

    //fn reregister(&self) -> i32
    //where
    //    Self: core::marker::Sized,
    //{
    //    return unsafe {
    //        reregister_ghost_agent(
    //            self as *const Self as *const raw::c_void,
    //            self.get_policy(),
    //            parse_message::<TransferIn, TransferOut, UserMessage, RevMessage, Self> as *const raw::c_void
    //        )
    //    };
    //}

    //fn unregister(&self) -> i32 {
    //    return unsafe {
    //        unregister_ghost_agent(self as *const Self as *const raw::c_void)
    //    };
    //}

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
        _agent_data: u64,
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
    
    //fn migrate_task_rq(&self, _pid: u64, _new_cpu: i32) {}
    fn migrate_task_rq(&self, _pid: u64, _sched: Schedulable, _guard: RQLockGuard) -> Schedulable;

    fn balance(&self, _cpu: i32, _guard: RQLockGuard) -> Option<u64> { None }

    //fn bento_update_prepare(&mut self) -> Option<TransferOut> {
    fn reregister_prepare(&mut self) -> Option<TransferOut> {
        None
    }

    fn reregister_init(&mut self, Option<TransferIn>) {}

    fn register_queue(&self, RingBuffer<UserMessage>) -> i32;

    fn register_reverse_queue(&self, RingBuffer<RevMessage>) -> i32;

    fn enter_queue(&self, id: i32, _entries: u32) {}

    fn unregister_queue(&self, id: i32) -> RingBuffer<UserMessage>;

    fn unregister_rev_queue(&self, id: i32) -> RingBuffer<RevMessage>;

    fn parse_hint(&self, hint: UserMessage) {}
}

///// BentoScheduler trait
/////
///// This trait is derived from the Filesystem trait from the fuse Rust crate.
/////
///// This trait must be implemented to provide a Bento scheduler.
//pub trait BentoScheduler<'a,  'b, TransferIn: Send, TransferOut: Send, UserMessage: Copy + Serialize + Deserialize<'a>,
//    RevMessage: Copy + Serialize + Deserialize<'a>> {
//    fn get_policy(&self) -> i32;
//    /// Register the filesystem with Bento.
//    ///
//    /// This should be called when the filesystem module is inserted and before
//    /// a filesystem is mounted.
//    //fn register(&self) -> i32
//    //where
//    //    Self: core::marker::Sized,
//    //{
//    //    let mut path = c::path::default();
//    //    unsafe {
//    //        let ret = register_ghost_agent(
//    //            self as *const Self as *const raw::c_void,
//    //            self.get_policy(),
//    //            parse_message::<TransferIn, TransferOut, UserMessage, Self> as *const raw::c_void
//    //        );
//    //        return ret;
//    //    }
//    //}
//
//    //fn reregister(&self) -> i32
//    //where
//    //    Self: core::marker::Sized,
//    //{
//    //    return unsafe {
//    //        reregister_ghost_agent(
//    //            self as *const Self as *const raw::c_void,
//    //            self.get_policy(),
//    //            parse_message::<TransferIn, TransferOut, UserMessage, Self> as *const raw::c_void
//    //        )
//    //    };
//    //}
//
//    //fn unregister(&self) -> i32 {
//    //    return unsafe {
//    //        unregister_ghost_agent(self as *const Self as *const raw::c_void)
//    //    };
//    //}
//
//    //fn bento_update_prepare(&mut self) -> Option<TransferOut> {
//    ////fn bento_update_prepare(&mut self) -> Option<*const raw::c_void> {
//    //    None
//    //}
//
//    fn pick_next_task(
//        &self,
//        _cpu: i32,
//    ) -> Option<Schedulable> {
//        None
//    }
//
//    fn pnt_err(&self, _sched: Schedulable) {}
//
//    fn task_dead(&self, _pid: u64) {}
//
//    fn task_blocked(
//        &self,
//        _pid: u64,
//        _runtime: u64,
//        _cpu_seqnum: u64,
//        _cpu: i32,
//        _from_switchto: i8,
//        _sched: Schedulable,
//    ) {}
//
//    fn task_wakeup(
//        &self,
//        _pid: u64,
//        _agent_data: u64,
//        _deferrable: bool,
//        _last_run_cpu: i32,
//        _wake_up_cpu: i32,
//        _waker_cpu: i32,
//        _sched: Schedulable,
//    ) {}
//
//    fn task_new(
//        &self,
//        _pid: u64,
//        _runtime: u64,
//        _runnable: u16,
//        _sched: Schedulable,
//    ) {}
//
//    fn task_preempt(
//        &self,
//        _pid: u64,
//        _runtime: u64,
//        _cpu_seqnum: u64,
//        _cpu: i32,
//        _from_switchto: i8,
//        _was_latched: i8,
//        _sched: Schedulable,
//    ) {}
//
//    fn task_yield(
//        &self,
//        _pid: u64,
//        _runtime: u64,
//        _cpu_seqnum: u64,
//        _cpu: i32,
//        _from_switchto: i8
//    ) {}
//
//    fn task_departed(
//        &self,
//        _pid: u64,
//        _cpu_seqnum: u64,
//        _cpu: i32,
//        _from_switchto: i8,
//        _was_current: i8
//    ) {}
//
//    fn task_switchto(
//        &self,
//        _pid: u64,
//        _runtime: u64,
//        _cpu_seqnum: u64,
//        _cpu: i32,
//    ) {}
//
//    fn task_affinity_changed(&self, _pid: u64) {}
//
//    fn task_latched(
//        &self,
//        _pid: u64,
//        _commit_time: u64,
//        _cpu_seqnum: u64,
//        _cpu: i32,
//        _latched_preempt: i8
//    ) {}
//
//    fn cpu_tick(&self, _cpu: i32) {}
//
//    fn cpu_not_idle(&self, _cpu: i32, _next_pid: u64) {}
//
//    fn select_task_rq(&self, _pid: u64) -> i32 { 0 }
//
//    fn selected_task_rq(&self, _sched: Schedulable) {}
//    
//    fn migrate_task_rq(&self, _pid: u64, _new_cpu: Schedulable) {}
//
//    fn balance(&self, _cpu: i32) -> Option<u64> { None }
//
//    fn reregister_prepare(&mut self) -> Option<TransferOut> {
//        None
//    }
//
//    fn reregister_init(&mut self, Option<TransferIn>) {}
//
//    fn register_queue(&self, RingBuffer<UserMessage>) {}
//
//    fn register_reverse_queue(&self, RingBuffer<RevMessage>) {}
//
//    fn enter_queue(&self, _entries: u32) {}
//
//    fn unregister_queue(&self) -> RingBuffer<UserMessage>;
//
//    fn unregister_rev_queue(&self) -> RingBuffer<RevMessage>;
//
//    fn parse_hint(&self, hint: UserMessage) {}
//}
