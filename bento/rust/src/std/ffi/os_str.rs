use crate::hash32::{Hash, Hasher};
use core::mem;
use core::str;

/// Copy of Rust libstd Slice
pub struct Slice {
    pub inner: [u8],
}

impl Slice {
    fn from_u8_slice(s: &[u8]) -> &Slice {
        unsafe { mem::transmute(s) }
    }

    pub fn from_str(s: &str) -> &Slice {
        Slice::from_u8_slice(s.as_bytes())
    }

    pub fn to_str(&self) -> Option<&str> {
        str::from_utf8(&self.inner).ok()
    }
}

// Hash for Slice
impl Hash for Slice {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

pub struct SliceHasher {
    state: u32,
}

impl SliceHasher {
    pub fn new() -> Self {
        SliceHasher { state: 5381 as u32 }
    }

    // djb2_hash
    pub fn write_u8(&mut self, i: u8) {
        self.state = ((self.state << 5) + self.state) + i as u32;
    }
}

impl Hasher for SliceHasher {
    fn finish(&self) -> u32 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        for i in bytes {
            self.write_u8(*i);
        }
    }
}

/// Copy of Rust libstd OsStr
pub struct OsStr {
    pub inner: Slice,
}

impl OsStr {
    pub fn new<S: AsRef<OsStr> + ?Sized>(s: &S) -> &OsStr {
        s.as_ref()
    }

    fn from_inner(inner: &Slice) -> &OsStr {
        unsafe { &*(inner as *const Slice as *const OsStr) }
    }

    pub fn to_str(&self) -> Option<&str> {
        self.inner.to_str()
    }

    pub fn len(&self) -> usize {
        self.inner.inner.len()
    }
}

impl AsRef<OsStr> for str {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        OsStr::from_inner(Slice::from_str(self))
    }
}
