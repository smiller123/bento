use core::cmp::min;
use core::mem;
use core::str;
use core::sync::atomic::{AtomicUsize, Ordering};

use arr_macro::arr;

use bento::bindings::*;
use bento::kernel;

use kernel::errno::*;
use kernel::fs::*;
use kernel::kobj::*;
use kernel::mem as kmem;
use kernel::semaphore::*;
use kernel::stat;
use kernel::string::*;

use crate::log::*;
use crate::xv6fs_file::*;
use crate::xv6fs_utils::*;

use bento::println;
use bento::DataBlock;

pub static DISK: Semaphore<Option<Disk>> = Semaphore::new(None);

// 'SB': the in-memory data structure for the xv6 superblock, with semaphore support.
pub static SB: Semaphore<Xv6fsSB> = Semaphore::new(Xv6fsSB {
    size: 0,
    nblocks: 0,
    ninodes: 0,
    nlog: 0,
    logstart: 0,
    inodestart: 0,
    bmapstart: 0,
});

static LAST_BLOCK: AtomicUsize = AtomicUsize::new(0);

// Read xv6 superblock from disk
fn readsb(xv6fs_sb: &mut Xv6fsSB) -> Result<(), Error> {
    let disk_guard = DISK.read();
    if let Some(disk) = &*disk_guard {
        let bh = disk.bread(1)?;
        let b_slice = bh.data();
        xv6fs_sb.extract_from(&b_slice[0..mem::size_of::<Xv6fsSB>()]).map_err(|_| Error::EIO)?;
    }
    return Ok(());
}

pub fn bzero(bno: usize) -> Result<(), Error> {
    let disk_guard = DISK.read();
    let disk = disk_guard.as_ref().unwrap();
    let mut bh = disk.bread(bno as u64)?;

    let b_slice = bh.data_mut();
    for byte in b_slice {
        *byte = 0;
    }

    bh.mark_buffer_dirty();
    log_write(bno as u32);

    return Ok(());
}

// Allocate a block on disk, using a slightly different alloc strategy from xv6.
// xv6 scans from 0th block and allocates the first available block, we scan from the latest used block since last boot.
pub fn balloc() -> Result<u32, Error> {
    let fs_size = SB.read().size;
    let mut allocated_block = None;

    // Bitmap operations on bitmap blocks
    let most_recent = LAST_BLOCK.load(Ordering::SeqCst);
    let mut first = true;
    // last_segment is the bitmap block ID and block_offset is the offset for 'most_recent'
    let last_segment = most_recent - most_recent % BPB;
    let mut block_offset = most_recent % BPB;

    let mut b = last_segment;

    while first || b < last_segment {
        let disk_guard = DISK.read();
        let disk = disk_guard.as_ref().unwrap();
        // Read bitmap block that contains bitmap for b/last_segment, bitmap_slice contains the data
        let mut bh = disk.bread(bblock(b as usize, &SB.read()) as u64)?;
        let bitmap_slice = bh.data_mut();

        let mut changed = false;

        // last allocated was block_offset, scan from it until end of block.
        for bi in block_offset..BPB {
            let _guard = BALLOC_LOCK.write();
            let curr_data_block = b as u32 + bi as u32; // 'b' is block id and 'bi' is offset
            if curr_data_block >= fs_size {
                return Err(Error::EIO);
            }

            let m = 1 << (bi % 8);
            let byte_data = bitmap_slice.get_mut(bi / 8).ok_or(Error::EIO)?;

            if *byte_data & m == 0 {
                // found unallocated, bitmap bit is zero.
                *byte_data |= m;
                changed = true;
                allocated_block = Some(curr_data_block);
                break;
            }
        }

        // Write buffer
        if changed {
            bh.mark_buffer_dirty();
            log_write(bblock(b as usize, &SB.read()) as u32);
        }
        // extract new block ID x
        if let Some(x) = allocated_block {
            LAST_BLOCK.store(x as usize, Ordering::SeqCst);
            bzero(x as usize)?;
            return Ok(x);
        }

        // did not find usable block in this bitmap block, go to the next bitmap block
        block_offset = 0;
        b += BPB;
        if b >= fs_size as usize {
            b = 0;
            first = false;
        }
    }
    return Err(Error::EIO);
}

pub fn bfree(block_id: usize) -> Result<(), Error> {
    // Get block number
    let block_num = bblock(block_id, &SB.read());

    // Read block
    let disk_guard = DISK.read();
    let disk = disk_guard.as_ref().unwrap();
    let mut bh = disk.bread(block_num as u64)?;
    let b_slice = bh.data_mut();

    // Get bit id
    let bit_id = block_id % BPB;
    let byte_id = bit_id / 8;
    let bit_in_byte = bit_id % 8;

    // Clear the bit
    let maybe_mut_byte = b_slice.get_mut(byte_id);
    let mut_byte = maybe_mut_byte.ok_or(Error::EIO)?;

    *mut_byte &= !(1 << bit_in_byte);

    // Write buffer
    bh.mark_buffer_dirty();
    log_write(block_num as u32);

    return Ok(());
}

pub static ILOCK_CACHE: Semaphore<[Semaphore<Inode>; NINODE]> =
    Semaphore::new(arr![Semaphore::new(Inode::new()); 300]);

pub static IALLOC_LOCK: Semaphore<usize> = Semaphore::new(0);
pub static BALLOC_LOCK: Semaphore<usize> = Semaphore::new(0);

pub fn iinit() {
    SB.init();
    let mut sb_ref = SB.write();
    if readsb(&mut sb_ref).is_err() {
        println!("Unable to read super block from disk.");
    }
    let _ = initlog(&mut sb_ref);
    println!(
        "sb: size {}, nblocks {}, ninodes {}, nlog {}, logstart {} inodestart {}, bmap start {}",
        sb_ref.size,
        sb_ref.nblocks,
        sb_ref.ninodes,
        sb_ref.nlog,
        sb_ref.logstart,
        sb_ref.inodestart,
        sb_ref.bmapstart
    );
    ILOCK_CACHE.init();
    IALLOC_LOCK.init();
    BALLOC_LOCK.init();
    for inode in ILOCK_CACHE.write().iter_mut() {
        inode.init();
        inode.write().internals.init();
    }
}

pub fn ialloc(i_type: u16) -> Result<CachedInode, Error> {
    let num_inodes = SB.read().ninodes;
    for block_inum in (0..num_inodes as usize).step_by(IPB) {
        let _guard = IALLOC_LOCK.write();
        let disk_guard = DISK.read();
        let disk = disk_guard.as_ref().unwrap();
        let mut bh = disk.bread(iblock(block_inum, &SB.read()) as u64)?;
        let data_slice = bh.data_mut();
        for inum_idx in (block_inum % IPB)..IPB {
            let inum = block_inum + inum_idx;
            if inum == 0 {
                continue;
            }
            // Get the specific inode offset
            let inode_offset = (inum as usize % IPB) * mem::size_of::<Xv6fsInode>();

            let inode_slice =
                &mut data_slice[inode_offset..inode_offset + mem::size_of::<Xv6fsInode>()];

            let mut dinode = Xv6fsInode::new();
            dinode.extract_from(inode_slice).map_err(|_| Error::EIO)?;
            let mut allocated = false;
            // Check if inode is free
            if dinode.inode_type == 0 {
                dinode.major = 0;
                dinode.minor = 0;
                dinode.size = 0;
                for addr_mut in dinode.addrs.iter_mut() {
                    *addr_mut = 0;
                }
                dinode.inode_type = i_type;
                dinode.nlink = 1;
                dinode.dump_into(inode_slice).map_err(|_| Error::EIO)?;
                allocated = true;
            }

            if allocated {
                bh.mark_buffer_dirty();
                log_write(iblock(block_inum, &SB.read()) as u32);
                return iget(inum as u64);
            }
        }
    }
    return Err(Error::EIO);
}

pub fn iupdate(internals: &InodeInternal, inum: u32) -> Result<(), Error> {
    let disk_guard = DISK.read();
    let disk = disk_guard.as_ref().unwrap();
    let mut bh = disk.bread(iblock(inum as usize, &SB.read()) as u64)?;
    let data_slice = bh.data_mut();

    // Get the specific inode offset
    let inode_offset = (inum as usize % IPB) * mem::size_of::<Xv6fsInode>();
    let inode_slice = &mut data_slice[inode_offset..inode_offset + mem::size_of::<Xv6fsInode>()];

    let mut disk_inode = Xv6fsInode::new();
    disk_inode
        .extract_from(inode_slice)
        .map_err(|_| Error::EIO)?;
    disk_inode.inode_type = internals.inode_type;
    disk_inode.major = internals.major;
    disk_inode.minor = internals.minor;
    disk_inode.nlink = internals.nlink;
    disk_inode.size = internals.size;
    disk_inode.addrs.copy_from_slice(&internals.addrs);
    disk_inode.dump_into(inode_slice).map_err(|_| Error::EIO)?;

    bh.mark_buffer_dirty();
    log_write(iblock(inum as usize, &SB.read()) as u32);
    return Ok(());
}

pub fn iget(inum: u64) -> Result<CachedInode, Error> {
    let mut final_idx = None;

    let ilock_cache = ILOCK_CACHE.read();
    for (idx, inode_lock) in ilock_cache.iter().enumerate() {
        let inode_opt = inode_lock.try_write();
        if inode_opt.is_none() {
            continue;
        }
        let mut inode = inode_opt.ok_or(Error::EIO)?;
        // TODO: get dev id
        if inode.nref > 0 && inode.dev == 3 as u32 && inode.inum == inum as u32 {
            inode.nref += 1;

            return Ok(CachedInode {
                idx: idx,
                inum: inum as u32,
            });
        }
        if final_idx.is_none() && inode.nref == 0 {
            {
                let mut new_inode_int = inode.internals.write();
                new_inode_int.valid = 0;
            }
            inode.dev = 3 as u32;
            inode.inum = inum as u32;
            inode.nref = 1;
            final_idx = Some(idx);
        }
    }

    let new_inode_idx = final_idx.ok_or(Error::EIO)?;

    let ret = Ok(CachedInode {
        idx: new_inode_idx,
        inum: inum as u32,
    });
    return ret;
}

pub fn ilock(
    inode_idx: usize,
    icache: &[Semaphore<Inode>; 300],
    inum: u32,
) -> Result<SemaphoreReadGuard<Inode>, Error> {
    let inode_outer_lock = icache.get(inode_idx).ok_or(Error::EIO)?;
    let inode_outer = inode_outer_lock.read();
    {
        let mut internals = inode_outer.internals.write();

        if internals.valid == 0 {
            let disk_guard = DISK.read();
            if let Some(disk) = &*disk_guard {
                let bh = disk.bread(iblock(inum as usize, &SB.read()) as u64)?;
                let data_slice = bh.data();

                // Get the specific inode offset
                let inode_offset = (inum as usize % IPB) * mem::size_of::<Xv6fsInode>();

                let inode_slice =
                    &data_slice[inode_offset..inode_offset + mem::size_of::<Xv6fsInode>()];
                let mut disk_inode = Xv6fsInode::new();
                disk_inode
                    .extract_from(inode_slice)
                    .map_err(|_| Error::EIO)?;

                internals.valid = 0;
                internals.inode_type = disk_inode.inode_type;
                internals.major = disk_inode.major;
                internals.minor = disk_inode.minor;
                internals.nlink = disk_inode.nlink;
                internals.size = disk_inode.size;
                internals.addrs.copy_from_slice(&disk_inode.addrs);
                internals.valid = 1;
                if internals.inode_type == 0 {
                    return Err(Error::EIO);
                }
            }
        }
    }
    return Ok(inode_outer);
}

pub fn iput<'a>(inode: &mut CachedInode) -> Result<(), Error> {
    let ilock_cache = ILOCK_CACHE.read();
    {
        let inode_guard = ilock(inode.idx, &ilock_cache, inode.inum)?;
        let mut internals = inode_guard.internals.write();
        if internals.valid != 0 && internals.nlink == 0 {
            let r;
            {
                let dinode_lock = ilock_cache.get(inode.idx).ok_or(Error::EIO)?;
                let dinode = dinode_lock.read();
                r = dinode.nref;
            }
            if r == 1 {
                itrunc(inode, &mut internals)?;
                internals.inode_type = 0;
                iupdate(&internals, inode.inum)?;
                internals.valid = 0;
            }
        }
    }

    let dinode_lock = ilock_cache.get(inode.idx).ok_or(Error::EIO)?;
    let mut dinode = dinode_lock.write();
    dinode.nref -= 1;
    return Ok(());
}

pub fn bmap(inode: &mut InodeInternal, blk_idx: usize) -> Result<u32, Error> {
    let mut idx = blk_idx;
    if idx < NDIRECT as usize {
        let addr = inode.addrs.get_mut(idx).ok_or(Error::EIO)?;
        if *addr == 0 {
            return balloc().map(|blk_id| {
                *addr = blk_id;
                blk_id
            });
        }
        return Ok(*addr);
    }

    idx -= NDIRECT as usize;
    if idx < NINDIRECT as usize {
        // indirect block
        let ind_blk_id = inode.addrs.get_mut(NDIRECT as usize).ok_or(Error::EIO)?;
        if *ind_blk_id == 0 {
            balloc().map(|blk_id| {
                *ind_blk_id = blk_id;
            })?;
        }

        let result_blk_id: u32;
        let disk_guard = DISK.read();
        let disk = disk_guard.as_ref().unwrap();
        let mut bh = disk.bread(*ind_blk_id as u64)?;
        let b_data = bh.get_buffer_data();

        let mut blks_cont = b_data.into_container::<u32>().ok_or(Error::EIO)?;
        let blks = blks_cont.to_slice_mut();
        let cell: &mut u32 = blks.get_mut(idx).ok_or(Error::EIO)?;
        if *cell == 0 {
            // need to allocate blk
            result_blk_id = balloc()?;
            *cell = result_blk_id;
            bh.mark_buffer_dirty();
            log_write(*ind_blk_id);
        } else {
            // just return the blk
            result_blk_id = *cell;
        }

        return Ok(result_blk_id);
    }

    if idx < (MAXFILE - NDIRECT) as usize {
        idx -= NINDIRECT as usize;
        // double indirect block
        let dind_blk_id = inode
            .addrs
            .get_mut(NDIRECT as usize + 1)
            .ok_or(Error::EIO)?;
        if *dind_blk_id == 0 {
            balloc().map(|blk_id| {
                *dind_blk_id = blk_id;
            })?;
        }

        let disk_guard = DISK.read();
        let disk = disk_guard.as_ref().unwrap();
        let mut bh = disk.bread(*dind_blk_id as u64)?;
        let b_data = bh.get_buffer_data();
        let dind_idx = idx / NINDIRECT as usize;

        let mut blks_cont = b_data.into_container::<u32>().ok_or(Error::EIO)?;
        let blks = blks_cont.to_slice_mut();
        let cell: &mut u32 = blks.get_mut(dind_idx).ok_or(Error::EIO)?;
        if *cell == 0 {
            *cell = balloc()?;
            bh.mark_buffer_dirty();
            log_write(*dind_blk_id);
        }

        let mut dbh = disk.bread(*cell as u64)?;
        let db_data = dbh.get_buffer_data();
        let dblock_idx = idx % NINDIRECT as usize;

        let result_blk_id: u32;
        let mut dblks_cont = db_data.into_container::<u32>().ok_or(Error::EIO)?;
        let dblks = dblks_cont.to_slice_mut();
        let dcell: &mut u32 = dblks.get_mut(dblock_idx).ok_or(Error::EIO)?;
        if *dcell == 0 {
            result_blk_id = balloc()?;
            *dcell = result_blk_id;
            dbh.mark_buffer_dirty();
            log_write(*dcell);
        } else {
            result_blk_id = *dcell;
        }
        return Ok(result_blk_id);
    }

    return Err(Error::EIO);
}

pub fn itrunc(inode: &mut CachedInode, internals: &mut InodeInternal) -> Result<(), Error> {
    for i in 0..NDIRECT as usize {
        let addr = internals.addrs.get_mut(i).ok_or(Error::EIO)?;
        if *addr != 0 {
            bfree(*addr as usize)?;
            *addr = 0;
        }
    }

    let disk_guard = DISK.read();
    let disk = disk_guard.as_ref().unwrap();
    let ind_blk_id = internals
        .addrs
        .get_mut(NDIRECT as usize)
        .ok_or(Error::EIO)?;
    if *ind_blk_id != 0 {
        let bh = disk.bread(*ind_blk_id as u64)?;
        let b_data = bh.get_buffer_data();

        let mut blks_cont = b_data.into_container::<u32>().ok_or(Error::EIO)?;
        let blks = blks_cont.to_slice_mut();
        for i in 0..NINDIRECT as usize {
            let addr = blks.get_mut(i).ok_or(Error::EIO)?;
            if *addr != 0 {
                bfree(*addr as usize)?;
            }
        }
        bfree(*ind_blk_id as usize)?;
        *ind_blk_id = 0;
    }
    let dind_blk_id = internals
        .addrs
        .get_mut(NDIRECT as usize + 1)
        .ok_or(Error::EIO)?;
    if *dind_blk_id != 0 {
        let bh = disk.bread(*dind_blk_id as u64)?;
        let b_data = bh.get_buffer_data();

        let mut blks_cont = b_data.into_container::<u32>().ok_or(Error::EIO)?;
        let blks = blks_cont.to_slice_mut();
        for i in 0..NINDIRECT as usize {
            let ind_blk_id = blks.get_mut(i).ok_or(Error::EIO)?;
            if *ind_blk_id != 0 {
                let dbh = disk.bread(*ind_blk_id as u64)?;
                let db_data = dbh.get_buffer_data();

                let mut dblks_cont = db_data.into_container::<u32>().ok_or(Error::EIO)?;
                let dblks = dblks_cont.to_slice_mut();
                for j in 0..NINDIRECT as usize {
                    let daddr = dblks.get_mut(j).ok_or(Error::EIO)?;
                    if *daddr != 0 {
                        bfree(*daddr as usize)?;
                    }
                }
                bfree(*ind_blk_id as usize)?;
                *ind_blk_id = 0;
            }
        }
        bfree(*dind_blk_id as usize)?;
        *dind_blk_id = 0;
    }

    internals.size = 0;
    return iupdate(&internals, inode.inum);
}

pub fn stati(ino: u64, stbuf: &mut fuse_attr, internals: &InodeInternal) -> Result<(), Error> {
    stbuf.ino = ino;
    if internals.inode_type == 0 {
        return Err(Error::ENOENT);
    }
    let i_type = match internals.inode_type {
        T_DIR => stat::S_IFDIR,
        T_LNK => stat::S_IFLNK | stat::S_IRWXUGO,
        _ => stat::S_IFREG,
    };
    stbuf.mode = 0o077 | i_type as u32;
    stbuf.nlink = internals.nlink as u32;
    stbuf.size = internals.size;
    // Clear remaining fields.
    stbuf.blocks = 0;
    stbuf.atime = 0;
    stbuf.mtime = 0;
    stbuf.ctime = 0;
    stbuf.atimensec = 0;
    stbuf.mtimensec = 0;
    stbuf.ctimensec = 0;
    stbuf.uid = 0;
    stbuf.gid = 0;
    stbuf.rdev = 0;
    stbuf.blksize = 0;
    return Ok(());
}

pub fn readi(
    buf: &mut [u8],
    _off: usize,
    _n: usize,
    internals: &mut InodeInternal,
) -> Result<usize, Error> {
    let mut n = _n;
    let mut off = _off;
    let i_size = internals.size as usize;
    if off > i_size || off + n < off {
        return Err(Error::EIO);
    }
    if off + n > i_size {
        n = i_size - off;
    }
    let mut m;
    let mut dst = 0;
    let mut tot = 0;

    while tot < n {
        let block_no = bmap(internals, off / BSIZE)?;
        m = min(n - tot, BSIZE - off % BSIZE);
        let disk_guard = DISK.read();
        let disk = disk_guard.as_ref().unwrap();
        let bh = disk.bread(block_no as u64)?;
        let data_slice = bh.data();

        let data_off = off % BSIZE;
        let data_region = &data_slice[data_off..data_off + m];

        let copy_region = &mut buf[dst..dst + m];
        copy_region.copy_from_slice(data_region);

        tot += m;
        off += m;
        dst += m;
    }
    return Ok(n);
}

pub fn writei(
    buf: &[u8],
    _off: usize,
    n: usize,
    internals: &mut InodeInternal,
    inum: u32,
) -> Result<usize, Error> {
    let mut off = _off;
    let i_size = internals.size as usize;
    if off + n < off {
        return Err(Error::EIO);
    }
    if off + n > (MAXFILE as usize) * BSIZE {
        return Err(Error::EIO);
    }

    let max_blocks = (MAXOPBLOCKS - 1 - 1 - 2) / 2;
    let mut written_blocks = 0;
    let mut end_size = i_size;

    if off > i_size {
        let mut start_off = i_size;
        while start_off < off {
            if written_blocks >= max_blocks {
                break;
            }
            let block_no = bmap(internals, start_off / BSIZE)?;
            let disk_guard = DISK.read();
            let disk = disk_guard.as_ref().unwrap();
            let mut bh = disk.bread(block_no as u64)?;

            let b_data = bh.data_mut();

            let m = min(off - start_off, BSIZE - start_off % BSIZE);
            for i in start_off..start_off + m {
                let idx = b_data.get_mut(i % BSIZE).ok_or(Error::EIO)?;
                *idx = 0;
            }
            bh.mark_buffer_dirty();
            written_blocks += 1;
            log_write(block_no);

            start_off += m;
            end_size = start_off;
        }
    }

    let mut src = 0;
    let mut tot = 0;

    while tot < n {
        if written_blocks >= max_blocks {
            break;
        }
        let block_no = bmap(internals, off / BSIZE)?;
        let m = min(n - tot, BSIZE - off % BSIZE);

        let disk_guard = DISK.read();
        let disk = disk_guard.as_ref().unwrap();
        let mut bh = disk.bread(block_no as u64)?;

        let data_slice = bh.data_mut();
        let data_off = off % BSIZE;
        let data_region = &mut data_slice[data_off..data_off + m];

        let copy_region = &buf[src..src + m];
        data_region.copy_from_slice(copy_region);
        bh.mark_buffer_dirty();
        log_write(block_no);
        written_blocks += 1;

        tot += m;
        off += m;
        src += m;
        end_size = off;
    }

    if n > 0 && end_size > i_size {
        internals.size = end_size as u64;
        iupdate(internals, inum)?;
    }
    return Ok(n);
}

pub fn namecmp(s: &CStr, t: &str) -> i32 {
    return strcmp_rs(s.to_raw() as *const i8, t.as_ptr() as *const i8);
}

pub fn dirlookup(
    internals: &mut InodeInternal,
    name: &CStr,
    poff: &mut u64,
) -> Result<CachedInode, Error> {
    // Check if inode is directory
    if internals.inode_type != T_DIR {
        return Err(Error::ENOTDIR);
    }
    let de_size = mem::size_of::<Xv6fsDirent>();
    let mut de_arr_cont = kmem::MemContainer::<u8>::alloc(BSIZE).ok_or(Error::EIO)?;

    let num_blocks = match internals.size {
        0 => 0,
        _ => (internals.size as usize - 1) / BSIZE + 1,
    };

    for block_idx in 0..num_blocks {
        let de_arr_slice = de_arr_cont.to_slice_mut();
        readi(de_arr_slice, BSIZE * block_idx, BSIZE, internals)?;
        // resolve all dirent entries in the current data block.
        for de_idx in 0..BSIZE / de_size {
            let mut de = Xv6fsDirent::new();
            let de_slice = &mut de_arr_slice[de_idx * de_size..(de_idx + 1) * de_size];
            de.extract_from(de_slice).map_err(|_| Error::EIO)?;

            if (block_idx * BSIZE + de_idx * de_size) as u64 >= internals.size {
                break;
            }
            if de.inum == 0 {
                continue;
            }
            let name_str = match str::from_utf8(&de.name) {
                Ok(x) => x,
                Err(_) => break,
            };
            if namecmp(name, name_str) == 0 {
                *poff = (block_idx * BSIZE + de_idx * de_size) as u64;
                return iget(de.inum as u64);
            }
        }
    }

    return Err(Error::ENOENT);
}

// create subdirectory with 'name' under the directory pointed to by 'internals'
pub fn dirlink(
    internals: &mut InodeInternal,
    name: &CStr,
    child_inum: u32,
    parent_inum: u32,
) -> Result<usize, Error> {
    // Check if inode is directory
    if internals.inode_type != T_DIR {
        return Err(Error::ENOTDIR);
    }

    let de_size = mem::size_of::<Xv6fsDirent>();
    let mut de_arr_cont = kmem::MemContainer::<u8>::alloc(BSIZE).ok_or(Error::EIO)?;
    let mut final_off = None;

    let num_blocks = match internals.size {
        0 => 0,
        _ => (internals.size as usize - 1) / BSIZE + 1,
    };
    for block_idx in 0..num_blocks {
        let de_arr_slice = de_arr_cont.to_slice_mut();
        readi(de_arr_slice, BSIZE * block_idx, BSIZE, internals)?;

        for de_idx in 0..BSIZE / de_size {
            if (block_idx * BSIZE + de_idx * de_size) >= internals.size as usize {
                break;
            }
            let mut de = Xv6fsDirent::new();
            let de_slice = &mut de_arr_slice[de_idx * de_size..(de_idx + 1) * de_size];
            de.extract_from(de_slice).map_err(|_| Error::EIO)?;
            if de.inum == 0 {
                final_off = Some((block_idx * BSIZE + de_idx * de_size) as u64);
                break;
            }
        }
        if final_off.is_some() {
            break;
        }
    }
    let final_off = final_off.unwrap_or(internals.size);
    let mut de_cont = kmem::MemContainer::<u8>::alloc(de_size).ok_or(Error::EIO)?;

    let buf_len = de_cont.len();
    let mut de = Xv6fsDirent::new();
    let de_slice = de_cont.to_slice_mut();
    de.extract_from(de_slice).map_err(|_| Error::EIO)?;

    let name_slice = name.to_bytes_with_nul();
    if name_slice.len() > DIRSIZ as usize {
        return Err(Error::EIO);
    }
    for (idx, ch) in de.name.iter_mut().enumerate() {
        *ch = match name_slice.get(idx) {
            Some(x) => *x,
            None => 0,
        };
    }

    de.inum = child_inum as u32;
    de.dump_into(de_slice).map_err(|_| Error::EIO)?;

    if writei(
        de_slice,
        final_off as usize,
        buf_len,
        internals,
        parent_inum,
    )? != buf_len
    {
        return Err(Error::EIO);
    }

    return Ok(0);
}
