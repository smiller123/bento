#ifndef __XV6FS_H__
#define __XV6FS_H__

/*
 *  Disk layout:
 *  +-----------------------------------------------+
 *  | empty | super | bitmap | inodes | data |  log |
 *  +-----------------------------------------------+
 *
 */

struct xv6fs_super_block {
  uint size;       // Size of file system image (blocks)
  uint nblocks;    // Number of data blocks
  uint ninodes;    // Number of inodes.
  uint nlog;       // Number of log blocks
  uint logstart;   // Block number of first log block
  uint inodestart; // Block number of first inode block
  uint bmapstart;  // Block number of first free map block
  uint xv6_magic;  // a magic number
};

#define ROOTINO 1  // root i-number
#define BSIZE 4096 // block size

#define SB_BLK_NO 1

#define NDIRECT 8
#define NINDIRECT (BSIZE / sizeof(uint))
#define NDINDIRECT (NINDIRECT * NINDIRECT)
#define MAXFILE (NDIRECT + NINDIRECT + NDINDIRECT)

// On-disk inode
struct disk_inode {
  short type;  // File type
  short major; // Major device number (T_DEV only)
  short minor; // Minor device number (T_DEV only)
  short nlink; // Number of links to inode in file system
  size_t size; // Size of file (bytes)
  int ctime;
  int atime;
  int mtime;

  uint addrs[NDIRECT + 2]; // Data block addresses
};

// some "special block numbers"
#define BLK_NP 0
#define BLK_ZERO_ON_DEMAND 1

#define FREE_BLK_START 2

#define I_BUSY 0x1
#define I_VALID 0x2

// Inodes per block.
#define IPB (BSIZE / sizeof(struct disk_inode))

// Block containing inode i
#define IBLOCK(i) ((i) / IPB + 2)

// Bitmap bits per block
#define BPB (BSIZE * 8)

// Block containing bit for block b
#define BBLOCK(b, ninodes) (b / BPB + (ninodes) / IPB + 3)

// Directory is a file containing a sequence of dirent structures.
#define DIRSIZ 58
struct dirent {
  char name[DIRSIZ];
  short type;
  uint ino;
} __attribute__((packed));

#endif
