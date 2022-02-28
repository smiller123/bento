use crate::hash32::{Hash, Hasher};
use core::mem;
use core::str;

/// Copy of Rust libstd Slice
pub struct Slice {
    pub inner: [u8],
}

impl Slice {
    pub fn from_u8_slice(s: &[u8]) -> &Slice {
        unsafe { mem::transmute(s) }
    }

    pub fn from_str(s: &str) -> &Slice {
        Slice::from_u8_slice(s.as_bytes())
    }

    pub fn to_str(&self) -> Option<&str> {
        str::from_utf8(&self.inner).ok()
    }
}

/// Copy of Rust libstd OsStr
pub struct OsStr {
    inner: Slice,
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

    fn bytes(&self) -> &[u8] {
        unsafe { &*(&self.inner as *const Slice as *const [u8]) }
    }
}

impl Hash for OsStr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bytes().hash(state);
    }
}

impl AsRef<OsStr> for str {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        OsStr::from_inner(Slice::from_str(self))
    }
}
