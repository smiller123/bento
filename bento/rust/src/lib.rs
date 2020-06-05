#![feature(lang_items)]
#![feature(concat_idents)]
#![feature(const_fn)]
#![feature(allocator_api)]
#![feature(alloc_error_handler)]
#![feature(alloc_layout_extra)]
#![feature(panic_info_message)]
#![no_std]

extern crate alloc;

pub mod bindings;
#[macro_use]
pub mod io;
#[allow(non_upper_case_globals)]
pub mod bentofs;
pub mod kernel;

extern crate datablock;
extern crate rlibc;
pub use datablock::*;

// These functions and traits are used by the compiler, but not
// for a bare-bones hello world. These are normally
// provided by libstd.
#[no_mangle]
#[lang = "eh_personality"]
pub fn eh_personality() {}

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[global_allocator]
static ALLOCATOR: kernel::allocator::KernelAllocator = kernel::allocator::KernelAllocator;
