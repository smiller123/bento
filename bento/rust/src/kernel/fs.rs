use kernel::ffi::*;
use kernel::kobj::*;
//use kernel::mem as kmem;
use kernel::raw::*;
//use kernel::semaphore::*;

//use core::alloc::*;
//use core::mem;

//use core::ops::{Deref, DerefMut};

/// Read a block from disk.
///
/// Calls the kernel `sb_bread` function. If that returns a NULL pointer, this function will
/// return `None`. Otherwise, this function will return `Some`.
///
/// TODO: Make this a method on the RsSuperBlock.
///
/// Arguments:
/// * `sb: &RsSuperBlock` - The kernel-provided superblock of the device
/// * `blockno: u64` - The block number to be read.
pub fn sb_bread_rust(sb: &RsSuperBlock, blockno: u64) -> Option<RsBufferHead> {
    let bh;
    unsafe {
        bh = sb_bread(sb.get_raw() as *const c_void, blockno);
    }
    if bh.is_null() {
        return None;
    } else {
        unsafe {
            return Some(RsBufferHead::from_raw(bh as *const c_void));
        }
    }
}

/// Flush a block device.
///
/// This function calls the kernel `blkdev_issue_flush` function.
///
/// If there's an error, it will be written to `error_sector`.
///
/// Arguments:
/// * `bdev: &RsBlockDevice` - The block device to flush.
/// * `gfp_mask: usize` - Memory allocation flags.
/// * `error_section: &mut u64` - Holder for error location.
pub fn blkdev_issue_flush_rust(
    bdev: &RsBlockDevice,
    gfp_mask: usize,
    error_sector: &mut u64,
) -> isize {
    unsafe {
        return blkdev_issue_flush(bdev.get_raw() as *const c_void, gfp_mask, error_sector);
    }
}

///// Currently broken. Do not use.
//struct SimpleDisk {}
//
//impl SimpleDisk {
//    pub const fn new() -> Self {
//        SimpleDisk {}
//    }
//
//    pub fn create(&mut self, _sectors: usize) -> Result<(), LayoutErr> {
//        return Ok(());
//    }
//
//    pub fn write_block(&self, sb: &RsSuperBlock, sector: usize) -> Result<RsBufferHead, i32> {
//        sb_bread_rust(sb, sector as u64).ok_or(-1)
//    }
//
//    pub fn read_block(&self, sb: &RsSuperBlock, sector: usize) -> Result<RsBufferHead, i32> {
//        sb_bread_rust(sb, sector as u64).ok_or(-1)
//    }
//}
//
///// Currently broken. Do not use.
//struct Disk {
//    sector_map: kmem::MemContainer<Semaphore<Option<RsBufferHead>>>,
//    sectors: usize,
//}
//
//struct BHWriteGuard<'a> {
//    write_guard: SemaphoreWriteGuard<'a, Option<RsBufferHead>>,
//}
//
//struct BHReadGuard {
//    read_guard: SemaphoreReadGuard<Option<RsBufferHead>>,
//}
//
//impl Disk {
//    //    pub const fn new() -> Self {
//    //        Disk {
//    //            sector_map: None,
//    //            sectors: 0,
//    //        }
//    //    }
//
//    pub fn create(&mut self, sectors: usize) -> Result<(), i32> {
//        //        printk!("starting create with %ld sectors\n", sectors);
//        //        let layout = Layout::new::<Semaphore<Option<RsBufferHead>>>();
//        //        printk!("create 1\n");
//        //        layout.repeat_packed(sectors)?;
//        //        printk!("create 2\n");
//        //        let disk_blocks = unsafe { KernelAllocator.alloc(layout) };
//        //        printk!("create 3\n");
//        //        let sector_cont = kmem::MemContainer::new_from_raw(disk_blocks as *mut Semaphore<Option<RsBufferHead>>,
//        //            sectors * mem::size_of::<Semaphore<Option<RsBufferHead>>())
//        //unsafe { from_raw_parts_mut(disk_blocks as *mut Semaphore<Option<RsBufferHead>>, sectors) };
//        self.sector_map =
//            kmem::MemContainer::alloc(sectors * mem::size_of::<Semaphore<Option<RsBufferHead>>>())
//                .ok_or(-1)?;
//        //printk!("create 4\n");
//        for sem in self.sector_map.to_slice_mut().iter_mut() {
//            //printk!("create 4.1\n");
//            *sem = Semaphore::new(None);
//            //printk!("create 4.2\n");
//            sem.init();
//            //printk!("create 4.3\n");
//            //            sem = &mut this_sem;
//            //printk!("create 4.4\n");
//            //            mem::forget(this_sem);
//            //printk!("create 4.5\n");
//        }
//        //printk!("create 5\n");
//        //        mem::forget(sector_arr);
//        //printk!("create 6\n");
//        //        mem::forget(disk_blocks);
//        //printk!("create 7\n");
//        self.sectors = sectors;
//        //printk!("create 8\n");
//        //        self.sector_map = Some(disk_blocks as *mut Semaphore<Option<RsBufferHead>>);
//        //        self.sector_map = Some(*from_raw_parts_mut(disk_blocks as *mut Semaphore<usize>, sectors) as *mut Semaphore<usize>);
//        //printk!("finishing create\n");
//        return Ok(());
//    }
//
//    pub fn write_block(&self, sb: &RsSuperBlock, sector: usize) -> Result<BHWriteGuard, i32> {
//        //printk!("write block: sector %ld\n", sector);
//        let sector_semaphore = self.sector_map.to_slice().get(sector).ok_or(-1)?;
//        //printk!("write block 4\n");
//        let mut bh_opt = sector_semaphore.write();
//        //printk!("write block 5\n");
//        if (*bh_opt).is_none() {
//            //printk!("write block 6\n");
//            *bh_opt = sb_bread_rust(sb, sector as u64);
//            //printk!("write block 6.5\n");
//        }
//        //printk!("write block 7\n");
//        return Ok(BHWriteGuard {
//            write_guard: bh_opt,
//        });
//    }
//
//    pub fn read_block(&self, sb: &RsSuperBlock, sector: usize) -> Result<BHReadGuard, i32> {
//        //printk!("read block: sector %ld\n", sector);
//        let mut alloc = false;
//        //printk!("read block 2\n");
//        loop {
//            let sector_semaphore = self.sector_map.to_slice().get(sector).ok_or(-1)?;
//            //printk!("read block 6\n");
//            if alloc {
//                //printk!("read block 6.1\n");
//                let mut bh_opt = sector_semaphore.write();
//                //printk!("read block 6.2\n");
//                *bh_opt = sb_bread_rust(sb, sector as u64);
//                //printk!("read block 6.3\n");
//            }
//            //printk!("read block 7\n");
//            let bh_opt = sector_semaphore.read();
//            if (*bh_opt).is_none() {
//                //printk!("read block 7.1\n");
//                alloc = true;
//                //printk!("read block 7.2\n");
//                continue;
//            }
//            //printk!("read block 8\n");
//            return Ok(BHReadGuard { read_guard: bh_opt });
//        }
//    }
//}
//
//impl<'a> Drop for BHWriteGuard<'a> {
//    fn drop(&mut self) {
//        if let Some(bh) = &mut *self.write_guard {
//            bh.mark_buffer_dirty();
//        }
//    }
//}
//
////impl Drop for Disk {
////    fn drop(&mut self) {
////        if let Some(ptr) = self.sector_map {
////            let sector_arr = unsafe { from_raw_parts_mut(ptr, self.sectors) };
////            for sem in sector_arr.iter_mut() {
////                mem::drop(*sem);
////            }
////            let layout = Layout::new::<usize>();
////            unsafe {
////                KernelAllocator.dealloc(ptr as *mut u8, layout)
////            }
////        }
////    }
////}
//
//impl Deref for BHReadGuard {
//    type Target = Option<RsBufferHead>;
//
//    fn deref(&self) -> &Option<RsBufferHead> {
//        self.read_guard.deref()
//    }
//}
//
//impl<'rwlock> Deref for BHWriteGuard<'rwlock> {
//    type Target = Option<RsBufferHead>;
//
//    fn deref(&self) -> &Option<RsBufferHead> {
//        self.write_guard.deref()
//    }
//}
//
//impl<'rwlock> DerefMut for BHWriteGuard<'rwlock> {
//    fn deref_mut(&mut self) -> &mut Option<RsBufferHead> {
//        self.write_guard.deref_mut()
//    }
//}
//
//unsafe impl Send for Disk {}
//unsafe impl Sync for Disk {}
