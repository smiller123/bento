/*
 * SPDX-License-Identifier: GPL-2.0
 * Copyright (C) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 *
 */

#include <linux/buffer_head.h>
#include <linux/slab.h>
#include <linux/delay.h>
#include <linux/fs.h>
#include <linux/backing-dev.h>
#include <linux/module.h>
#include <linux/jbd2.h>

struct block_device *
get_bdev_helper(const char* dev_name, fmode_t mode) {
	return lookup_bdev(dev_name, mode);
}

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

dev_t
rs_block_device_get_bd_dev(struct block_device *bdev)
{
	return bdev->bd_dev;
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

void rs_wake_up_all(struct wait_queue_head* wq_head) {
    wake_up_all(wq_head);
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

// TODO journal
journal_t* rs_jbd2_journal_init_dev(struct block_device *bdev, 
                                    struct block_device *fs_dev, 
                                    unsigned long long start, 
                                    int len, 
                                    int bsize) {
    return jbd2_journal_init_dev(bdev, fs_dev, start, len, bsize);
}

handle_t *rs_jbd2_journal_start(journal_t * journal, int nblocks) {
    return jbd2_journal_start(journal, nblocks);
}

int rs_jbd2_journal_stop(handle_t * handle) {
    return jbd2_journal_stop(handle);
}

int rs_jbd2_journal_get_write_access(handle_t * handle, struct buffer_head * bh) {
    return jbd2_journal_get_write_access(handle, bh);
}
