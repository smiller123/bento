.DEFAULT_GOAL := kernel
.PHONY: kernel userspace clean
KDIR ?= /lib/modules/$(shell uname -r)/build

all:
	make kernel
	make userspace

kernel:
	cp src/* kernel/src
	$(MAKE) -C $(KDIR) M=$(CURDIR)/kernel

userspace:
	cp src/* userspace/src
	(cd userspace; cargo build --release --features "user")

clean:
	$(MAKE) -C $(KDIR) M=$(CURDIR)/kernel clean
	(cd kernel; cargo clean)
	rm -f kernel/src/xv6*.rs
	(cd userspace; cargo clean)
	rm -f userspace/src/xv6*.rs
