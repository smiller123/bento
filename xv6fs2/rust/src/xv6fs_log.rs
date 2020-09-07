#[cfg(not(feature = "user"))]
use crate::libc;
#[cfg(not(feature = "user"))]
use crate::std;
#[cfg(not(feature = "user"))]
use crate::println;
#[cfg(not(feature = "user"))]
use crate::bento_utils;

use alloc::sync::Arc;

use core::mem;
use core::sync::atomic::{AtomicBool, Ordering};

use bento_utils::Disk;
use bento_utils::BufferHead;

use std::sync::Mutex;
use std::sync::Condvar;

use datablock::DataBlock;

use crate::xv6fs_utils::*;
const LOGSIZE: usize = 128;

#[repr(C)]
#[derive(DataBlock)]
pub struct journal_header_t
{
    h_magic: u32,
    h_blocktype: u32,
    h_sequence: u32,
}

impl journal_header_t {
    pub fn new() -> journal_header_t {
        journal_header_t {
            h_magic: 0,
            h_blocktype: 0,
            h_sequence: 0,
        }
    }
}

#[repr(C)]
#[derive(DataBlock)]
pub struct journal_superblock_t {
    s_header: journal_header_t,

    s_blocksize: u32,
    s_maxlen: u32,
    s_first: u32,

    s_sequence: u32,
    s_start: u32,

    s_errno: u32,

    s_feature_compat: u32,
    s_feature_incompat: u32,
    s_feature_ro_compat: u32,
    s_uuid: [u8; 16],

    s_nr_users: u32,

    s_dynsuper: u32,

    s_max_transaction: u32,
    s_max_trans_data: u32,

    s_checksum_type: u8,
    s_padding2: [u8; 3],
    s_padding: [u32; 42],
    s_checksum: u32,

    s_users: [u8; 16*48],
    n: u32,
    block: [u32; LOGSIZE],
}

impl journal_superblock_t {
    pub fn new() -> journal_superblock_t {
        journal_superblock_t {
            s_header: journal_header_t::new(),
            s_blocksize: 0,
            s_maxlen: 0,
            s_first: 0,
            s_sequence: 0,
            s_start: 0,
            s_errno: 0,
            s_feature_compat: 0,
            s_feature_incompat: 0,
            s_feature_ro_compat: 0,
            s_uuid: [0 as u8; 16],
            s_nr_users: 0,
            s_dynsuper: 0,
            s_max_transaction: 0,
            s_max_trans_data: 0,
            s_checksum_type: 0,
            s_padding2: [0 as u8; 3],
            s_padding: [0 as u32; 42],
            s_checksum: 0,
            s_users: [0 as u8; 16*48],
            n: 0,
            block: [0 as u32; LOGSIZE],
        }
    }
}


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

pub static BLOCKER: AtomicBool = AtomicBool::new(false);

fn wait_cont(_l: &mut Log) -> bool {
    return BLOCKER.load(Ordering::SeqCst);
}

pub struct Journal {
    log_globl: Mutex<Log>,
    wait_q: Condvar,
    disk: Arc<Disk>,
}


impl Journal {
    #[allow(dead_code)]
    pub fn new_from_disk(disk: Arc<Disk>, _disk2: Arc<Disk>, start: u64, len: i32, bsize: i32) -> Option<Journal> {
        let new_journal = Self {
            log_globl: Mutex::new(Log {
                start: 0,
                size: 0,
                outstanding: 0,
                committing: 0,
                lh: logheader {
                    n: 0,
                    block: [0; LOGSIZE],
                },
            }),
            wait_q: Condvar::new(),
            disk: disk,
        };
        new_journal.initlog(start, len, bsize);
        return Some(new_journal);
    }

    pub fn destroy(&self) {}

    // xv6_sb is the xv6 filesystem superblock.
    #[allow(dead_code)]
    pub fn initlog(&self, start: u64, len: i32, _bsize: i32) -> Result<(), libc::c_int> {
        let mut log = &mut self.log_globl.lock().unwrap();
        log.start = start as u32;
        log.size = len as u32;
        println!("initlog: logstart {}, nlog: {}", log.start, log.size);
        self.recover_from_log(&mut log)
    }

    // Begin of a tx, must call begin_op in a filesystem syscall
    #[allow(dead_code)]
    pub fn begin_op<'log>(&'log self, _size: u32) -> Handle<'log> {
        let mut waiting = false;
        loop {
            let mut guard = self.log_globl.lock().unwrap();
            if waiting {
                // Wait on condvar
                guard = self.wait_q.wait_while(guard, wait_cont).unwrap();
            }
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
    
        Handle {
            xv6_log: self
        }
    }

    #[allow(dead_code)]
    pub fn force_commit(&self) {
        let mut guard = self.log_globl.lock().unwrap();
        let log: &mut Log = &mut *guard;
        log.committing = 1;
    
        let _com_out = self.commit(log);
        log.committing = 0;
        BLOCKER.store(true, Ordering::SeqCst);
        self.wait_q.notify_one();
    }

    // Only writes to buffer cache, does not persist; only install_trans will persist data.
    #[allow(dead_code)]
    pub fn log_write(&self, blk_no: u32) {
        let mut guard = self.log_globl.lock().unwrap();
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

    fn read_head(&self,log: &mut Log) -> Result<(), libc::c_int> {
        //let disk = self.disk.as_ref().unwrap();
        let bh = self.disk.bread(log.start as u64)?;
        let bh_slice = bh.data();
        let jsuper_slice = &bh_slice[0..mem::size_of::<journal_superblock_t>()];
        let mut jsuper = journal_superblock_t::new();
        jsuper.extract_from(&jsuper_slice).map_err(|_| libc::EIO)?;
        //let mut lh = logheader::new();
        //lh.extract_from(&lh_slice).map_err(|_| libc::EIO)?;
        log.lh.n = jsuper.n;
        for i in 0..(jsuper.n as usize) {
            jsuper.block
                .get(i)
                .and_then(|b| {
                    log.lh.block.get_mut(i).and_then(|r| {
                        *r = *b;
                        Some(())
                    })
                })
                .ok_or(libc::EIO)?;
        }
        
        Ok(())
    }
    
    // Transaction commits to log.
    fn write_head(&self,log: &mut Log) -> Result<(), libc::c_int> {
        //let disk = XV6FS.disk.as_ref().unwrap();
        let mut bh = self.disk.bread(log.start as u64)?;
        let bh_slice = bh.data_mut();
        let jsuper_slice = &mut bh_slice[0..mem::size_of::<journal_superblock_t>()];
        let mut jsuper = journal_superblock_t::new();
        jsuper.extract_from(&jsuper_slice).map_err(|_| libc::EIO)?;
        //let lh_slice = &mut bh_slice[0..mem::size_of::<logheader>()];
        //let mut lh = logheader::new();
        //lh.extract_from(lh_slice).map_err(|_| libc::EIO)?;
        jsuper.n = log.lh.n;
        for i in 0..(jsuper.n as usize) {
            log.lh
                .block
                .get(i)
                .and_then(|b| {
                    jsuper.block.get_mut(i).and_then(|r| {
                        *r = *b;
                        Some(())
                    })
                })
                .ok_or(libc::EIO)?;
        }
        jsuper.dump_into(jsuper_slice).map_err(|_| libc::EIO)?;
        bh.mark_buffer_dirty();
        
        Ok(())
    }
    
    pub fn install_trans(&self,log: &mut Log) -> Result<(), libc::c_int> {
        //let disk = XV6FS.disk.as_ref().unwrap();
        for tail in 0..(log.lh.n as usize) {
            if let Some (dst_blk_id) = log.lh.block.get(tail) {
                let src_blk_no: u64 = log.start as u64 + tail as u64 + 1;
                let src_bh = self.disk.bread(src_blk_no)?;
                let mut dst_bh = self.disk.bread(*dst_blk_id as u64)?;
                let src_slice = src_bh.data();
                let dst_slice = dst_bh.data_mut();
                dst_slice.copy_from_slice(src_slice);
                dst_bh.mark_buffer_dirty();
                dst_bh.sync_dirty_buffer();
            };
        }
        
        Ok(())
    }
    
    pub fn recover_from_log(&self, log: &mut Log) -> Result<(), libc::c_int> {
        self.read_head(log)?;
        self.install_trans(log)?;
        log.lh.n = 0;
        self.write_head(log)
    }
     
    fn write_log(&self, log: &mut Log) -> Result<(), libc::c_int> {
        for tail in 0..(log.lh.n as usize) {
            if let Some (src_blk_no) = log.lh.block.get(tail) {
                //let disk = XV6FS.disk.as_ref().unwrap();
                let dst_blk_no: u64 = log.start as u64 + tail as u64 + 1;
                let src_bh = self.disk.bread(*src_blk_no as u64)?;
                let mut dst_bh = self.disk.bread(dst_blk_no)?;
                let src_slice = src_bh.data();
                let dst_slice = dst_bh.data_mut();
                dst_slice.copy_from_slice(src_slice);
                dst_bh.mark_buffer_dirty();
                dst_bh.sync_dirty_buffer();
            };
        }
        Ok(())
    }
     
    // Commits in-log transaction, persists data to disk.
    fn commit(&self, log: &mut Log) -> Result<(), libc::c_int> {
        if log.lh.n > 0 {
            self.write_log(log)?;
            self.write_head(log)?;
            self.install_trans(log)?;
            log.lh.n = 0;
            let res = self.write_head(log);
            return res;
        } else {
            return Ok(());
        }
    }    
}

// Implements 'end_op' in original xv6, but does not need to be explicitly called.
pub struct Handle<'log> {
    xv6_log: &'log Journal,
}

impl Handle<'_> {
    pub fn get_write_access(&self, _bh: &BufferHead) -> i32 {
        return 0;
    }

    pub fn get_create_access(&self, _bh: &BufferHead) -> i32 {
        return 0;
    }

    pub fn journal_write(&self, bh: &mut BufferHead) -> i32 {
        bh.mark_buffer_dirty();
        self.xv6_log.log_write(bh.blk_no as u32);
        0
    }
}

impl Drop for Handle<'_> {
    fn drop(&mut self) {
        let mut do_commit = 0;
        {
            let mut guard = self.xv6_log.log_globl.lock().unwrap();
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
                self.xv6_log.wait_q.notify_one();
            }

            if do_commit != 0 {
                let _com_out = self.xv6_log.commit(log);
                log.committing = 0;
                BLOCKER.store(true, Ordering::SeqCst);
                self.xv6_log.wait_q.notify_one();
            }
        }
    }
}
