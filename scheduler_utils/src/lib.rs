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


pub trait BentoScheduler<'a, 'b, TransferIn: Send, TransferOut: Send, UserMessage: Send + Copy + Serialize + Deserialize<'a>,
    RevMessage: Send + Copy + Serialize + Deserialize<'b>> {
    fn get_policy(&self) -> i32;
    
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

    fn register_queue(&self, _pid: u64, RingBuffer<UserMessage>) -> i32;

    fn register_reverse_queue(&self, _pid: u64, RingBuffer<RevMessage>) -> i32;

    fn enter_queue(&self, id: i32, _entries: u32) {}

    fn unregister_queue(&self, id: i32) -> RingBuffer<UserMessage>;

    fn unregister_rev_queue(&self, id: i32) -> RingBuffer<RevMessage>;

    fn parse_hint(&self, hint: UserMessage) {}
}

