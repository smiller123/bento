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

# TODO update copyrite stuff

from __future__ import print_function
from bcc import BPF
import ctypes as ct

# load BPF program
b = BPF(text="""
#include <asm/current.h>
#include <linux/sched.h>

struct data_t {
    u32 type;
    u64 ts;
    u32 pid;
    u32 ppid;
    u32 dup_fd;
    int sendmsg_fd;
    //char *execname;
};

BPF_PERF_OUTPUT(events);

void kprobe__sys_clone(void *ctx) {
    struct data_t data = {};
    struct task_struct *myproc = (struct task_struct *) bpf_get_current_task();
    data.type = 0;
    data.ts = bpf_ktime_get_ns() / 1000;
    data.pid = myproc->pid;
    data.ppid = myproc->real_parent->pid;
    //data.execname = myproc->comm;
    events.perf_submit(ctx, &data, sizeof(data));
};

void kprobe__sys_execve(void *ctx) {
    struct data_t data = {};
    struct task_struct *myproc = (struct task_struct *) bpf_get_current_task();
    data.type = 1;
    data.ts = bpf_ktime_get_ns() / 1000;
    data.pid = myproc->pid;
    data.ppid = myproc->real_parent->pid;
    events.perf_submit(ctx, &data, sizeof(data));
};

void kprobe__sys_pipe(void *ctx) {
    struct data_t data = {};
    struct task_struct *myproc = (struct task_struct *) bpf_get_current_task();
    data.type = 2;
    data.ts = bpf_ktime_get_ns() / 1000;
    data.pid = myproc->pid;
    data.ppid = myproc->real_parent->pid;
    events.perf_submit(ctx, &data, sizeof(data));
};

void kprobe__sys_dup(struct pt_regs  *ctx, unsigned int fildes) {
    struct data_t data = {};
    struct task_struct *myproc = (struct task_struct *) bpf_get_current_task();
    data.type = 3;
    data.ts = bpf_ktime_get_ns() / 1000;
    data.pid = myproc->pid;
    data.ppid = myproc->real_parent->pid;
    data.dup_fd = fildes;
    events.perf_submit(ctx, &data, sizeof(data));
};

void kprobe__sys_sendmsg(struct pt_regs *ctx, int fd) {
    struct data_t data = {};
    struct task_struct *myproc = (struct task_struct *) bpf_get_current_task();
    data.type = 4;
    data.ts = bpf_ktime_get_ns() / 1000;
    data.pid = myproc->pid;
    data.ppid = myproc->real_parent->pid;
    data.sendmsg_fd = fd;
    events.perf_submit(ctx, &data, sizeof(data));
};
""")

class Data(ct.Structure):
    _fields_ = [
        ("type", ct.c_int),
        ("ts", ct.c_ulonglong),
        ("pid", ct.c_int),
        ("ppid", ct.c_int),
        ("dup_fd", ct.c_int),
        ("sendmsg_fd", ct.c_int) 
    ]

# header
print("%-18s %s" % ("TIME(s)", "CALL"))


"""
Notes:
clone is used instead of fork, basically the same thing
for some reason EXECS are showing up before the CLONE for that process shows up, which is counter intuitive
I'll add other information for pipes and dups
Need to find info about message recipient in sendmsg
"""

# process event
def print_event(cpu, data, size):
    event = ct.cast(data, ct.POINTER(Data)).contents
    syscall_names = ["CLONE", "EXEC", "PIPE", "DUP", "SENDMSG"]
    if event.type == 0:
        print("%-18.9f %s(), pid=%d, ppid=%d" % (float(event.ts) / 1000000, "CLONE", event.pid, event.ppid))
    elif event.type == 1:
        print("%-18.9f %s(), pid=%d, ppid=%d" % (float(event.ts) / 1000000, "EXEC", event.pid, event.ppid))
    elif event.type == 2:
        print("%-18.9f %s(), pid=%d" % (float(event.ts) / 1000000, "PIPE", event.pid))
    elif event.type == 3:
        print("%-18.9f %s(), pid=%d, fd=%u" % (float(event.ts) / 1000000, "DUP", event.pid, event.dup_fd))
    elif event.type == 4:
        print("%-18.9f %s(), pid=%d, fd=%i" % (float(event.ts) / 1000000, "SENDMSG", event.pid, event.sendmsg_fd))


# loop with callback to print_event
b["events"].open_perf_buffer(print_event)
while 1:
    b.kprobe_poll()
