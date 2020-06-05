use core;
//use core::mem;
use core::slice;
use kernel::ffi::*;
use kernel::raw::*;
use rlibc;

/// Memcpy between two `MemContainer`s.
///
/// The two `MemContainer`s must be of the same and must both be at least as long as the requested
/// size. If either of the `MemContainer`s is too small, the function will return an error.
pub fn memcpy_rust<T>(
    to: &mut MemContainer<T>,
    from: &MemContainer<T>,
    _size: c_size_t,
) -> Result<(), i32> {
    let size = _size as usize;
    if to.len() < size || from.len() < size {
        return Err(-1);
    }
    if size % core::mem::size_of::<T>() != 0 {
        return Err(-1);
    }
    unsafe {
        rlibc::memcpy(
            to.as_mut_ptr() as *mut u8,
            from.as_mut_ptr() as *const u8,
            _size as usize,
        );
    }
    return Ok(());
}

/// Copy a Rust `str` into a `MemContainer<c_uchar>`.
///
/// If the `MemContainer` isn't long enough to hold the string, this function will return an error.
/// Otherwise, it will memcpy the string.
pub fn strcpy_rust(to: &MemContainer<c_uchar>, from: &str) -> Result<(), i32> {
    let size = from.len();
    if to.len() < size {
        return Err(-1);
    }
    unsafe {
        rlibc::memcpy(to.as_mut_ptr() as *mut u8, from.as_ptr() as *const u8, size);
    }
    return Ok(());
}

/// Memset a `MemContainer<c_uchar>` for the given size.
///
/// If the `MemContainer` isn't large enough, this function will return an error. Otherwise, it
/// will memset the memory region.
pub fn memset_rust(s: &mut MemContainer<c_uchar>, c: u8, _n: c_size_t) -> Result<(), i32> {
    let n = _n as usize;
    if s.len() < n {
        return Err(-1);
    }
    unsafe {
        rlibc::memset(s.as_mut_ptr() as *mut u8, c as i32, n);
    }
    Ok(())
}

/// A data structure to represent a sized, typed blob of memory.
///
/// This data structure handles calling kmalloc and kfree for the user, enabling Rust memory
/// semantics for kernel-allocated memory.
///
/// This can represent memory that's owned by Rust or memory that's owned by the kernel and only
/// borrowed by Rust. For memory owned by Rust and allocated within Rust, the `drop` field will be
/// set to true, and the memory will be reclaimed when the `MemContainer` goes out of scope. For
/// memory allocated in and owned by C, `drop` will be false, and the memory won't be reclaimed.
///
/// This data structure is much like a slice, and in the future, we may move to using slices in
/// place of MemContainer where possible.
#[repr(C)]
#[derive(Debug)]
pub struct MemContainer<T> {
    ptr: *mut T,
    len: usize,
    drop: bool,
}

impl<T> MemContainer<T> {
    /// Create a `MemContainer` given a pointer and a size.
    ///
    /// This is unsafe because it must assume that the pointer is non-NULL and points to a valid
    /// memory region of the correct size.
    pub unsafe fn new_from_raw(ptr: *mut T, len: usize) -> Self {
        MemContainer {
            ptr: ptr,
            len: len,
            drop: false,
        }
    }

    /// Allocate a new `MemContainer` of the correct size.
    ///
    /// Arguments:
    /// * `size: usize`: The size of the new `MemContainer` in bytes.
    pub fn alloc(size: usize) -> Option<Self> {
        unsafe {
            let ptr = __kmalloc(size as c_size_t, 0x90) as *mut T;
            if ptr.is_null() {
                None
            } else {
                Some(MemContainer {
                    ptr: ptr,
                    len: size,
                    drop: true,
                })
            }
        }
    }

    /// Convert a mutable `MemContainer` into a mutable slice.
    pub fn to_slice_mut<'a>(&'a mut self) -> &'a mut [T] {
        unsafe {
            let size = self.len / core::mem::size_of::<T>();
            slice::from_raw_parts_mut(self.as_mut_ptr(), size)
        }
    }

    /// Convert an immutable `MemContainer` into an immutable slice.
    pub fn to_slice<'a>(&'a self) -> &'a [T] {
        unsafe {
            let size = self.len / core::mem::size_of::<T>();
            slice::from_raw_parts(self.as_ptr(), size)
        }
    }

    /// Create a `MemContainer` of one type from a `MemContainer` of another type.
    ///
    /// The type of the resulting `MemContainer` must implement `From` on the type of the initial
    /// `MemContainer`.
    pub fn into_container<U: From<T>>(self) -> Option<MemContainer<U>> {
        if self.len % core::mem::size_of::<U>() == 0 {
            Some(MemContainer::<U> {
                ptr: self.ptr as *mut U,
                len: self.len,
                drop: self.drop,
            })
        } else {
            None
        }
    }

    /// Get the length of the memory region.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Get a mutable reference to the `MemContainer`'s underlying type.
    ///
    /// Returns None if the `MemContainer` isn't large enough.
    pub fn to_mut<'a>(&'a mut self) -> Option<&'a mut T> {
        if core::mem::size_of::<T>() > self.len {
            None
        } else {
            unsafe { self.ptr.as_mut() }
        }
    }

    /// Get a reference to the `MemContainer`'s underlying type.
    ///
    /// Returns None if the `MemContainer` isn't large enough.
    pub fn to_ref(&self) -> Option<&T> {
        if core::mem::size_of::<T>() > self.len {
            None
        } else {
            let struct_ptr: *const T = self.ptr as *const T;
            unsafe { struct_ptr.as_ref() }
        }
    }

    /// Get a raw mut pointer to the memory region.
    pub unsafe fn as_mut_ptr(&self) -> *mut T {
        self.ptr
    }

    /// Get a raw const pointer to the memory region.
    pub unsafe fn as_ptr(&self) -> *const T {
        self.ptr as *const T
    }

    /// Truncate the `MemContainer` to `new_len` bytes.
    pub fn truncate(&mut self, new_len: usize) -> () {
        if new_len < self.len {
            self.len = new_len;
        }
    }
}

impl<T> Drop for MemContainer<T> {
    fn drop(&mut self) {
        if self.drop {
            unsafe {
                kfree(self.ptr as *mut c_void);
            }
        }
    }
}
