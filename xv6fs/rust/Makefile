.DEFAULT_GOAL := kernel
.PHONY: kernel userspace clean
KDIR ?= /lib/modules/$(shell uname -r)/build

all:
	make kernel
	make userspace

kernel:
	#cp src/* kernel/src/
	cp src/xv6fs_log.rs src/xv6fs_utils.rs src/xv6fs_file.rs kernel/src
	$(MAKE) -C $(KDIR) M=$(CURDIR)/kernel

userspace:
	#cp src/* userspace/src/
	cp src/xv6fs_log.rs src/xv6fs_utils.rs src/xv6fs_file.rs userspace/src
	(cd userspace; cargo build --release --features "user")

clean:
	$(MAKE) -C $(KDIR) M=$(CURDIR)/kernel clean
	(cd kernel; cargo clean)
	#rm -f kernel/src/xv6*.rs
	rm -f kernel/src/xv6fs_log.rs kernel/src/xv6fs_utils.rs kernel/src/xv6fs_file.rs
	(cd userspace; cargo clean)
	#rm -f userspace/src/xv6*.rs
	rm -f userspace/src/xv6fs_log.rs userspace/src/xv6fs_utils.rs userspace/src/xv6fs_file.rs
