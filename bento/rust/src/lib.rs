/*
* SPDX-License-Identifier: GPL-2.0
* Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
     Anderson, Ang Chen, University of Washington
*
*/

#![feature(unboxed_closures)]
#![feature(fn_traits)]

#![feature(lang_items)]
#![feature(concat_idents)]
#![feature(allocator_api)]
#![feature(alloc_error_handler)]
#![feature(alloc_layout_extra)]
#![feature(panic_info_message)]
#![feature(rustc_private)]
#![no_std]

extern crate alloc;
extern crate serde;
extern crate spin;

#[cfg(feature = "capnproto")]
extern crate capnp;

pub mod bento_utils;
pub mod bindings;
#[macro_use]
pub mod io;
#[allow(non_upper_case_globals)]
pub mod fuse;
pub mod kernel;
#[allow(non_camel_case_types)]
pub mod libc;
pub mod scheduler_utils;
pub mod std;
pub mod time;
pub mod spin_rs;

extern crate datablock;
extern crate hash32;
pub use datablock::*;

// These functions and traits are used by the compiler, but not
// for a bare-bones hello world. These are normally
// provided by libstd.

#[no_mangle]
#[lang = "eh_personality"]
// #[cfg(not(test))]
pub fn eh_personality() {}

/// Declares the entrypoint for a kernel module. The first argument should be a type which
/// implements the [`KernelModule`] trait. Also accepts various forms of kernel metadata.
///
/// Example:
/// ```rust,no_run
/// use linux_kernel_module;
/// struct MyKernelModule;
/// impl linux_kernel_module::KernelModule for MyKernelModule {
///     fn init() -> linux_kernel_module::KernelResult<Self> {
///         Ok(MyKernelModule)
///     }
/// }
///
/// linux_kernel_module::kernel_module!(
///     MyKernelModule,
///     author: b"Fish in a Barrel Contributors",
///     description: b"My very own kernel module!",
///     license: b"GPL"
/// );
#[macro_export]
macro_rules! kernel_module {
    ($module:ty, $($name:ident : $value:expr),*) => {
        static mut __MOD: Option<$module> = None;
        #[no_mangle]
        pub extern "C" fn init_module() -> $crate::kernel::raw::c_int {
            match <$module as $crate::KernelModule>::init() {
                Ok(m) => {
                    unsafe {
                        __MOD = Some(m);
                    }
                    return 0;
                }
                Err(e) => {
                    return e;
                }
            }
        }

        #[no_mangle]
        pub extern "C" fn cleanup_module() {
            unsafe {
                // Invokes drop() on __MOD, which should be used for cleanup.
                __MOD = None;
            }
        }

        $(
            $crate::kernel_module!(@attribute $name, $value);
        )*
    };

    // TODO: The modinfo attributes below depend on the compiler placing
    // the variables in order in the .modinfo section, so that you end up
    // with b"key=value\0" in order in the section. This is a reasonably
    // standard trick in C, but I'm not sure that rustc guarantees it.
    //
    // Ideally we'd be able to use concat_bytes! + stringify_bytes! +
    // some way of turning a string literal (or at least a string
    // literal token) into a bytes literal, and get a single static
    // [u8; * N] with the whole thing, but those don't really exist yet.
    // Most of the alternatives (e.g. .as_bytes() as a const fn) give
    // you a pointer, not an array, which isn't right.

    (@attribute author, $value:expr) => {
        #[link_section = ".modinfo"]
        #[used]
        pub static AUTHOR_KEY: [u8; 7] = *b"author=";
        #[link_section = ".modinfo"]
        #[used]
        pub static AUTHOR_VALUE: [u8; $value.len()] = *$value;
        #[link_section = ".modinfo"]
        #[used]
        pub static AUTHOR_NUL: [u8; 1] = *b"\0";
    };

    (@attribute description, $value:expr) => {
        #[link_section = ".modinfo"]
        #[used]
        pub static DESCRIPTION_KEY: [u8; 12] = *b"description=";
        #[link_section = ".modinfo"]
        #[used]
        pub static DESCRIPTION_VALUE: [u8; $value.len()] = *$value;
        #[link_section = ".modinfo"]
        #[used]
        pub static DESCRIPTION_NUL: [u8; 1] = *b"\0";
    };

    (@attribute license, $value:expr) => {
        #[link_section = ".modinfo"]
        #[used]
        pub static LICENSE_KEY: [u8; 8] = *b"license=";
        #[link_section = ".modinfo"]
        #[used]
        pub static LICENSE_VALUE: [u8; $value.len()] = *$value;
        #[link_section = ".modinfo"]
        #[used]
        pub static LICENSE_NUL: [u8; 1] = *b"\0";
    };
}

/// KernelModule is the top level entrypoint to implementing a kernel module. Your kernel module
/// should implement the `init` method on it, which maps to the `module_init` macro in Linux C API.
/// You can use this method to do whatever setup or registration your module should do. For any
/// teardown or cleanup operations, your type may implement [`Drop`].
///
/// [`Drop`]: https://doc.rust-lang.org/stable/core/ops/trait.Drop.html
pub trait KernelModule: Sized + Sync {
    fn init() -> Result<Self, i32>;
}

use core::panic::PanicInfo;

#[panic_handler]
// #[cfg(not(test))]
fn panic(_info: &PanicInfo) -> ! {
    println!("panicing and it's bad\n");
    loop {}
}

#[global_allocator]
static ALLOCATOR: kernel::allocator::KernelAllocator = kernel::allocator::KernelAllocator;
