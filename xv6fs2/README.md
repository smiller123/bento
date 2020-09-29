This is a Rust file system. The structure is based on the xv6 file system,
but the file system includes a number of optimizations not included in xv6.

This is a redepoyable version of `xv6fs` that can be deployed on top of `xv6fs`.
The main difference between this file system and `xv6fs` is that this uses `reregister_bento_fs`
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
This file system should be inserted after an existing `xv6fs` file system has been inserted and/or mounted.
```
sudo insmod kernel/xv6fs2.ko
```

**To remove module:**
```
sudo rmmod xv6fs2
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
sudo mkdir -p /mnt/xv6fsll
sudo userspace/target/release/xv6fs ../xv6fs/mkfs/fs.img /mnt/xv6fsll blkdev
```

**To unmount:**
```
sudo fusermount -u /mnt/xv6fsll
```
