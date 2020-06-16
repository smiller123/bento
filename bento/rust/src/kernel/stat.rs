/*
 * SPDX-License-Identifier: GPL-2.0
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 *
 */

pub const S_IFMT: u16 = 0o170000;
pub const S_IFSOCK: u16 = 0o140000;
pub const S_IFLNK: u16 = 0o120000;
pub const S_IFREG: u16 = 0o100000;
pub const S_IFBLK: u16 = 0o60000;
pub const S_IFDIR: u16 = 0o40000;
pub const S_IFCHR: u16 = 0o20000;
pub const S_IFIFO: u16 = 0o10000;
pub const S_ISUID: u16 = 0o4000;
pub const S_ISGID: u16 = 0o2000;
pub const S_ISVTX: u16 = 0o1000;

pub const S_IRWXU: u16 = 0o700;
pub const S_IRUSR: u16 = 0o400;
pub const S_IWUSR: u16 = 0o200;
pub const S_IXUSR: u16 = 0o100;

pub const S_IRWXG: u16 = 0o70;
pub const S_IRGRP: u16 = 0o40;
pub const S_IWGRP: u16 = 0o20;
pub const S_IXGRP: u16 = 0o10;

pub const S_IRWXO: u16 = 0o7;
pub const S_IROTH: u16 = 0o4;
pub const S_IWOTH: u16 = 0o2;
pub const S_IXOTH: u16 = 0o1;

pub const S_IRWXUGO: u16 = S_IRWXU | S_IRWXG | S_IRWXO;
pub const S_IALLUGO: u16 = S_ISUID | S_ISGID | S_ISVTX | S_IRWXUGO;
pub const S_IRUGO: u16 = S_IRUSR | S_IRGRP | S_IROTH;
pub const S_IWUGO: u16 = S_IWUSR | S_IWGRP | S_IWOTH;
pub const S_IXUGO: u16 = S_IXUSR | S_IXGRP | S_IXOTH;
