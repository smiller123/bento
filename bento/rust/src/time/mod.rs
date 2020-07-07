#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Timespec {
    pub sec: i64,
    pub nsec: i32,
}

impl Timespec {
    pub const fn new(sec: i64, nsec: i32) -> Self {
        Self {
            sec: sec,
            nsec: nsec,
        }
    }
}
