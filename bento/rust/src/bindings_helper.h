/*
 * SPDX-License-Identifier: GPL-2.0
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 *
 * Based on code from fishinabarrel/linux-kernel-module-rust on Github
 *
 */

#include <linux/cdev.h>
#include <uapi/linux/fuse.h>
#include <linux/fs.h>
#include <linux/module.h>
#include <linux/random.h>
#include <linux/slab.h>
#include <linux/uaccess.h>
#include <linux/version.h>
#include <linux/in.h>
#include <linux/in6.h>
#include <linux/net.h>
#include <uapi/linux/tcp.h>
#include <linux/uio.h>
#include <linux/socket.h>
#include <linux/kthread.h>
#include <net/sock.h>
#include <net/tcp.h>
#include <net/inet_connection_sock.h>

// Bindgen gets confused at certain things
//
const gfp_t BINDINGS_GFP_KERNEL = GFP_KERNEL;
