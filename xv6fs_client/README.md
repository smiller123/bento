This is a Rust file system. The structure is based on the xv6 file system,
but the file system includes a number of optimizations not included in xv6.

The file system is compiled as a Linux kernel module and depends on the
bentofs kernel module.

We use Linux kernel version 4.15 and Rust nightly version 1.43.0.

## Disk setup
**To create the disk image:**
In mkfs:
```
make
./mkfs fs.img
```

## Kernel version
**To compile:**
First, compile bentofs in a neighboring directory.
```
make
```

**To clean:**
```
cargo clean
```

**To insert:**
First, insert bentofs kernel module.
```
sudo insmod kernel/xv6fs.ko
```

**To mount file system:**
```
sudo mkdir -p /mnt/xv6fsll
sudo mount -t bentoblk -o loop -o rootmode=40000,user_id=0,group_id=0,blksize=4096,name=xv6fs_ll mkfs/fs.img /mnt/xv6fsll
```

**To unmount file system:**
```
sudo umount /mnt/xv6fsll
```

**To remove module:**
```
sudo rmmod xv6fs
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

**To mount/insert:**
```
sudo userspace/target/release/user_xv6fs mkfs/fs.img /mnt/xv6fsll blkdev
```

**To unmount:**
```
sudo fusermount -u /mnt/xv6fsll
```
