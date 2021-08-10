This is a simple Rust file system. The file system consists of one file called hello, which can
be read and written to. The file system doesn't support file creation or deletion.

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
First, insert bentofs kernel module, described in the bentofs directory.
```
sudo insmod kernel/hello_ll.ko
```

**To mount file system:**
Mount the file system as a loop device using the `hello` file as the device.

```
sudo mkdir -p /mnt/hello_ll
sudo mount -t bentoblk -o loop -o rootmode=40000,user_id=0,group_id=0,blksize=4096,name=hello_ll hello /mnt/hello_ll
```

**To unmount file system:**
```
sudo umount /mnt/hello_ll
```

**To remove module:**
```
sudo rmmod hello_ll
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
sudo userspace/target/release/user_hello hello /mnt/hello_ll
```
If using a physical block device to back the file system, add ```blkdev``` to the end of the second command.

This will start a process that will remain running while the FUSE file system is mounted.

**To unmount:**
```
sudo fusermount -u /mnt/hello_ll
```
