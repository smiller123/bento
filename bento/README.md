Usage
-----

Build the code by running:

```make clean
make
```

Create a RAMDisk by running:

```sudo modprobe brd rd_nr=1 rd_size=2097152 max_part=0```
```sudo dd if=$(FS_IMG) of=/dev/ram0```

Insert the kernel module by running:

```sudo insmod module/xv6fs_ll.ko```

Mount a file system by running:

```sudo mount -t myfuseblk -o fd=10,rootmode=40000,user_id=0,group_id=0,blksize=4096 -o loop /dev/ram0 /mnt/xv6```

The current file system image is fs4.img. It has two files, the binary `initcode`, and the
C file `init.c`. Other file system images are older versions.

To test the file system, run:

``sudo ls /mnt/xv6
sudo cat /mnt/xv6/init.c
sudo touch /mnt/xv6/hi
sudo su
echo "Hello, World!" >> /mnt/xv6/init.c
echo "Hello, World!" >> /mnt/xv6/init.c
```
