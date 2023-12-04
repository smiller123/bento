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
//#include "../../../jbd2/jbd2.h"
#include <linux/namei.h>
#include <linux/path.h>
#include <linux/mount.h>
#include <linux/in.h>
#include <linux/net.h>
#include <linux/kthread.h>
#include <linux/timekeeping.h>
#include <net/sock.h>
#include <net/xfrm.h>
#include <linux/siphash.h>
#include <linux/sockptr.h>
#include <linux/ghost.h>
#include <linux/smp.h>
#include <linux/cpumask.h>
//#include <uapi/linux/ghost.h>

static siphash_key_t rs_net_secret __read_mostly;

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
current_net(void) {
	return current->nsproxy->net_ns;
}

int
kernel_bind(struct socket *sock, struct sockaddr *uaddr, int addr_len) {
	return sock->ops->bind(sock, uaddr, addr_len);
}

int
kernel_listen(struct socket *sock, int backlog) {
	return sock->ops->listen(sock, backlog);
}

int
kernel_getsockopt(struct socket *sock, int level, int optname,
			   char *optval, int *optlen) {
	return sock->ops->getsockopt(sock, level, optname, optval, *optlen);
}

int
kernel_setsockopt(struct socket *sock, int level, int optname,
			   char *optval, int *optlen) {
	return sock->ops->setsockopt(sock, level, optname, KERNEL_SOCKPTR(optval), optlen);
}

int
kernel_getname(struct socket *sock, struct sockaddr *uaddr,
		 int peer) {
	return sock->ops->getname(sock, uaddr, peer);
}

unsigned int
current_flags(void) {
	return current->flags;
}

unsigned int
current_pid(void) {
	return current->pid;
	//return pid_nr(get_task_pid(current, PIDTYPE_PID));
}

struct task_struct *rs_current(void) {
	return current;
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

rwlock_t* rs_get_rwlock(void) {
	rwlock_t* lock = kmalloc(sizeof(rwlock_t), GFP_KERNEL);
	rwlock_init(lock);
	return lock;
}

void rs_put_semaphore(struct rw_semaphore *sem) {
	kfree(sem);
}

void rs_put_rwlock(rwlock_t *lock) {
	kfree(lock);
}

void rs_read_lock(rwlock_t *lock) {
	read_lock(lock);
}

void rs_read_unlock(rwlock_t *lock) {
	read_unlock(lock);
}

void rs_write_lock(rwlock_t *lock) {
	write_lock(lock);
}

void rs_write_unlock(rwlock_t *lock) {
	write_unlock(lock);
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
    journal->j_max_transaction_buffers = jbd2_journal_get_max_txn_bufs(journal);

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

struct timespec64 current_kernel_time_rs(void)
{
	struct timespec64 ts;
	ktime_get_real_ts64(&ts);
	return ts;
}

void rs_jbd2_journal_setup(journal_t *journal) {
	jbd2_journal_clear_features(journal,
                        JBD2_FEATURE_COMPAT_CHECKSUM, 0,
                        JBD2_FEATURE_INCOMPAT_CSUM_V3 |
                        JBD2_FEATURE_INCOMPAT_CSUM_V2);
	jbd2_journal_set_features(journal,
                        0, 0,
                        JBD2_FEATURE_INCOMPAT_CSUM_V3);
        jbd2_journal_clear_features(journal, 0, 0,
                        JBD2_FEATURE_INCOMPAT_ASYNC_COMMIT);
}

void rs_sk_mem_reclaim(struct sock *sk) {
	sk_mem_reclaim(sk);
}

void rs_release_dst_cache(struct sock *sk) {
	dst_release(rcu_dereference_protected(sk->sk_dst_cache, 1));
}

void rs_sk_refcnt_debug_inc(struct sock *sk) {
	sk_refcnt_debug_inc(sk);
}

void rs_sk_refcnt_debug_dec(struct sock *sk) {
	sk_refcnt_debug_dec(sk);
}

void rs_sk_refcnt_debug_release(struct sock *sk) {
	sk_refcnt_debug_release(sk);
}

bool rs_sock_flag(const struct sock *sk, enum sock_flags flag) {
	return sock_flag(sk, flag);
}

void rs_sock_set_flag(struct sock *sk, enum sock_flags flag) {
	return sock_set_flag(sk, flag);
}

void rs_sock_reset_flag(struct sock *sk, enum sock_flags flag) {
	return sock_reset_flag(sk, flag);
}

void rs_sock_hold(struct sock *sk) {
	sock_hold(sk);
}

void rs_sock_orphan(struct sock *sk) {
	sock_orphan(sk);
}

void rs_sock_put(struct sock *sk) {
	sock_put(sk);
}

void rs_local_bh_enable(void) {
	local_bh_enable();
}

void rs_local_bh_disable(void) {
	local_bh_disable();
}

//void rs_bh_lock_sock(struct sock *sk) {
//	bh_lock_sock(sk);
//}
//
//void rs_bh_lock_sock_nested(struct sock *sk) {
//	bh_lock_sock_nested(sk);
//}
//
//void rs_bh_unlock_sock(struct sock *sk) {
//	bh_unlock_sock(sk);
//}

void rs_inc_orphan(struct sock *sk) {
	percpu_counter_inc(sk->sk_prot->orphan_count);
}

void rs_dec_orphan(struct sock *sk) {
	percpu_counter_dec(sk->sk_prot->orphan_count);
}

void rs_xfrm_sk_free_policy(struct sock *sk) {
	xfrm_sk_free_policy(sk);
}

void rs_sock_alloc_dec(struct sock *sk) {
	percpu_counter_dec(sk->sk_prot->sockets_allocated);
}

unsigned short rs_ntohs(__be16 in) {
	return ntohs(in);
}

__be16 rs_htons(unsigned short in) {
	return htons(in);
}

__be32 rs_htonl(unsigned long in) {
	return htonl(in);
}

unsigned long rs_ntohl(__be32 in) {
	return ntohl(in);
}

bool rs_inet_port_requires_bind_service(struct net *net, unsigned short port) {
	return inet_port_requires_bind_service(net, port);
}

struct net *rs_sock_net(const struct sock *sk) {
	return sock_net(sk);
}

void rs_lock_sock(struct sock *sk) {
	return lock_sock(sk);
}

void rs_sk_dst_reset(struct sock *sk) {
	return sk_dst_reset(sk);
}

void rs_reqsk_queue_alloc(struct request_sock_queue *queue)
{
	spin_lock_init(&queue->rskq_lock);

	spin_lock_init(&queue->fastopenq.lock);
	queue->fastopenq.rskq_rst_head = NULL;
	queue->fastopenq.rskq_rst_tail = NULL;
	queue->fastopenq.qlen = 0;

	queue->rskq_accept_head = NULL;
}

void rs_inet_csk_delack_init(struct sock *sk) {
	inet_csk_delack_init(sk);
}

void rs_smp_store_release(char *p, char v) {
	smp_store_release(p, v);
}

void rs_sock_prot_inuse_add(struct net *net, struct proto *prot,
		int inc)
{
	sock_prot_inuse_add(net, prot, inc);
}

int rs_sock_prot_inuse_get(struct net *net, struct proto *prot)
{
	return sock_prot_inuse_get(net, prot);
}

void rs_sock_graft(struct sock *sk, struct socket *parent) {
	return sock_graft(sk, parent);
}

bool rs_reqsk_queue_empty(const struct request_sock_queue *queue) {
	return reqsk_queue_empty(queue);
}

long rs_sock_rcvtimeo(const struct sock *sk, bool noblock) {
	return sock_rcvtimeo(sk, noblock);
}

long rs_sock_sndtimeo(const struct sock *sk, bool noblock) {
	return sock_sndtimeo(sk, noblock);
}

struct wait_queue_entry rs_define_wait(void) {
	DEFINE_WAIT(wait);
	return wait;
}

struct wait_queue_entry rs_define_wait_func(wait_queue_func_t func) {
	DEFINE_WAIT_FUNC(wait, func);
	return wait;
}

wait_queue_head_t *rs_sk_sleep(struct sock *sk) {
	return sk_sleep(sk);
}

void rs_sched_annotate_sleep(void) {
	sched_annotate_sleep();
}

int rs_sock_intr_errno(long timeo) {
	return sock_intr_errno(timeo);
}

int rs_signal_pending(void) {
	return signal_pending(current);
}

int rs_sock_error(struct sock *sk) {
	return sock_error(sk);
}

struct ip_options_rcu *rs_get_inet_opt(struct inet_sock *inet, struct sock *sk) {
	return rcu_dereference_protected(inet->inet_opt, lockdep_sock_is_held(sk));
}

int rs_sk_mem_pages(int amt) {
	return sk_mem_pages(amt);
}

long rs_sk_memory_allocated_add(struct sock *sk, int amt) {
	return sk_memory_allocated_add(sk, amt);
}

bool rs_sk_wmem_schedule(struct sock *sk, int size) {
	return sk_wmem_schedule(sk, size);
}

void rs_init_list_head(struct list_head *list) {
	return INIT_LIST_HEAD(list);
}

void rs__skb_header_release(struct sk_buff *skb) {
	return __skb_header_release(skb);
}

void rs_sk_wmem_queued_add(struct sock *sk, int val) {
	return sk_wmem_queued_add(sk, val);
}

void rs_sk_mem_charge(struct sock *sk, int size) {
	return sk_mem_charge(sk, size);
}

u64 rs_ktime_get_ns(void) {
	return ktime_get_real_ns();
}

int rs_skb_cloned(const struct sk_buff *skb) {
	return skb_cloned(skb);
}

struct sk_buff *rs_pskb_copy(struct sk_buff *skb,
					gfp_t gfp_mask) {
	return pskb_copy(skb, gfp_mask);
}

void rs_skb_orphan(struct sk_buff *skb) {
	skb_orphan(skb);
}

bool rs_refcount_sub_and_test(int i, refcount_t *r) {
	return refcount_sub_and_test(i, r);
}

void rs_refcount_add(int i, refcount_t *r) {
	refcount_add(i, r);
}

__sum16 rs_csum_tcpudp_magic(__be32 saddr, __be32 daddr,
			  __u32 len, __u8 proto, __wsum sum) {
	return csum_tcpudp_magic(saddr, daddr, len, proto, sum);
}

//void rs_rcu_read_lock_bh(void) {
//	rcu_read_lock_bh();
//}
//
//void rs_rcu_read_unlock_bh(void) {
//	rcu_read_unlock_bh();
//}

__be16 rs_cpu_to_be16(short i) {
	return cpu_to_be16(i);
}

int rs_skb_orphan_frags_rx(struct sk_buff *skb, gfp_t gfp_mask) {
	return skb_orphan_frags_rx(skb, gfp_mask);
}

int rs_gfp_atomic(void) {
	return GFP_ATOMIC;
}

struct ubuf_info *rs_skb_zcopy(struct sk_buff *skb) {
	return skb_zcopy(skb);
}

void rs_check_skb(struct sk_buff *skb) {
	printk(KERN_INFO "pf memalloc %d\n", skb->pfmemalloc);
	printk(KERN_INFO "zcopy %d\n", skb_zcopy(skb));
	printk(KERN_INFO "protocol %x\n", skb->protocol);
}

int rs_dev_hard_header(struct sk_buff *skb, struct net_device *dev,
				  unsigned short type,
				  const void *daddr, const void *saddr,
				  unsigned int len) {
	return dev_hard_header(skb, dev, type, daddr, saddr, len);
}

static __always_inline void rs_net_secret_init(void)
{
	net_get_random_once(&rs_net_secret, sizeof(rs_net_secret));
}

static u32 rs_seq_scale(u32 seq)
{
	/*
	 *	As close as possible to RFC 793, which
	 *	suggests using a 250 kHz clock.
	 *	Further reading shows this assumes 2 Mb/s networks.
	 *	For 10 Mb/s Ethernet, a 1 MHz clock is appropriate.
	 *	For 10 Gb/s Ethernet, a 1 GHz clock should be ok, but
	 *	we also need to limit the resolution so that the u32 seq
	 *	overlaps less than one time per MSL (2 minutes).
	 *	Choosing a clock of 64 ns period is OK. (period of 274 s)
	 */
	struct timespec64 ts;
	ktime_get_real_ts64(&ts);
	return seq + (timespec64_to_ktime(ts) >> 6);
}

u32 rs_secure_tcp_seq(__be32 saddr, __be32 daddr,
		   __be16 sport, __be16 dport) {
	u32 hash;
	rs_net_secret_init();
	hash = siphash_3u32((__force u32)saddr, (__force u32)daddr,
			    (__force u32)sport << 16 | (__force u32)dport,
			    &rs_net_secret);
	return rs_seq_scale(hash);
}

struct sk_buff *rs_skb_share_check(struct sk_buff *skb) {
	return skb_share_check(skb, GFP_ATOMIC);
}

bool rs_pskb_may_pull(struct sk_buff *skb, unsigned int len) {
	return pskb_may_pull(skb, len);
}

__sum16 rs_ip_fast_csum(const void *iph, unsigned int ihl) {
	return ip_fast_csum(iph, ihl);
}

int rs_pskb_trim_rcsum(struct sk_buff *skb, unsigned int len) {
	return pskb_trim_rcsum(skb, len);
}

struct net *rs_dev_net(const struct net_device *dev) {
	return dev_net(dev);
}

void *rs___skb_pull(struct sk_buff *skb, unsigned int len) {
	return __skb_pull(skb, len);
}

//void rs_rcu_read_lock(void) {
//	rcu_read_lock();
//}
//
//void rs_rcu_read_unlock(void) {
//	rcu_read_unlock();
//}

__wsum rs_inet_compute_pseudo(struct sk_buff *skb, int proto)
{
	return csum_tcpudp_nofold(ip_hdr(skb)->saddr, ip_hdr(skb)->daddr,
				  skb->len, proto, 0);
}

__sum16 rs_skb_checksum_init(struct sk_buff *skb, int proto) {
	return skb_checksum_init(skb, proto, rs_inet_compute_pseudo);
}

void rs_sk_incoming_cpu_update(struct sock *sk) {
	return sk_incoming_cpu_update(sk);
}

int rs_skb_csum_unnecessary(const struct sk_buff *skb) {
	return skb_csum_unnecessary(skb);
}

struct request_sock *
rs_reqsk_alloc(const struct request_sock_ops *ops, struct sock *sk_listener,
	    bool attach_listener) {
	return reqsk_alloc(ops, sk_listener, attach_listener);
}

void rs_skb_set_owner_w(struct sk_buff *skb, struct sock *sk)
{
	int alloc_offset = offsetof(struct sock, sk_wmem_alloc);
	int rcv_nxt_off = offsetof(struct tcp_request_sock, rcv_nxt);
	printk(KERN_INFO "alloc offset %d\n", alloc_offset);
	printk(KERN_INFO "last tcp_request offset %d\n", rcv_nxt_off);
	skb_orphan(skb);
	skb->sk = sk;
#ifdef CONFIG_INET
	printk(KERN_INFO "yes config inet\n");
	if (unlikely(!sk_fullsock(sk))) {
		printk(KERN_INFO "not full sock\n");
		skb->destructor = sock_edemux;
		sock_hold(sk);
		return;
	}
#endif
	skb->destructor = sock_wfree;
	skb_set_hash_from_sk(skb, sk);
}

void rs_refcount_set(refcount_t *r, int n) {
	refcount_set(r, n);
}

int rs_net_xmit_eval(int e) {
	return net_xmit_eval(e);
}

void rs_write_pnet(possible_net_t *pnet, struct net *net) {
	return write_pnet(pnet, net);
}

struct net *rs_read_pnet(possible_net_t *pnet) {
	return read_pnet(pnet);
}

void rs_reqsk_put(struct request_sock *req) {
	return reqsk_put(req);
}

void rs__skb_queue_tail(struct sk_buff_head *list,
				   struct sk_buff *newsk) {
	return __skb_queue_tail(list, newsk);
}

struct sk_buff *rs_skb_peek(struct sk_buff_head *list) {
	return skb_peek(list);
}

void rs__skb_unlink(struct sk_buff *skb, struct sk_buff_head *list)
{
	return __skb_unlink(skb, list);
}

void *rs__skb_pull(struct sk_buff *skb, unsigned int len) {
	return __skb_pull(skb, len);
}

void timer_func(struct timer_list *timer) {
}

void rs_timer_setup(struct timer_list *timer,
		    void (*func)(struct timer_list *), unsigned int flags) {
	timer_setup(timer, func, flags);
}

u64 rs_get_jiffies_64(void) {
	return get_jiffies_64();
}

void rs_req_prot_cleanup(struct request_sock_ops *rsk_prot)
{
	if (!rsk_prot)
		return;
	kfree(rsk_prot->slab_name);
	rsk_prot->slab_name = NULL;
	kmem_cache_destroy(rsk_prot->slab);
	rsk_prot->slab = NULL;
}

void rs_proto_unregister_mod(struct proto *prot) {
	//mutex_lock(&proto_list_mutex);
	//release_proto_idx(prot);
	list_del(&prot->node);
	//mutex_unlock(&proto_list_mutex);

	kmem_cache_destroy(prot->slab);
	prot->slab = NULL;

	rs_req_prot_cleanup(prot->rsk_prot);
	if (prot->twsk_prot != NULL && prot->twsk_prot->twsk_slab != NULL) {
		kmem_cache_destroy(prot->twsk_prot->twsk_slab);
		kfree(prot->twsk_prot->twsk_slab_name);
		prot->twsk_prot->twsk_slab = NULL;
	}
}

void mod_print_stats(struct module *module) {
	printk(KERN_INFO "does mod have exit? %p", module->exit);
}

int rs_kern_path(const char *name, unsigned int flags, struct path *path) {
	return kern_path(name, flags, path);
}

const struct cred *rs_current_cred(void) {
	return current_cred();
}

struct vfsmount *rs_clone_private_mount(const struct path *path) {
	return clone_private_mount(path);
}

struct super_block *rs_vfsmount_get_mnt_sb(struct vfsmount *mnt) {
    return mnt->mnt_sb;
}

const char *rs_vfsmount_get_name(struct vfsmount *mnt) {
	return mnt->mnt_sb->s_type->name;
}

unsigned int rs_GHOST_IOC_CREATE_QUEUE(void) {
	return GHOST_IOC_CREATE_QUEUE;
}

struct fd rs_fdget(unsigned int fd) {
	return fdget(fd);
}

//int rs_register_ghost_agent(struct ghost_agent_type * agent) {
//	return register_ghost_agent(agent);
//}

void rs_hrtick_start(int cpu, u64 delay) {
	hrtick_start_cpu(cpu, delay);
	return;
}

int rs_smp_processor_id(void) {
	return smp_processor_id();
}

int rs_num_online_cpus(void) {
	return num_online_cpus();
}
