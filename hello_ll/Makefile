RELEASE=1

RUSTC := rustc
RUSTCFLAGS += --crate-type=staticlib -Z no-landing-pads -Z plt=y -C no-redzone -C code-model=kernel -C relocation-model=static -C force-frame-pointers=yes -C target-feature="-mmx,-sse,-sse2,-sse3,-ssse3,-sse4.1,-sse4.2,-3dnow,-3dnowa,-avx,-avx2" --emit=llvm-ir

CARGO := cargo
CARGOFLAGS :=

RUST_TARGET := x86_64-unknown-linux-gnu
#x86_64-unknown-none

export C_DIR := wrappers
export RUST_DIR := rust
export KERN_BUILD_DIR := module
#export TRANSFORMER_DIR := transformer
#export CHECKER_DIR := checker

C_SRCS := $(wildcard $(C_DIR)/*.c)
C_HDRS := $(wildcard $(C_DIR)/*.h)
C_SRCS := $(patsubst $(C_DIR)/%.c,$(KERN_BUILD_DIR)/%.c,$(C_SRCS))
C_HDRS := $(patsubst $(C_DIR)/%.h,$(KERN_BUILD_DIR)/%.h,$(C_HDRS))
C_OBJS := $(patsubst $(KERN_BUILD_DIR)/%.c,%.o,$(C_SRCS))

KERN_MAKEFILE := kbuild.mk

export MODULE_NAME := hello_ll
export RUST_LIB_NAME := libhello_ll.a
export KERN_OBJS := $(C_OBJS) $(RUST_LIB_NAME)

.PHONY: clean dirs

all: dirs $(C_SRCS) $(C_HDRS) $(RUST_LIB_NAME) $(KERN_MAKEFILE)
	cp $(KERN_MAKEFILE) $(KERN_BUILD_DIR)/Makefile;
	cp $(RUST_DIR)/target/$(RUST_TARGET)/$(if ${RELEASE},release,debug)/$(RUST_LIB_NAME) $(KERN_BUILD_DIR);
	cd $(KERN_BUILD_DIR) && make -C .

#all: dirs $(C_SRCS) $(C_HDRS) $(KERN_MAKEFILE)
#	cp $(KERN_MAKEFILE) $(KERN_BUILD_DIR)/Makefile;
#	cd $(KERN_BUILD_DIR) && make -C .

$(RUST_LIB_NAME):
	cd $(RUST_DIR) && $(CARGO) rustc $(if $(RELEASE),--release) $(if $(V),--verbose) \
	$(CARGOFLAGS) --target="$(RUST_TARGET)" -- $(RUSTCFLAGS);

dirs: 
	mkdir -p $(KERN_BUILD_DIR)

$(KERN_BUILD_DIR)/%.h: $(C_DIR)/%.h
	cp $< $@

$(KERN_BUILD_DIR)/%.c: $(C_DIR)/%.c
	cp $< $@

clean:
	cd $(RUST_DIR) && $(CARGO) clean;
	rm -rf $(OBJ_DIR) $(KERN_BUILD_DIR)

#clean:
#	rm -rf $(OBJ_DIR) $(KERN_BUILD_DIR)
