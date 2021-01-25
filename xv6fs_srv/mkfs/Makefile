CFLAGS += -O2 -g -fPIC

all:	mkfs

mkfs:	mkfs.o
	gcc -o mkfs $^

clean:
	rm -rf *.o mkfs
