This is a version of the Bento xv6fs file system including file provenance tracking.

System call level provenance tracking is included in the `tracing` directory.

The file system is compiled as a Linux kernel module and depends on the
bentofs kernel module.

We use Linux kernel version 4.15 and Rust nightly version 1.43.0.

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
sudo insmod kernel/xv6fs_prov.ko
```

**To mount file system:**
Mount the file system using `mkfs/fs.img` as a loop device.
```
sudo mkdir -p /mnt/xv6fsll
sudo mount -t bentoblk -o loop -o rootmode=40000,user_id=0,group_id=0,blksize=4096,name=xv6fs_ll ../xv6fs/mkfs/fs.img /mnt/xv6fsll
```

**To unmount file system:**
```
sudo umount /mnt/xv6fsll
```

**To remove module:**
```
sudo rmmod xv6fs_prov
```

## Redepoyable version
To compile to be deployed on top of an existing file system instead of as a standalone file system, swich `register_bento_fs` in `kernel/lib.rs` to `reregister_bento_fs` and recompile.

To deploy on top of an existing file system, insert the module while another version of the `xv6fs` file system is inserted and/or mounted. This file system will relpace the existing one, and the previous file system module can be removed.

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
sudo userspace/target/release/user_xv6fs ../xv6fs/mkfs/fs.img /mnt/xv6fsll
```
If using a physical block device to back the file system, add ```blkdev``` to the end of the command.

This will start a process that will remain running while the FUSE file system is mounted.

**To unmount:**
```
sudo fusermount -u /mnt/xv6fsll
```
