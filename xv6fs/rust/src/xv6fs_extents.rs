// TODO: might be used for helper functions, remove if not necessary
#[cfg(not(feature = "user"))]
use crate::libc;
#[cfg(not(feature = "user"))]
use crate::println;
#[cfg(not(feature = "user"))]
use bento::kernel::kobj::BufferHead;

use core::mem;
use datablock::DataBlock;

use alloc::sync::Arc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::xv6fs_file::*;
use crate::xv6fs_utils::*;
// TODO: might be used to add helper functions, remove if not necessary
use crate::xv6fs_fs::*;

// ***Note*** MSB not used. Might want to implement preallocation feature when converting to log file system
pub const EXT_INIT_MAX_LEN: u16 = 1 << 15; // max # of blocks in an extent is 2^15

// Max depth for an extent tree is root + 2 levels of index nodes + 1 level of leaf nodes
pub const EXT_TREE_MAX_DEPTH: u16 = 3;

pub const EH_LEN: usize = mem::size_of::<Xv6fsExtentHeader>();
pub const EXT_LEN: usize = mem::size_of::<Xv6fsExtent>();
pub const EI_LEN: usize = mem::size_of::<Xv6fsExtentIdx>();
// TODO: define macros to get index of first and last extents in a extent vector slice

/**
 * xv6fs_inode has addrs array (64 bytes)
 * The first 12 bytes store and xv6fs_extent_header;
 * the remainder stores an array of xv6fs_extent
 * 
 **/

/**
 * This is the extent on-disk structure.
 * It's used at the bottom of the tree
 **/
#[repr(C)]
#[derive(DataBlock, Copy, Clone)]
pub struct Xv6fsExtent {
    pub xe_block: u32,      /* first logical block extent covers */ 
    pub xe_block_addr: u32, /* actuall block address */
    pub xe_len: u16,        /* number of blocks covered by extent */
}

impl Xv6fsExtent {
    pub const fn new() -> Self {
        Self {
            xe_block: 0,
            xe_block_addr: 0,
            xe_len: 0,
        }
    }

    pub fn clear(&mut self) {
        self.xe_block = 0;
        self.xe_block_addr = 0;
        self.xe_len = 0;
    }
}

/**
 * This is index on-disk structure.
 * It's used at all the levels except the bottom.
 **/
#[repr(C)]
#[derive(DataBlock, Copy, Clone)]
 pub struct Xv6fsExtentIdx {
     pub ei_block: u32,     /* index covers logical blocks from 'block' */
     pub ei_block_addr: u32, /* actual logical block address */
 }

 impl Xv6fsExtentIdx {
     pub const fn new() -> Self {
         Self {
             ei_block: 0,
             ei_block_addr: 0,
         }
     }

     pub fn clear(&mut self) {
         self.ei_block = 0;
         self.ei_block_addr = 0;
     }
 }

 // TODO: prob don't need eh_magic & eh_generation
 /**
  * Each block (inode, indexes and leaves) has a header.
  **/
#[repr(C)]
#[derive(DataBlock, Copy, Clone)]
pub struct Xv6fsExtentHeader {
	pub	eh_entries: u16,	/* number of valid entries */
	pub	eh_max: u16,		/* capacity of store in entries */
	pub	eh_depth: u16,	    /* has tree real underlying blocks? */
}

impl Xv6fsExtentHeader {
    pub const fn new() -> Self {
        Self {
            eh_entries: 0,
            eh_max: 0,
            eh_depth: 0,
        }
    }

    pub fn clear(&mut self) {
        self.eh_entries = 0;
        self.eh_max = 0;
        self.eh_depth = 0;
    }
}

// TODO: might not need this struct at all
/**
 * Array of Xv6fsExtPath contains path to some extent.
 * Creation/lookup routines use it for traversal/splitting/etc.
 * Truncate uses it to simulate recursive walking.
 **/
pub struct Xv6fsExtentPath {
    // TODO: might need to remove p_block and p_block_addr since the same information is stored in the extent pointers below. Currently kept
    pub p_block: u32, // block to current path (i.e. extent or extent index).
    pub p_block_addr: u32, // actual addr block to current path.
    pub p_depth: u16, // depth of the extent or extent index.
    pub p_maxdepth: u16, // max depth of this extent tree. TODO: double check on this value
    pub p_ext: Option<Arc<Xv6fsExtent>>, // extent pointer to next node if current block is root.
    pub p_idx: Option<Arc<Xv6fsExtentIdx>>, // extent index pointer to next node if current block is index node.
    pub p_hdr: Option<Arc<Xv6fsExtentHeader>>, // header of current extent node. Can be root, index, or leaf node.
    pub p_bh: Option<Arc<BufferHead>>, // buffer head of the current node.
}

impl Xv6fsExtentPath {
    pub const fn new() -> Self {
        Self {
            p_block: 0,
            p_block_addr: 0,
            p_depth: 0,
            p_maxdepth: 0,
            p_ext: None,
            p_idx: None,
            p_hdr: None,
            p_bh: None,
        }
    }
}


// Extent related functions
/*
// Extract the extent header from an inode slice
pub fn inode_to_ext_header(inode_slice: &mut [u8]) -> Result<Xv6fsExtentHeader, libc::c_int> {
   let inode_ext_header_slice = &mut inode_slice[mem::size_of::<Xv6fsInode> - NDIRECT - 2>..];
   let mut xv6fs_ext_header = Xv6fsExtentHeader::new();
   xv6fs_ext_header
      .extract_from(inode_ext_header_slice)
      .map_err(|_| libc::EIO);
   return xv6fs_ext_header;
}
*/
// Extract the extent header from an extent block
pub fn bh_to_ext_header(path: &Xv6fsExtentPath) -> Result<Xv6fsExtentHeader, libc::c_int> {
    let bh_arr_slice = Arc::get_mut(&mut path.p_bh.unwrap()).unwrap().data_mut();
    let eh_slice = &mut bh_arr_slice[0..mem::size_of::<Xv6fsExtentHeader>()];
    let mut eh = Xv6fsExtentHeader::new();
    eh.extract_from(eh_slice).map_err(|_| libc::EIO)?;
    return Ok(eh);
}

/*
 * Given the size of an inode, it returns the next block index to use for block indexing
 * 
 * The next block index should be only increasing.
 */
pub fn get_next_block_idx(size: u64) -> u32 {
    return (2 + ((size - 1) / BSIZE as u64)) as u32;
}

/*
 * Given an inode and new extent, it return a sorted btreemap with the extents
 * 
 */
pub fn get_sorted_root_ext(inode: &mut InodeInternal, new_ext: Xv6fsExtent) -> Result<BTreeMap<u32, Xv6fsExtent>, libc::c_int> {
    let re_map: BTreeMap<u32, Xv6fsExtent> = BTreeMap::new();
        for root_idx in 0..inode.eh.eh_entries {
            re_map.insert(inode.ee_arr[root_idx as usize].xe_block, inode.ee_arr[root_idx as usize]);
        }
        re_map.insert(new_ext.xe_block, new_ext);
    return Ok(re_map);
}

/*
 * Copies values from a BTreeMap to the root extents array
 * 
 */
pub fn insert_to_root_ext_sorted(inode: &mut InodeInternal, re_map: &BTreeMap<u32, Xv6fsExtent>) -> Result<(), libc::c_int> {
    let re_keys: Vec<u32> = re_map.keys().cloned().collect();
    let idx = 0;
    while idx < re_keys.len() {
        if idx >= INEXTENTS as usize {
            println!("root extents >= INEXTENTS");
            return Err(libc::EIO);
        }
        inode.ee_arr[idx as usize] = *re_map.get(&re_keys[idx]).unwrap();
        idx += 1;
    }
    inode.eh.eh_entries = re_keys.len() as u16;
    return Ok(());
}
/*
 * Our extents in the inode differ from ext4
 * In all cases, the extents in the root will be extents and not extent indeces for simplicity.
 * When ext tree depth > 0, extents will be used as extent indeces.
 */
pub fn ext_max_entries(inode: &InodeInternal, depth: u16) -> u16 {
    let max_depth = inode.eh.eh_depth;
    let mut max_entries = 0 as usize;
    if depth == max_depth {
        if depth == 0 { // number of entries in root node
            max_entries = INEXTENTS as usize;
        } else { // leaf node
            max_entries = BSIZE;
            max_entries -= mem::size_of::<Xv6fsExtentHeader>();
            max_entries /= mem::size_of::<Xv6fsExtent>();
        }
    } else { // depth != max_entries
        if depth == 0 { // number of entries in root node
            max_entries = INEXTENTS as usize;
        } else { // index node
            max_entries = BSIZE;
            max_entries -= mem::size_of::<Xv6fsExtentHeader>();
            max_entries /= mem::size_of::<Xv6fsExtentIdx>();
        }
    }
    return max_entries as u16;
}

/**
 * Check extent header before calling this method.
 * binary search on leaf node block of tree, only extents are present at this level where curr_depth == max_tree_depth
 * 
 */
pub fn ext_binary_search(arr: &[Xv6fsExtent], len: u32, target_block: u32) -> Option<u32> {
    let mut lo: u32 = 0;
    let mut hi: u32 = len as u32 - 1;
    
    while lo <= hi { 
        let mid = lo + (hi - lo) / 2;
        if mid >= len {
            return Some(mid - 1);
        }
        let block = arr[mid as usize].xe_block;
        if target_block < block {
            if mid == 0 {
                return None;
            }
            hi = mid - 1;
        } else {
            lo = mid + 1;
        }
        return Some(lo - 1);
    }
    return None;
}

/**
 * Check extent header before calling this method.
 * binary search on index node block of tree 
 */
pub fn ext_binary_search_idx(arr: &[Xv6fsExtentIdx], len: u32, target_block: u32) -> Option<u32> {
    let mut lo: u32 = 0;
    let mut hi: u32 = len as u32 - 1;
    
    while lo <= hi { 
        let mid = lo + (hi - lo) / 2;
        if mid >= len {
            return Some(mid - 1);
        }
        let block = arr[mid as usize].ei_block;
        if target_block < block {
            if mid == 0 {
                return None;
            }
            hi = mid - 1;
        } else {
            lo = mid + 1;
        }
        return Some(lo - 1);
    }
    return None;
}
