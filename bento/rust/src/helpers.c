#include <linux/buffer_head.h>
#include <linux/slab.h>
#include <linux/delay.h>
#include <linux/fs.h>
#include <linux/backing-dev.h>
#include <linux/module.h>

//static int test_bdev_super(struct super_block *s, void *data)
//{
//	return (void *)s->s_bdev == data;
//}
//
//static int set_bdev_super(struct super_block *s, void *data)
//{
//	return 0;
//}

void put_filesystem(struct file_system_type *fs)
{
	module_put(fs->owner);
}

struct block_device *
get_bdev_helper(const char* dev_name, fmode_t mode, bool blk) {
	struct block_device *bdev;
	struct file_system_type *fs_type;
	if (blk)
		fs_type = get_fs_type("bentoblk");
	else
		fs_type = get_fs_type("bento");
	bdev = blkdev_get_by_path(dev_name, mode, fs_type);
	blkdev_put(bdev, mode); //FMODE_READ | FMODE_EXCL | FMODE_WRITE);
	put_filesystem(fs_type);
	return bdev;
}

long mount(void) {
	long ret = 0;
	//const char* dev = "/dev/nvme0n1";
	//const char __user * dir = "/mnt/xv6fsll";
	//const char* type = "bentoblk";
	//unsigned long flags = 0;
	//const char* options = "fd=10,rootmode=40000,user_id=0,group_id=0,blksize=4096,name=xv6fs_ll";
	//ret = do_mount(dev, dir, type, flags, (void *) options);
	//ret = do_mount((const char*) 0, (const char __user *) 0, (const char *) 0, 0, (const char *) 0);
	return ret;
}

//static int mount() {
//	struct file_system_type *type;
//	struct vfsmount *mnt;
//	int err;
//	int mnt_flags = 0;
//	struct path path;
//	int sb_flags = (SB_RDONLY |
//		SB_SYNCHRONOUS |
//		SB_MANDLOCK |
//		SB_DIRSYNC |
//		SB_SILENT |
//		SB_POSIXACL |
//		SB_LAZYTIME |
//		SB_I_VERSION);
//
//	type = get_fs_type("bentoblk");
//	if (!type)
//		return -ENODEV;
//
//	mnt = vfs_kern_mount(type, sb_flags, "/dev/nvme0n1", "fd=10,rootmode=40000,user_id=0,group_id=0,blksize=4096,name=xv6fs_ll");
//
//	put_filesystem(type);
//	if (IS_ERR(mnt))
//		return PTR_ERR(mnt);
//
//	err = kern_path("/mnt/xv6fsll", LOOKUP_FOLLOW, &path);
//	err = do_add_mount(real_mount(mnt), &path, mnt_flags);
//	if (err)
//		mntput(mnt);
//	return err;
//}

void
rs_dump_super_block(struct super_block* sb) {
    printk(KERN_INFO "s_blocksize = %lx\n", sb->s_blocksize);
    printk(KERN_INFO "s_blocksize_bits = %x\n", sb->s_blocksize_bits);
    printk(KERN_INFO "s_maxbytes = %llx\n", sb->s_maxbytes);
    printk(KERN_INFO "s_magic = %lx\n", sb->s_magic);
    printk(KERN_INFO "s_flags = %lx\n", sb->s_flags);
    printk(KERN_INFO "s_op = %p\n", sb->s_op);
    printk(KERN_INFO "s_dev = %x\n", sb->s_dev);
    printk(KERN_INFO "s_bdev = %p\n", sb->s_bdev);
    printk(KERN_INFO "s_mtd = %p\n", sb->s_mtd);
    printk(KERN_INFO "s_fs_info = %p\n", sb->s_fs_info);
}

struct buffer_head *
rs_sb_bread(void *ptr, sector_t block)
{
    struct super_block *sb = (struct super_block *)ptr;
    return __bread_gfp(sb->s_bdev, block, sb->s_blocksize, __GFP_MOVABLE);
}

struct buffer_head *
bread_helper(void *ptr, sector_t block, unsigned size)
{
	struct block_device *bdev = (struct block_device *)ptr;
	return __bread_gfp(bdev, block, size, __GFP_MOVABLE);
}

struct block_device*
rs_super_block_get_s_bdev(struct super_block *sb)
{
    return sb->s_bdev;
}

void*
rs_buffer_head_get_b_data(void* bh) {
    struct buffer_head* buffer_head = (struct buffer_head*) bh;
    return (void*) buffer_head->b_data;
}

size_t
rs_buffer_head_get_b_size(void* bh) {
    struct buffer_head* buffer_head = (struct buffer_head*) bh;
    return buffer_head->b_size;
}

struct wait_queue_head* rs_get_wait_queue_head(void) {
	struct wait_queue_head* wq_head = kmalloc(sizeof(struct wait_queue_head), GFP_KERNEL);
	init_waitqueue_head(wq_head);
	return wq_head;
}

void rs_put_wait_queue_head(struct wait_queue_head* wq_head) {
	kfree(wq_head);
}

void rs_wake_up(struct wait_queue_head* wq_head) {
    wake_up(wq_head);
}

void rs_wait_event(struct wait_queue_head* wq_head, bool (condition)(void)) {
    wait_event(*wq_head, condition());
}

struct rw_semaphore* rs_get_semaphore(void) {
	struct rw_semaphore* sem = kmalloc(sizeof(struct rw_semaphore), GFP_KERNEL);
	init_rwsem(sem);
	return sem;
}

void rs_put_semaphore(struct rw_semaphore *sem) {
	kfree(sem);
}

void rs_ndelay(unsigned long x) {
    ndelay(x);
}
