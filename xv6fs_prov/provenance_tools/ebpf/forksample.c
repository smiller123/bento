#include <fcntl.h>
#include <unistd.h>
#include <stdio.h>
#include  <sys/types.h>
#include <sys/wait.h>

int main(int argc, char *argv[]) {
	int pid = fork();
	if (pid) {
		int status;
		wait(&status);
	}
	//sync();
	return 0;
}
