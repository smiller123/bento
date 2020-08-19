.DEFAULT_GOAL := kernel
.PHONY: kernel userspace clean
KDIR ?= /lib/modules/$(shell uname -r)/build

all:
	make kernel
	make userspace

kernel:
	$(MAKE) -C $(KDIR) M=$(CURDIR)/kernel

userspace:
	(cd userspace; cargo build --release --features "user")

clean:
	$(MAKE) -C $(KDIR) M=$(CURDIR)/kernel clean
	(cd kernel; cargo clean)
	(cd userspace; cargo clean)
