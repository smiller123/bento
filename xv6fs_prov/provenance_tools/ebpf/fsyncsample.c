#include <unistd.h>
#include <stdio.h>
#include <fcntl.h>

int main(int argc, char *argv[]){
	char *path = "/mnt/xv6fsll/";
	int fd = open("/mnt/xv6fsll/", O_DIRECTORY, 0777);
	//int fd = open("/mnt/xv6fsll/test", O_CREAT, 0777);
	fsync(fd);
}
