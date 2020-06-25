use core::cmp::min;

use kernel::errno::*;
use kernel::ffi::*;
use kernel::kobj::*;
use kernel::raw::*;

use crate::std::os::unix::io::*;
use crate::std::io;
use crate::std::os::unix::fs::*;

use bindings::*;

pub const FMODE_READ: u32 = 0x1;
pub const FMODE_WRITE: u32 = 0x2;
pub const FMODE_EXCL: u32 = 0x80;

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
pub fn sb_bread_rust(sb: &RsSuperBlock, blockno: u64) -> Option<BufferHead> {
    let bh;
    unsafe {
        bh = sb_bread(sb.get_raw() as *const c_void, blockno);
    }
    if bh.is_null() {
        return None;
    } else {
        unsafe {
            return Some(BufferHead::from_raw(bh as *const c_void));
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

pub struct BlockDevice {
    bdev: RsBlockDevice,
    bsize: u32,
}

impl BlockDevice {
    pub fn new(dev_name: &str, bsize: u32) -> Self {
        Self {
            bdev: RsBlockDevice::new(dev_name),
            bsize: bsize,
        }
    }

    pub fn sync_all(&self) -> Result<(), i32> {
        let mut error_sector = 0;
        blkdev_issue_flush_rust(&self.bdev, GFP_KERNEL as usize, &mut error_sector);
        match error_sector {
            0 => Ok(()),
            _ => Err(error_sector as i32),
        }
    }

    pub fn sync_data(&self) -> Result<(), i32> {
        let mut error_sector = 0;
        blkdev_issue_flush_rust(&self.bdev, GFP_KERNEL as usize, &mut error_sector);
        match error_sector {
            0 => Ok(()),
            _ => Err(error_sector as i32),
        }
    }

    pub fn sync_block(&self, sector: u64) -> Result<(), Error> {
        if let Some(mut bh) = self.bdev.bread(sector, self.bsize) {
            bh.sync_dirty_buffer();
        }
        Ok(())
    }

    pub fn bread(&self, blockno: u64) -> Result<BufferHead, Error> {
        self.bdev.bread(blockno, self.bsize).ok_or(Error::EIO)
    }
}

impl AsRawFd for BlockDevice {
    fn as_raw_fd(&self) -> RawFd {
        self.bdev.bd_dev() as RawFd
    }
}

pub struct Disk {
    bdev: BlockDevice,
}

impl Disk {
    pub fn new(dev_name: &str, bsize: u64) -> Self {
        Disk {
            bdev: BlockDevice::new(dev_name, bsize as u32),
        }
    }

    pub fn sync_all(&self) -> Result<(), i32> {
        self.bdev.sync_all()
    }

    pub fn sync_data(&self) -> Result<(), i32> {
        self.bdev.sync_data()
    }

    pub fn sync_block(&self, sector: u64) -> Result<(), Error> {
        self.bdev.sync_block(sector)
    }

    pub fn bread(&self, blockno: u64) -> Result<BufferHead, Error> {
        self.bdev.bread(blockno)
    }
}

impl AsRawFd for Disk {
    fn as_raw_fd(&self) -> RawFd {
        self.bdev.as_raw_fd()
    }
}

pub struct DiskFile {
    bdev: BlockDevice,
    bsize: u64,
}

impl DiskFile {
    pub fn new(dev_name: &str, bsize: u64) -> Self {
        Self {
            bdev: BlockDevice::new(dev_name, bsize as u32),
            bsize: bsize,
        }
    }

    pub fn sync_all(&self) -> Result<(), i32> {
        self.bdev.sync_all()
    }

    pub fn sync_data(&self) -> Result<(), i32> {
        self.bdev.sync_data()
    }

    pub fn sync_block(&self, sector: u64) {
        if let Ok(mut bh) = self.bdev.bread(sector) {
            bh.sync_dirty_buffer();
        }
    }
}

impl FileExt for DiskFile {
    fn read_at(&self, data: &mut [u8], offset: u64) -> io::Result<usize> {
        let mut read = 0;
        while read < data.len() {
            let curr_off = offset as usize + read;
            let sector = curr_off / self.bsize as usize;
            let sector_off = curr_off % self.bsize as usize;
            let read_size = min(self.bsize as usize - sector_off, data.len() - read);

            let bh = self.bdev.bread(sector as u64)
                .map_err(|err| { io::Error::from_raw_os_error(err) })?;
            let b_slice = bh.data();
            let read_region = &b_slice[curr_off..curr_off+read_size];
            let data_region = &mut data[read..read+read_size];
            data_region.copy_from_slice(read_region);
            read += read_size;
        }
        Ok(read)
    }

    fn write_at(&self, data: &[u8], offset: u64) -> io::Result<usize> {
        let mut written = 0;
        while written < data.len() {
            let curr_off = offset as usize + written;
            let sector = curr_off / self.bsize as usize;
            let sector_off = curr_off % self.bsize as usize;
            let write_size = min(self.bsize as usize - sector_off, data.len() - written);

            let mut bh = self.bdev.bread(sector as u64)
                .map_err(|err| { io::Error::from_raw_os_error(err) })?;
            let b_slice = bh.data_mut();
            let write_region = &mut b_slice[curr_off..curr_off+write_size];
            let data_region = &data[written..written+write_size];
            write_region.copy_from_slice(data_region);
            written += write_size;
        }
        Ok(written)
        //let mut bh = self.bdev.bread(sector)
        //    .map_err(|err| { io::Error::from_raw_os_error(err) })?;
        //let b_slice = bh.data_mut();
        //let write_region = &mut b_slice[offset..offset+data.len()];
        //write_region.copy_from_slice(data);
        //Ok(())
    }
}

impl AsRawFd for DiskFile {
    fn as_raw_fd(&self) -> RawFd {
        self.bdev.as_raw_fd()
    }
}
