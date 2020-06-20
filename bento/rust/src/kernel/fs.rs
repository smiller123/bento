use kernel::errno::*;
use kernel::ffi::*;
use kernel::kobj::*;
use kernel::raw::*;

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

pub struct Disk {
    bdev: BlockDevice,
}

impl Disk {
    pub fn new(dev_name: &str, bsize: u32) -> Self {
        Disk {
            bdev: BlockDevice::new(dev_name, bsize),
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

pub struct DiskFile {
    bdev: BlockDevice,
}

impl DiskFile {
    pub fn new(dev_name: &str, bsize: u32) -> Self {
        Self {
            bdev: BlockDevice::new(dev_name, bsize),
        }
    }

    pub fn write_at(&self, sector: u64, data: &[u8], offset: usize) -> Result<(), Error> {
        if offset + data.len() > self.bdev.bsize as usize {
            return Err(Error::EOVERFLOW);
        }
        let bh = self.bdev.bread(sector)?;
        let mut b_data = bh.get_buffer_data();
        let b_slice = b_data.to_slice_mut();
        let write_region = &mut b_slice[offset..offset+data.len()];
        write_region.copy_from_slice(data);
        Ok(())
    }

    pub fn read_at(&self, sector: u64, data: &mut [u8], offset: usize) -> Result<(), Error> {
        if offset + data.len() > self.bdev.bsize as usize {
            return Err(Error::EOVERFLOW);
        }
        let bh = self.bdev.bread(sector)?;
        let b_data = bh.get_buffer_data();
        let b_slice = b_data.to_slice();
        let read_region = &b_slice[offset..offset+data.len()];
        data.copy_from_slice(read_region);
        Ok(())
    }

    pub fn sync_block(&self, sector: u64) {
        if let Ok(mut bh) = self.bdev.bread(sector) {
            bh.sync_dirty_buffer();
        }
    }
}
