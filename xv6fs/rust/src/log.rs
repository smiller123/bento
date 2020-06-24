use core::mem;
use core::sync::atomic::{AtomicBool, Ordering};

use bento::kernel;
use kernel::errno::*;
use kernel::semaphore::*;
use kernel::wait_queue::*;

use bento::println;
use bento::DataBlock;

use crate::xv6fs_utils::*;

use crate::xv6fs_fs::DISK;

#[repr(C)]
#[derive(DataBlock)]
pub struct logheader {
    n: u32,
    block: [u32; LOGSIZE],
}

impl logheader {
    pub const fn new() -> Self {
        Self {
            n: 0,
            block: [0; LOGSIZE],
        }
    }
}

#[derive(DataBlock)]
pub struct Log {
    start: u32,
    size: u32,
    outstanding: u32,
    committing: u32,
    lh: logheader,
}

// Wraps semaphore in C kernel
pub static LOG_GLOBL: Semaphore<Log> = Semaphore::new(Log {
    start: 0,
    size: 0,
    outstanding: 0,
    committing: 0,
    lh: logheader {
        n: 0,
        block: [0; LOGSIZE],
    },
});

// Wraps wait_queue in C kernel
pub static WAIT_Q: WaitQueue = WaitQueue::new();

pub static BLOCKER: AtomicBool = AtomicBool::new(false);

// xv6_sb is the xv6 filesystem superblock.
pub fn initlog(xv6_sb: &mut Xv6fsSB) -> Result<(), Error> {
    LOG_GLOBL.init();
    WAIT_Q.init();
    let mut log = &mut LOG_GLOBL.write();
    log.start = xv6_sb.logstart;
    log.size = xv6_sb.nlog;
    println!("initlog: logstart {}, nlog: {}", log.start, log.size);
    recover_from_log(&mut log)
}

fn read_head(log: &mut Log) -> Result<(), Error> {
    let disk_guard = DISK.read();
    if let Some(disk) = &*disk_guard {
        let bh = disk.bread(log.start as u64)?;
        let bh_slice = bh.data();
        let lh_slice = &bh_slice[0..mem::size_of::<logheader>()];
        let mut lh = logheader::new();
        lh.extract_from(&lh_slice).map_err(|_| Error::EIO)?;
        log.lh.n = lh.n;
        for i in 0..(lh.n as usize) {
            lh.block
                .get(i)
                .and_then(|b| {
                    log.lh.block.get_mut(i).and_then(|r| {
                        *r = *b;
                        Some(())
                    })
                })
                .ok_or(Error::EIO)?;
        }
    }
    Ok(())
}

// Transaction commits to log.
fn write_head(log: &mut Log) -> Result<(), Error> {
    let disk_guard = DISK.read();
    if let Some(disk) = &*disk_guard {
        let mut bh = disk.bread(log.start as u64)?;
        let bh_slice = bh.data_mut();
        let lh_slice = &mut bh_slice[0..mem::size_of::<logheader>()];
        let mut lh = logheader::new();
        lh.extract_from(lh_slice).map_err(|_| Error::EIO)?;
        lh.n = log.lh.n;
        for i in 0..(lh.n as usize) {
            log.lh
                .block
                .get(i)
                .and_then(|b| {
                    lh.block.get_mut(i).and_then(|r| {
                        *r = *b;
                        Some(())
                    })
                })
                .ok_or(Error::EIO)?;
        }
        lh.dump_into(lh_slice).map_err(|_| Error::EIO)?;
        bh.mark_buffer_dirty();
    }
    Ok(())
}

pub fn install_trans(log: &mut Log) -> Result<(), Error> {
    let disk_guard = DISK.read();
    if let Some(disk) = &*disk_guard {
        for tail in 0..(log.lh.n as usize) {
            log.lh.block.get(tail).map_or(Ok(()), |dst_blk_id| {
                let src_blk_no: u64 = log.start as u64 + tail as u64 + 1;
                let src_bh = disk.bread(src_blk_no)?;
                let mut dst_bh = disk.bread(*dst_blk_id as u64)?;
                let src_slice = src_bh.data();
                let dst_slice = dst_bh.data_mut();
                dst_slice.copy_from_slice(src_slice);
                dst_bh.mark_buffer_dirty();
                dst_bh.sync_dirty_buffer();
                //disk.sync_block(*dst_blk_id as u64)?;
                Ok(())
            })?;
        }
    }
    Ok(())
}

pub fn recover_from_log(log: &mut Log) -> Result<(), Error> {
    read_head(log)?;
    install_trans(log)?;
    log.lh.n = 0;
    write_head(log)
}

// Implements 'end_op' in original xv6, but does not need to be explicitly called.
pub struct LogOpGuard {
}

impl Drop for LogOpGuard {
    fn drop(&mut self) {
        let mut do_commit = 0;
        {
            let mut guard = LOG_GLOBL.write();
            let log: &mut Log = &mut *guard;
            log.outstanding -= 1;
            if log.committing != 0 {
                println!("PANIC: log_committing");
                loop {}
            }

            if log.outstanding == 0 {
                do_commit = 1;
                log.committing = 1;
            } else {
                BLOCKER.store(true, Ordering::SeqCst);
                WAIT_Q.wake_up();
            }

            if do_commit != 0 {
                let _com_out = commit(log);
                log.committing = 0;
                BLOCKER.store(true, Ordering::SeqCst);
                WAIT_Q.wake_up();
            }
        }
    }
}

extern "C" fn wait_cont() -> bool {
    return BLOCKER.load(Ordering::SeqCst);
}

// Begin of a tx, must call begin_op in a filesystem syscall
pub fn begin_op() -> LogOpGuard {
    let mut waiting = false;
    loop {
        if waiting {
            // Wait on condvar
            WAIT_Q.wait_event(wait_cont);
        }
        let mut guard = LOG_GLOBL.write();
        let log: &mut Log = &mut *guard;
        if log.lh.n as usize + (log.outstanding as usize + 1) * MAXOPBLOCKS > LOGSIZE {
            BLOCKER.store(false, Ordering::SeqCst);
            waiting = true;
            continue;
        } else {
            log.outstanding += 1;
            break;
        }
    }

    LogOpGuard { }
}

fn write_log(log: &mut Log) -> Result<(), Error> {
    for tail in 0..(log.lh.n as usize) {
        log.lh.block.get(tail).map_or(Ok(()), |src_blk_no| {
            let disk_guard = DISK.read();
            let disk = disk_guard.as_ref().unwrap();
            let dst_blk_no: u64 = log.start as u64 + tail as u64 + 1;
            let src_bh = disk.bread(*src_blk_no as u64)?;
            let mut dst_bh = disk.bread(dst_blk_no)?;
            let src_slice = src_bh.data();
            let dst_slice = dst_bh.data_mut();
            dst_slice.copy_from_slice(src_slice);
            dst_bh.mark_buffer_dirty();
            dst_bh.sync_dirty_buffer();
            Ok(())
        })?;
    }
    Ok(())
}

pub fn force_commit() {
    let mut guard = LOG_GLOBL.write();
    let log: &mut Log = &mut *guard;
    log.committing = 1;

    let _com_out = commit(log);
    log.committing = 0;
    BLOCKER.store(true, Ordering::SeqCst);
    WAIT_Q.wake_up();
}

// Commits in-log transaction, persists data to disk.
fn commit(log: &mut Log) -> Result<(), Error> {
    if log.lh.n > 0 {
        write_log(log)?;
        write_head(log)?;
        install_trans(log)?;
        log.lh.n = 0;
        let res = write_head(log);
        return res;
    } else {
        return Ok(());
    }
}

// Only writes to buffer cache, does not persist; only install_trans will persist data.
pub fn log_write(blk_no: u32) {
    let mut guard = LOG_GLOBL.write();
    let log: &mut Log = &mut *guard;
    if log.lh.n as usize >= LOGSIZE || log.lh.n >= log.size {
        // TODO: panic
        println!("log_write: panic: too big transaction {}", log.lh.n);
        loop {}
    }

    if log.outstanding < 1 {
        // TODO: panic
        println!("log_write: panic: log_write outside of transaction");
        loop {}
    }

    let mut i: usize = 0;
    while i < log.lh.n as usize {
        let r = log.lh.block.get(i);
        let should_absorb = r.map(|bn| *bn == blk_no).unwrap_or(false);
        if should_absorb {
            break;
        }
        i += 1;
    }
    let b = log.lh.block.get_mut(i);
    b.map(|r| *r = blk_no);
    if i == log.lh.n as usize {
        log.lh.n += 1;
    }
}
