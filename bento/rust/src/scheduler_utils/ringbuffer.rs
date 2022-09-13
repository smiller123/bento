/// Based on ringbuffer on Github https://github.com/NULLx76/ringbuffer

extern crate alloc;
// We need vecs so depend on alloc
use alloc::vec::Vec;
use core::iter::FromIterator;
use core::mem;
use core::mem::MaybeUninit;
use core::slice;
use kernel::raw;

pub struct BufferInner<T> {
    pub offset: u32,
    pub capacity: u32,
    pub writeptr: u32,
    pub readptr: u32,
    pub val: T,
}

pub struct RingBuffer<T> {
//    buf: Vec<MaybeUninit<T>>,
    pub inner: *mut BufferInner<T>
}

impl<T: Copy> BufferInner<T> {
    fn len(&self) -> u32 {
        self.writeptr - self.readptr
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    fn is_full(&self) -> bool {
        // Does this actually loop correctly?
        self.len() == self.capacity
    }

    fn dequeue(&mut self, ptr: *mut Self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            let index = self.readptr & (self.capacity - 1);
            let start = (ptr as u64 + self.offset as u64) as *mut T;
            println!("start {:?}", start);
            let slice_buf = unsafe {
                slice::from_raw_parts_mut(start, self.capacity as usize)
            };
            println!("index {}", index);
            let res = slice_buf[index as usize];
            self.readptr += 1;
            unsafe { Some(res) }
        }
    }
}

impl<T: Copy> RingBuffer<T> {
    fn len(&self) -> u32 {
        unsafe {
            (*self.inner).len()
        }
    }

    #[inline]
    fn is_empty(&self) -> bool {
        unsafe {
            (*self.inner).is_empty()
        }
    }

    #[inline]
    fn is_full(&self) -> bool {
        unsafe {
            (*self.inner).is_full()
        }
        // Does this actually loop correctly?
    }

    pub fn dequeue(&mut self) -> Option<T> {
        unsafe {
            (*self.inner).dequeue(self.inner)
        }
    }

    pub unsafe fn from_raw(ptr: *mut raw::c_void) -> Self {
        Self {
            inner: ptr as *mut BufferInner<T>
        }
    }
}
