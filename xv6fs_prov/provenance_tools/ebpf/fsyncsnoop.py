#! /usr/bin/python2
# @lint-avoid-python-3-compatibility-imports
#
# syncsnoop Trace sync() syscall.
#           For Linux, uses BCC, eBPF. Embedded C.
#
# Written as a basic example of BCC trace & reformat. See
# examples/hello_world.py for a BCC trace with default output example.
#
# Copyright (c) 2015 Brendan Gregg.
# Licensed under the Apache License, Version 2.0 (the "License")
#
# 13-Aug-2015   Brendan Gregg   Created this.
# 19-Feb-2016   Allan McAleavy migrated to BPF_PERF_OUTPUT

# TODO update this copyright stuff

from __future__ import print_function
from bcc import BPF
import ctypes as ct

# load BPF program
b = BPF(text="""
#include <linux/fs.h>
struct data_t {
    u64 ts;
    u64 fn;
    int error;
};

BPF_PERF_OUTPUT(events);

void kprobe__vfs_fsync_range(struct pt_regs *ctx, struct file *file, loff_t start, loff_t end, int datasync) {
    struct data_t data = {};
    struct inode *inode = file->f_mapping->host;
    data.fn = (unsigned long long) file->f_op->fsync;
    data.error = (!file->f_op->fsync) || (!datasync && (inode->i_state & I_DIRTY_TIME));
    data.ts = bpf_ktime_get_ns() / 1000;
    events.perf_submit(ctx, &data, sizeof(data));
};
""")

class Data(ct.Structure):
    _fields_ = [
        ("ts", ct.c_ulonglong),
        ("fn", ct.c_ulonglong),
        ("error", ct.c_int)
    ]

# header
print("%-18s %s" % ("TIME(s)", "CALL"))

# process event
def print_event(cpu, data, size):
    event = ct.cast(data, ct.POINTER(Data)).contents
    print("%-18.9f fsync(), fn_ptr=%lx" % (float(event.ts) / 1000000, event.fn))
    if event.error != 0:
        print("error")

# loop with callback to print_event
b["events"].open_perf_buffer(print_event)
while 1:
    b.kprobe_poll()
