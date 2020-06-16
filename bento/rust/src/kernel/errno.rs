/*
 * SPDX-License-Identifier: GPL-2.0
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 *
 */

use crate::bindings::*;

pub enum Error {
    EPERM = -(EPERM as isize),
    ENOENT = -(ENOENT as isize),
    EIO = -(EIO as isize),
    ENOMEM = -(ENOMEM as isize),
    EEXIST = -(EEXIST as isize),
    ENOTDIR = -(ENOTDIR as isize),
    EISDIR = -(EISDIR as isize),
    EINVAL = -(EINVAL as isize),
    ENOSYS = -(ENOSYS as isize),
    ENAMETOOLONG = -(ENAMETOOLONG as isize),
    EOVERFLOW = -(EOVERFLOW as isize),
}
