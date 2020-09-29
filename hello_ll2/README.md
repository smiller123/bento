This is a simple Rust file system. The file system consists of one file called hello, which can be read and written to. The file system doesn't support file creation or deletion.

This is a redepoyable version of `hello_ll` that can be deployed on top of `hello_ll`.
The main difference between this file system and `hello_ll` is that this uses `reregister_bento_fs`
in `kernel/lib.rs` instead of `register_bento_fs`.

The file system is compiled as a Linux kernel module and depends on the
bentofs kernel module.

We use Linux kernel version 4.15 and Rust nightly version 1.43.0.

## Kernel Version
**To compile:**
First, compile bentofs in a neighboring directory.
```
make
```

**To clean:**
```
make clean
```

**To insert:**
This file system should be inserted after an existing `hello_ll` file system has been inserted and/or mounted.
```
sudo insmod kernel/hello_ll2.ko
```

**To remove module:**
```
sudo rmmod hello_ll2
```

## User version
**To compile:**
```
make userspace
```

**To clean:**
```
make clean
```

**To insert/mount:**
```
sudo mkdir -p /mnt/hello_ll
sudo userspace/target/release/user_hello hello /mnt/hello_ll blkdev
```

**To unmount:**
```
sudo fusermount -u /mnt/hello_ll
```
