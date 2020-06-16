#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <string.h>
#include <stdio.h>
#include <unistd.h>


#include "../xv6fs.h"

void pwrite_repeat(int fd, const void *buf, size_t count, off_t offset) {
    off_t off = offset;
    off_t end = offset + count;
    printf("pwrite params: %d %p %lu %lu\n", fd, buf, count, offset);
    while (off != end) {
        int ret = pwrite(fd, buf+ (off - offset), end - off, off);
        off += ret;
        if (ret < 0) {
            perror("Pwrite_repeat");
            return;
        }
    }
    return;
}

int main(int argc, char *argv[]) {
    char *fn;
    int fd;
    int i;


    if (argc < 2) {
        printf("USAGE %s <file-name>\n", argv[0]);
        return -1;
    }

    fn = argv[1];

    fd = open(fn, O_CREAT | O_TRUNC | O_RDWR, 00666);

    if (!fd) {
        goto err_out;
    }

    struct xv6fs_super_block super;
    super.size = 2000000;
    super.ninodes = 10000;
    super.nlog = 128;
    super.logstart = super.size - super.nlog - 1;
    super.inodestart = 160;
    super.bmapstart = 2;
    super.xv6_magic = 0xdeadbeef;

    char zero[4096];
    memset(zero, 0, sizeof(zero));
    for (i = 0; i < super.size; i++) {
        pwrite_repeat(fd, (void *)zero, sizeof(zero), (size_t)i * 4096L);
    }
    // super block
    pwrite_repeat(fd, (void *)&super, sizeof(struct xv6fs_super_block), BSIZE);

    // root inode
    struct disk_inode root_inode;
    memset((void *)&root_inode, 0, sizeof(struct disk_inode));
    root_inode.nlink = 2;
    root_inode.type = S_IFDIR | 00777;
    root_inode.size = 0;

    for (i = 0; i < NDIRECT + 2; i++) {
        root_inode.addrs[i] = 0;
    }

    char bm = 0x1;
    pwrite_repeat(fd, (void *)&bm, sizeof(char), super.inodestart + super.ninodes);

    pwrite_repeat(fd, (void *)&root_inode, sizeof(struct disk_inode),
                  super.inodestart * BSIZE + ROOTINO * sizeof(struct disk_inode));

    fdatasync(fd);

    close(fd);

    return 0;

 err_out:
    return -1;
}
