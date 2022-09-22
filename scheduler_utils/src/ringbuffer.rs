/// Based on ringbuffer on Github https://github.com/NULLx76/ringbuffer

extern crate serde;
extern crate postcard;
//extern crate alloc;
// We need vecs so depend on alloc
//use alloc::vec::Vec;
use core::fmt::Debug;
use std::io;
use std::io::BufRead;
use std::fs::File;
use std::path::Path;
use std::marker::PhantomData;
//use core::iter::FromIterator;
//use core::mem;
//use core::mem::MaybeUninit;
//use core::slice;
//use kernel::raw;
//use bindings as c; 

use self::serde::{Serialize, Deserialize};
use self::serde::de::DeserializeOwned;

//pub struct BufferInner<T> {
//    pub offset: u32,
//    pub capacity: u32,
//    pub writeptr: u32,
//    pub readptr: u32,
//    pub val: T,
//}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}


pub struct RingBuffer<T> {
    lines: io::Lines<io::BufReader<File>>,
    phantom: PhantomData<T>,
    buf_vec: Vec<[u8; 128]>,
//    ref_vec: Vec<&'a [u8; 128]>
//    buf: Vec<MaybeUninit<T>>,
    //pub inner: *mut BufferInner<T>
}

//impl<T: Copy + Debug> BufferInner<T> {
//    fn len(&self) -> u32 {
//        self.writeptr - self.readptr
//    }
//
//    #[inline]
//    fn is_empty(&self) -> bool {
//        self.len() == 0
//    }
//
//    #[inline]
//    fn is_full(&self) -> bool {
//        // Does this actually loop correctly?
//        self.len() == self.capacity
//    }
//
//    fn dequeue(&mut self, ptr: *mut Self) -> Option<T> {
//        if self.is_empty() {
//            None
//        } else {
//            let index = self.readptr & (self.capacity - 1);
//            let start = (ptr as u64 + self.offset as u64) as *mut T;
//            println!("start {:?}", start);
//            let slice_buf = unsafe {
//                slice::from_raw_parts_mut(start, self.capacity as usize)
//            };
//            println!("index {}", index);
//            let res = slice_buf[index as usize];
//            self.readptr += 1;
//            unsafe { Some(res) }
//        }
//    }
//}

impl<T: Copy + Serialize + DeserializeOwned> RingBuffer<T> {
    //fn len(&self) -> u32 {
    //    unsafe {
    //        (*self.inner).len()
    //    }
    //}

    //#[inline]
    //fn is_empty(&self) -> bool {
    //    unsafe {
    //        (*self.inner).is_empty()
    //    }
    //}

    //#[inline]
    //fn is_full(&self) -> bool {
    //    unsafe {
    //        (*self.inner).is_full()
    //    }
    //    // Does this actually loop correctly?
    //}

    pub fn dequeue(&mut self) -> Option<T> {
        loop {
            let line_res = self.lines.next();
            if let Some(line_res2) = line_res {
                let mut line = line_res2.unwrap();
                let mut split = line.split_whitespace();
                if split.next().unwrap() == "dequeue:" {
                    let mut buf = [0u8; 128];
                    let num1: u128 = split.next().unwrap().parse().unwrap();
                    let num2: u128 = split.next().unwrap().parse().unwrap();
                    let num3: u128 = split.next().unwrap().parse().unwrap();
                    let num4: u128 = split.next().unwrap().parse().unwrap();
                    let num5: u128 = split.next().unwrap().parse().unwrap();
                    let num6: u128 = split.next().unwrap().parse().unwrap();
                    let num7: u128 = split.next().unwrap().parse().unwrap();
                    let num8: u128 = split.next().unwrap().parse().unwrap();

                    let buf1 = num1.to_be_bytes();
                    let buf2 = num2.to_be_bytes();
                    let buf3 = num3.to_be_bytes();
                    let buf4 = num4.to_be_bytes();
                    let buf5 = num5.to_be_bytes();
                    let buf6 = num6.to_be_bytes();
                    let buf7 = num7.to_be_bytes();
                    let buf8 = num8.to_be_bytes();

                    buf[0..16].copy_from_slice(&buf1);
                    buf[16..32].copy_from_slice(&buf2);
                    buf[32..48].copy_from_slice(&buf3);
                    buf[48..64].copy_from_slice(&buf4);
                    buf[64..80].copy_from_slice(&buf5);
                    buf[80..96].copy_from_slice(&buf6);
                    buf[96..112].copy_from_slice(&buf7);
                    buf[112..128].copy_from_slice(&buf8);
                    return postcard::from_bytes(&buf).unwrap();
                }
            } else {
                return None;
            }
        }
    }

    pub fn from_file<P>(filename: P) -> Self where P: AsRef<Path> {
        let lines = read_lines(filename).unwrap();
        Self {
            lines: lines,
            phantom: PhantomData,
            buf_vec: Vec::new()
        }
    }

    //pub unsafe fn from_raw(ptr: *mut raw::c_void) -> Self {
    //    Self {
    //        inner: ptr as *mut BufferInner<T>
    //    }
    //}
}
