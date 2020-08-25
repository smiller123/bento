use std::alloc;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::os::unix::fs::FileExt;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::RawFd;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Weak;
use std::sync::RwLock;

use std::cmp::min;

unsafe fn alloc_aligned_vec(bsize: usize) -> Result<Vec<u8>, alloc::LayoutErr> {
    let layout = alloc::Layout::from_size_align(bsize as usize, bsize as usize)?;
    let ptr = alloc::alloc(layout);
    Ok(Vec::from_raw_parts(ptr, bsize as usize, bsize as usize))
}

struct ArrWrapper {
    arr: Vec<u8>,
    dirty: bool,
    blockno: u64,
    disk: Arc<File>,
    bsize: u64,
}

impl ArrWrapper {
    fn new(blockno: u64, disk: Arc<File>, bsize: u64) -> Result<ArrWrapper, libc::c_int> {
        // Allocate a vector aligned to `bsize`
        let mut block_arr = unsafe {
            alloc_aligned_vec(bsize as usize)
        }.map_err(|_| libc::EIO)?;
        disk.read_at(block_arr.as_mut_slice(), blockno * bsize).map_err(|_| libc::EIO)?;
        Ok(Self {
            arr: block_arr,
            dirty: false,
            blockno: blockno,
            disk: disk,
            bsize: bsize,
        })
    }

    fn mark_buffer_dirty(&mut self) {
        self.dirty = true;
    }

    fn sync_dirty_buffer(&mut self) {
        if self.dirty {
            let _ = self.disk.write_at(self.arr.as_slice(), self.blockno * self.bsize);
        }
        self.dirty = false;
    }

    fn data(&self) -> &[u8] {
        self.arr.as_slice()
    }

    fn data_mut(&mut self) -> &mut [u8] {
        self.arr.as_mut_slice()
    }
}

impl Drop for ArrWrapper {
    fn drop(&mut self) {
        if self.dirty {
            let _ = self.disk.write_at(self.arr.as_mut_slice(), self.blockno * self.bsize);
        }
    }
}

pub struct BufferHead {
    buffer: Arc<ArrWrapper>,
    pub blk_no: u64
}

/// Currently not thread-safe. Multiple threads can mutate the same block at the same time.
impl BufferHead {
    fn new(buffer: Arc<ArrWrapper>, blk_no: u64) -> Self {
        Self {
            buffer: buffer, 
            blk_no: blk_no,
        }
    }

    pub fn mark_buffer_dirty(&mut self) {
        unsafe {
            Arc::get_mut_unchecked(&mut self.buffer).mark_buffer_dirty();
        }
    }

    pub fn sync_dirty_buffer(&mut self) {
        unsafe {
            Arc::get_mut_unchecked(&mut self.buffer).sync_dirty_buffer();
        }
    }

    pub fn data(&self) -> &[u8] {
        self.buffer.data()
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        unsafe {
            Arc::get_mut_unchecked(&mut self.buffer).data_mut()
        }
    }

}

struct BufferCache {
    file: Arc<File>,
    cache: RwLock<HashMap<u64, Weak<ArrWrapper>>>,
    bsize: u64
}

impl BufferCache {
    fn new(name: &str, bsize: u64) -> Self {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_DIRECT)
            .open(name)
            .unwrap();
        Self {
            file: Arc::new(f),
            cache: RwLock::new(HashMap::new()),
            bsize: bsize,
        }
    }

    fn sync_all(&self) -> Result<(), libc::c_int> {
        let cache_read = self.cache.read().unwrap();
        for weak in cache_read.values() {
            if let Some(mut bh) = weak.upgrade() {
                unsafe {
                    Arc::get_mut_unchecked(&mut bh).sync_dirty_buffer();
                }
            }
        }
        self.file.sync_all().map_err(|_| libc::EIO)
    }

    fn sync_data(&self) -> Result<(), libc::c_int> {
        let cache_read = self.cache.read().unwrap();
        for weak in cache_read.values() {
            if let Some(mut bh) = weak.upgrade() {
                unsafe {
                    Arc::get_mut_unchecked(&mut bh).sync_dirty_buffer();
                }
            }
        }
        self.file.sync_data().map_err(|_| libc::EIO)
    }

    #[allow(dead_code)]
    fn sync_block(&self, blockno: u64) -> Result<(), libc::c_int> {
        let cache_read = self.cache.read().unwrap();
        if let Some(weak) = cache_read.get(&blockno) {
            if let Some(mut bh) = weak.upgrade() {
                unsafe {
                    Arc::get_mut_unchecked(&mut bh).sync_dirty_buffer();
                }
            }
        }
        self.file.sync_data().map_err(|_| libc::EIO)
    }

    fn bread(&self, blockno: u64) -> Result<BufferHead, libc::c_int> {
        let mut cache_lock = self.cache.write().unwrap();
        if let Some(weak) = cache_lock.get_mut(&blockno) {
            if let Some(buf_lock) = weak.upgrade() {
                return Ok(BufferHead::new(Arc::clone(&buf_lock), blockno));
            }
        }
        let bh_buf = ArrWrapper::new(blockno, Arc::clone(&self.file), self.bsize)?;
        let new_arc = Arc::new(bh_buf);
        cache_lock.insert(blockno, Arc::downgrade(&new_arc));
        return Ok(BufferHead::new(new_arc, blockno));
    }
}

pub struct Disk {
    cache: BufferCache,
}

impl Disk {
    pub fn new(name: &str, bsize: u64) -> Self {
        Self {
            cache: BufferCache::new(name, bsize),
        }
    }

    pub fn sync_all(&self) -> Result<(), libc::c_int> {
        self.cache.sync_all()
    }

    pub fn sync_data(&self) -> Result<(), libc::c_int> {
        self.cache.sync_data()
    }

    #[allow(dead_code)]
    fn sync_block(&self, blockno: u64) -> Result<(), libc::c_int> {
        self.cache.sync_block(blockno)
    }

    pub fn bread(&self, blockno: u64) -> Result<BufferHead, libc::c_int> {
        self.cache.bread(blockno)
    }
}

impl AsRawFd for Disk {
    fn as_raw_fd(&self) -> RawFd {
        self.cache.file.as_raw_fd()
    }
}

pub struct DiskFile {
    cache: BufferCache,
    bsize: u64,
}

impl DiskFile {
    pub fn new(name: &str, bsize: u64) -> Self {
        Self {
            cache: BufferCache::new(name, bsize),
            bsize: bsize,
        }
    }

    pub fn sync_all(&self) -> Result<(), libc::c_int> {
        self.cache.file.sync_all().map_err(|_| libc::EIO)
    }

    pub fn sync_data(&self) -> Result<(), libc::c_int> {
        self.cache.sync_data()
    }

    pub fn sync_block(&self, blockno: u64) -> Result<(), libc::c_int> {
        self.cache.sync_block(blockno)
    }
}

impl AsRawFd for DiskFile {
    fn as_raw_fd(&self) -> RawFd {
        self.cache.file.as_raw_fd()
    }
}

impl FileExt for DiskFile {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        let mut read = 0;
        while read < buf.len() {
            let curr_off = offset as usize + read;
            let block = curr_off / self.bsize as usize;
            let block_offset = curr_off % self.bsize as usize;
            let read_size = min(self.bsize as usize - block_offset, buf.len() - read);

            let bh = self.cache.bread(block as u64)
                .map_err(|err| { io::Error::from_raw_os_error(err) })?;
            let bh_data = bh.data();
            let buf_region = &mut buf[read..read + read_size];
            let bh_region = &bh_data[block_offset..block_offset+read_size];
            buf_region.copy_from_slice(bh_region);
            read += read_size;
        }
        return Ok(read);
    }

    fn write_at(&self, buf: &[u8], offset: u64) -> io::Result<usize> {
        let mut written = 0;
        while written < buf.len() {
            let curr_off = offset as usize + written;
            let block = curr_off / self.bsize as usize;
            let block_offset = curr_off % self.bsize as usize;
            let write_size = min(self.bsize as usize - block_offset, buf.len() - written);

            let mut bh = self.cache.bread(block as u64)
                .map_err(|err| { io::Error::from_raw_os_error(err) })?;
            let bh_data = bh.data_mut();
            let buf_region = &buf[written..written + write_size];
            let bh_region = &mut bh_data[block_offset..block_offset+write_size];
            bh_region.copy_from_slice(buf_region);
            written += write_size;
            bh.mark_buffer_dirty();
        }
        return Ok(written);
    }
}
