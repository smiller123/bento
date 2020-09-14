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
#include <linux/namei.h>
#include <linux/path.h>
#include <linux/mount.h>
#include <linux/in.h>
#include <linux/net.h>
#include <linux/kthread.h>

void
wait_a_bit(void) {
	set_current_state(TASK_INTERRUPTIBLE);
	cond_resched();
	//schedule();
}

void
wait_for_interrupt(void) {
	set_current_state(TASK_INTERRUPTIBLE);
	schedule();
}

struct task_struct *
kthread_run_helper(int (*threadfn)(void *data), void *data, const char *namefmt){
	return kthread_run(threadfn, data, namefmt);
}

struct net *
current_net() {
	return current->nsproxy->net_ns;
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
rs_getblk(struct block_device *bdev, sector_t block, unsigned size)
{
	return __getblk_gfp(bdev, block, size, __GFP_MOVABLE);
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

size_t
rs_buffer_head_get_b_blocknr(void* bh) {
    struct buffer_head* buffer_head = (struct buffer_head*) bh;
    return buffer_head->b_blocknr;
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

int journal_get_superblock(journal_t *journal);

void rs_set_buffer_uptodate(struct buffer_head *bh)
{
        set_buffer_uptodate(bh);
}

void rs_lock_buffer(struct buffer_head *bh)
{
	might_sleep();
	if (!trylock_buffer(bh))
		__lock_buffer(bh);
}

// TODO journal
journal_t* rs_jbd2_journal_init_dev(struct block_device *bdev, 
                                    struct block_device *fs_dev, 
                                    unsigned long long start, 
                                    int len, 
                                    int bsize) {
    journal_t *journal = jbd2_journal_init_dev(bdev, fs_dev, start, len, bsize);
    journal->j_max_transaction_buffers = journal->j_maxlen / 4;

    return journal; 
}

int rs_jbd2_journal_load(journal_t *journal) {
    return jbd2_journal_load(journal);
}

int rs_jbd2_journal_destroy(journal_t *journal) {
    return jbd2_journal_destroy(journal);
}

handle_t *rs_jbd2_journal_start(journal_t * journal, int nblocks) {
    /*printk(KERN_INFO "begin_op\n\tbarrier_count %u\n\tmax_len %u\n\tmax_transaction_buffers %u\n\tcommit interval %u",
                                                journal->j_barrier_count,
                                                journal->j_maxlen,
                                                journal->j_max_transaction_buffers,
						journal->j_commit_interval);*/
    return jbd2_journal_start(journal, nblocks);
}

int rs_jbd2_journal_stop(handle_t * handle) {
    return jbd2_journal_stop(handle);
}

int rs_jbd2_journal_get_write_access(handle_t * handle, struct buffer_head * bh) {
    return jbd2_journal_get_write_access(handle, bh);
}

int rs_jbd2_journal_get_create_access(handle_t * handle, struct buffer_head * bh) {
    return jbd2_journal_get_create_access(handle, bh);
}

int rs_jbd2_journal_dirty_metadata (handle_t *handle, struct buffer_head *bh) {
    return jbd2_journal_dirty_metadata(handle, bh);
}

int rs_jbd2_journal_force_commit(journal_t *journal) {
    return jbd2_journal_force_commit(journal);
}

void rs_jbd2_journal_set_barrier(journal_t *journal) {
	journal->j_flags |= JBD2_BARRIER;
	jbd2_journal_set_features(journal, 0, 0,
                                       JBD2_FEATURE_INCOMPAT_64BIT);
}

void rs_jbd2_journal_set_async_commit(journal_t *journal) {
	jbd2_journal_clear_features(journal,
			JBD2_FEATURE_COMPAT_CHECKSUM, 0,
			JBD2_FEATURE_INCOMPAT_CSUM_V3 |
			JBD2_FEATURE_INCOMPAT_CSUM_V2);
	jbd2_journal_clear_features(journal, 0,
			0, JBD2_FEATURE_INCOMPAT_ASYNC_COMMIT);
}
