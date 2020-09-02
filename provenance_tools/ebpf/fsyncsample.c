#include <unistd.h>
#include <stdio.h>
#include <fcntl.h>

int main(int argc, char *argv[]){
	char *path = "/mnt/xv6fsll/test.txt";
	int fd = open(path, O_CREAT, 0);
	fsync(fd);
}
