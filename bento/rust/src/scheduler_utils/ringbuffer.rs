/// Based on ringbuffer on Github https://github.com/NULLx76/ringbuffer

extern crate alloc;
extern crate serde;
extern crate postcard;
// We need vecs so depend on alloc
use alloc::vec::Vec;
use core::fmt::Debug;
use core::iter::FromIterator;
use core::mem;
use core::mem::MaybeUninit;
use core::slice;
use kernel::raw;
use bindings as c; 

use serde::{Serialize, Deserialize};
use core::convert::TryInto;
//use postcard;


pub struct BufferInner<T: Send> {
    pub offset: u32,
    pub capacity: u32,
    pub writeptr: u32,
    pub readptr: u32,
    pub val: T,
}

pub struct RingBuffer<T: Send> {
//    buf: Vec<MaybeUninit<T>>,
    pub inner: *mut BufferInner<T>,
    pub policy: i32
}

unsafe impl<T: Send> Send for RingBuffer<T> {}

impl<'a, T: Send + Copy + Serialize + Deserialize<'a>> BufferInner<T> {
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

    fn enqueue(&mut self, ptr: *mut Self, val: T) {
        //if self.is_empty() {
        //    None
        //} else {
        let index = self.writeptr & (self.capacity - 1);
        let start = (ptr as u64 + self.offset as u64) as *mut T;
        println!("start {:?}", start);
        let slice_buf = unsafe {
            slice::from_raw_parts_mut(start, self.capacity as usize)
        };
        println!("index {}", index);
        //let res = slice_buf[index as usize];
        slice_buf[index as usize] = val;
        self.writeptr += 1;
        //unsafe { Some(res) }
        //}
    }
}

impl<'a, T: Send + Copy + Serialize + Deserialize<'a>> RingBuffer<T> {
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
            let ret = (*self.inner).dequeue(self.inner);

            // make this size more correct.
            #[cfg(feature = "record")]
            {
                let mut buf = [0u8; 128];
                postcard::to_slice(&ret, &mut buf);
                let num1 = u128::from_be_bytes(buf[0..16].try_into().unwrap());
                let num2 = u128::from_be_bytes(buf[16..32].try_into().unwrap());
                let num3 = u128::from_be_bytes(buf[32..48].try_into().unwrap());
                let num4 = u128::from_be_bytes(buf[48..64].try_into().unwrap());
                let num5 = u128::from_be_bytes(buf[64..80].try_into().unwrap());
                let num6 = u128::from_be_bytes(buf[80..96].try_into().unwrap());
                let num7 = u128::from_be_bytes(buf[96..112].try_into().unwrap());
                let num8 = u128::from_be_bytes(buf[112..128].try_into().unwrap());
                let mut write_str = alloc::format!("dequeue: {} {} {} {} {} {} {} {}\n\0",
                                                   num1, num2, num3, num4, num5, num6,
                                                   num7, num8);
                //c::printk_deferred(write_str.as_ptr() as *const i8);
                c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
            }
            ret
        }
    }

    pub fn enqueue(&mut self, val: T) {
        unsafe {
            let ret = (*self.inner).enqueue(self.inner, val);

            // make this size more correct.
            #[cfg(feature = "record")]
            {
                let mut buf = [0u8; 128];
                postcard::to_slice(&val, &mut buf);
                let num1 = u128::from_be_bytes(buf[0..16].try_into().unwrap());
                let num2 = u128::from_be_bytes(buf[16..32].try_into().unwrap());
                let num3 = u128::from_be_bytes(buf[32..48].try_into().unwrap());
                let num4 = u128::from_be_bytes(buf[48..64].try_into().unwrap());
                let num5 = u128::from_be_bytes(buf[64..80].try_into().unwrap());
                let num6 = u128::from_be_bytes(buf[80..96].try_into().unwrap());
                let num7 = u128::from_be_bytes(buf[96..112].try_into().unwrap());
                let num8 = u128::from_be_bytes(buf[112..128].try_into().unwrap());
                let mut write_str = alloc::format!("enqueue: {} {} {} {} {} {} {} {}\n\0",
                                                   num1, num2, num3, num4, num5, num6,
                                                   num7, num8);
                //c::printk_deferred(write_str.as_ptr() as *const i8);
                c::file_write_deferred(write_str.as_mut_ptr() as *mut i8);
            }
            ret
        }
    }

    pub unsafe fn from_raw(ptr: *mut raw::c_void, policy: i32) -> Self {
        Self {
            inner: ptr as *mut BufferInner<T>,
            policy: policy
        }
    }
}
