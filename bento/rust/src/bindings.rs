/*
 * SPDX-License-Identifier: GPL-2.0
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 *
 * Based on code from fishinabarrel/linux-kernel-module-rust on Github
 *
 */

#![allow(non_camel_case_types, non_upper_case_globals, non_snake_case, improper_ctypes)]

/// Taken from fishinabarrel/linux-kernel-module-rust on Github
use crate::kernel::raw;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub const GFP_KERNEL: gfp_t = BINDINGS_GFP_KERNEL;
