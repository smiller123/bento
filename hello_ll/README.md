This is a simple Rust file system. The file system consists of one file called hello, which can
be read and written to. The file system doesn't support file creation or deletion.

The file system is compiled as a Linux kernel module and depends on the
bentofs kernel module.

We use Linux kernel version 4.15 and Rust nightly version 1.43.0.

## Disk setup
**To make a RAM Disk:**
```
sudo modprobe brd rd_nr=1 rd_size=20971520 max_part=0
```

**To load a disk image to the disk:**
```
sudo dd if=$FS_IMG of=$DEV_FILE
```
For example:
```
sudo dd if=fs.img of=/dev/ram0
``` 

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
```
sudo mkdir -p /mnt/hello_ll
sudo mount -t bentoblk -o fd=10,rootmode=40000,user_id=0,group_id=0,blksize=4096,name=hello_ll $DEV_FILE /mnt/hello_ll
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
sudo userspace/target/release/user_hello $DEV_FILE /mnt/hello_ll blkdev
```

**To unmount:**
```
sudo fusermount -u /mnt/hello_ll
```
