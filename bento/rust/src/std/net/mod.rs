mod addr;
pub use self::addr::*;

mod ip;
pub use self::ip::*;

mod tcp;
pub use self::tcp::*;

#[inline]
pub const fn htons(i: u16) -> u16 {
    i.to_be()
}
#[inline]
pub const fn ntohs(i: u16) -> u16 {
    u16::from_be(i)
}

