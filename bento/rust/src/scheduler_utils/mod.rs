use alloc::boxed::Box;
use libc::ENOSYS;

use time::Timespec;

use bindings as c;
use bindings::{register_ghost_agent,unregister_ghost_agent,reregister_ghost_agent};
use kernel::raw;

pub const BENTO_KERNEL_VERSION: u32 = 1;
pub const BENTO_KERNEL_MINOR_VERSION: u32 = 0;

pub fn parse_message<TransferIn: Send, TransferOut: Send, T: BentoScheduler<TransferIn, TransferOut>>(
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
        match type_ as u32 {
            c::MSG_PNT => {
                let payload_data = payload as *const c::ghost_msg_payload_pnt;
                agent.pick_next_task((*payload_data).cpu, &mut *retval);
            }
            c::MSG_TASK_DEAD => {
                let payload_data = payload as *const c::ghost_msg_payload_task_dead;
                agent.task_dead((*payload_data).pid);
            }
            c::MSG_TASK_BLOCKED => {
                let payload_data = payload as *const c::ghost_msg_payload_task_blocked;
                agent.task_blocked((*payload_data).pid, (*payload_data).runtime,
                    (*payload_data).cpu_seqnum,
                    (*payload_data).cpu, (*payload_data).from_switchto);
            }
            c::MSG_TASK_WAKEUP => {
                let payload_data = payload as *const c::ghost_msg_payload_task_wakeup;
                agent.task_wakeup((*payload_data).pid, (*payload_data).agent_data,
                    (*payload_data).deferrable, (*payload_data).last_ran_cpu,
                    (*payload_data).wake_up_cpu, (*payload_data).waker_cpu);
            }
            c::MSG_TASK_NEW => {
                let payload_data = payload as *const c::ghost_msg_payload_task_new;
                agent.task_new((*payload_data).pid, (*payload_data).runtime, (*payload_data).runnable);
            }
            c::MSG_TASK_PREEMPT => {
                let payload_data = payload as *const c::ghost_msg_payload_task_preempt;
                agent.task_preempt((*payload_data).pid, (*payload_data).runtime,
                    (*payload_data).cpu_seqnum, (*payload_data).cpu,
                    (*payload_data).from_switchto, (*payload_data).was_latched);
            }
            c::MSG_TASK_YIELD => {
                let payload_data = payload as *const c::ghost_msg_payload_task_yield;
                agent.task_yield((*payload_data).pid, (*payload_data).runtime,
                    (*payload_data).cpu_seqnum, (*payload_data).cpu,
                    (*payload_data).from_switchto);
            }
            c::MSG_TASK_DEPARTED => {
                let payload_data = payload as *const c::ghost_msg_payload_task_departed;
                agent.task_departed((*payload_data).pid, (*payload_data).cpu_seqnum,
                    (*payload_data).cpu, (*payload_data).from_switchto,
                    (*payload_data).was_current);
            }
            c::MSG_TASK_SWITCHTO => {
                let payload_data = payload as *const c::ghost_msg_payload_task_switchto;
                agent.task_switchto((*payload_data).pid, (*payload_data).runtime,
                    (*payload_data).cpu_seqnum, (*payload_data).cpu);
            }
            c::MSG_TASK_AFFINITY_CHANGED => {
                let payload_data = payload as *const c::ghost_msg_payload_task_affinity_changed;
                agent.task_affinity_changed((*payload_data).pid);
            }
            c::MSG_TASK_LATCHED => {
                let payload_data = payload as *const c::ghost_msg_payload_task_latched;
                agent.task_latched((*payload_data).pid, (*payload_data).commit_time,
                    (*payload_data).cpu_seqnum, (*payload_data).cpu,
                    (*payload_data).latched_preempt);
            }
            c::MSG_CPU_TICK => {
                let payload_data = payload as *const c::ghost_msg_payload_cpu_tick;
                agent.cpu_tick((*payload_data).cpu);
            }
            c::MSG_CPU_NOT_IDLE => {
                let payload_data = payload as *const c::ghost_msg_payload_cpu_not_idle;
                agent.cpu_not_idle((*payload_data).cpu, (*payload_data).next_pid);
            }
            c::MSG_TASK_SELECT_RQ => {
                let payload_data = payload as *const c::ghost_msg_payload_select_task_rq;
                agent.select_task_rq((*payload_data).pid, &mut *retval);
            }
            c::MSG_TASK_MIGRATE_RQ => {
                let payload_data = payload as *const c::ghost_msg_payload_migrate_task_rq;
                agent.migrate_task_rq((*payload_data).pid, (*payload_data).new_cpu);
            }
            c::MSG_BALANCE => {
                let payload_data = payload as *const c::ghost_msg_payload_balance;
                agent.balance((*payload_data).cpu);
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
                    unsafe { Some(*Box::from_raw((*payload_data).data as *mut TransferIn)) }
                };
                agent.reregister_init(data);
            }
            _ => {
                println!("Unsupported message type");
            }
        }
    }
}

/// BentoScheduler trait
///
/// This trait is derived from the Filesystem trait from the fuse Rust crate.
///
/// This trait must be implemented to provide a Bento scheduler.
pub trait BentoScheduler<TransferIn: Send, TransferOut: Send> {
    fn get_policy(&self) -> i32;
    /// Register the filesystem with Bento.
    ///
    /// This should be called when the filesystem module is inserted and before
    /// a filesystem is mounted.
    fn register(&self) -> i32
    where
        Self: core::marker::Sized,
    {
        return unsafe {
            register_ghost_agent(
                self as *const Self as *const raw::c_void,
                self.get_policy(),
                parse_message::<TransferIn, TransferOut, Self> as *const raw::c_void
            )
        };
    }

    fn reregister(&self) -> i32
    where
        Self: core::marker::Sized,
    {
        return unsafe {
            reregister_ghost_agent(
                self as *const Self as *const raw::c_void,
                self.get_policy(),
                parse_message::<TransferIn, TransferOut, Self> as *const raw::c_void
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
        ret: &mut i32
    ) {
        *ret = 0;
    }

    fn task_dead(&self, _pid: u64) {}

    fn task_blocked(
        &self,
        _pid: u64,
        _runtime: u64,
        _cpu_seqnum: u64,
        _cpu: i32,
        _from_switchto: i8
    ) {}

    fn task_wakeup(
        &self,
        _pid: u64,
        _agent_data: u64,
        _deferrable: i8,
        _last_run_cpu: i32,
        _wake_up_cpu: i32,
        _waker_cpu: i32
    ) {}

    fn task_new(
        &self,
        _pid: u64,
        _runtime: u64,
        _runnable: u16,
    ) {}

    fn task_preempt(
        &self,
        _pid: u64,
        _runtime: u64,
        _cpu_seqnum: u64,
        _cpu: i32,
        _from_switchto: i8,
        _was_latched: i8
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

    fn cpu_tick(&self, _cpu: i32) {}

    fn cpu_not_idle(&self, _cpu: i32, _next_pid: u64) {}

    fn select_task_rq(&self, _pid: u64, _retval: &mut i32) {}
    
    fn migrate_task_rq(&self, _pid: u64, _new_cpu: i32) {}

    fn balance(&self, _cpu: i32) {}

    //fn bento_update_prepare(&mut self) -> Option<TransferOut> {
    fn reregister_prepare(&mut self) -> Option<TransferOut> {
        None
    }

    fn reregister_init(&mut self, Option<TransferIn>) {}
}
