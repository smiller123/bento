This is a Rust re-implementation of the xv6 file system using Bento.

The file system is compiled as a Linux kernel module and depends on the
bentofs kernel module.

We use Linux kernel version 4.15 and Rust nightly version 1.43.0.

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
sudo insmod xv6fs.ko
```

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

**To mount file system:**
```
sudo mkdir /mnt/xv6fsll
sudo mount -t bentoblk -o fd=10,rootmode=40000,user_id=0,group_id=0,blksize=4096,name=xv6fs_ll $DEV_FILE /mnt/xv6fsll
```

**To unmount file system:**
```
sudo umount /mnt/xv6fsll
```

**To remove module:**
```
sudo rmmod xv6fs
```
