#ifndef __XV6FS_H__
#define __XV6FS_H__

#define T_DIR  1   // Directory
#define T_FILE 2   // File
#define T_DEV  3   // Device

#define MAXOPBLOCKS 	32
#define LOGSIZE 	32768
#define FSSIZE       	2000000

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
};

#define ROOTINO 1  // root i-number
#define BSIZE 4096 // block size

#define SB_BLK_NO 1

#define NDIRECT 10
#define NINDIRECT (BSIZE / sizeof(uint))
#define NDINDIRECT (NINDIRECT * NINDIRECT)
#define MAXFILE (NDIRECT + NINDIRECT + NDINDIRECT)

// On-disk inode
struct dinode {
  short type;  // File type
  short major; // Major device number (T_DEV only)
  short minor; // Minor device number (T_DEV only)
  short nlink; // Number of links to inode in file system
  unsigned long long size; // Size of file (bytes)

  uint addrs[NDIRECT + 2]; // Data block addresses
};

// some "special block numbers"
#define BLK_NP 0
#define BLK_ZERO_ON_DEMAND 1

#define FREE_BLK_START 2

#define I_BUSY 0x1
#define I_VALID 0x2

// Inodes per block.
#define IPB (BSIZE / sizeof(struct dinode))

// Block containing inode i
#define IBLOCK(i, sb) ((i) / IPB + sb.inodestart)

// Bitmap bits per block
#define BPB (BSIZE * 8)

// Directory is a file containing a sequence of dirent structures.
#define DIRSIZ 60
struct dirent {
  uint inum;
  char name[DIRSIZ];
} __attribute__((packed));

#endif
