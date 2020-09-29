This is a userspace server for a networked version of the simple `hello_ll` file system.

The file system is compiled as a Linux kernel module and depends on the
bentofs kernel module.

We use Linux kernel version 4.15 and Rust nightly version 1.43.0.

**To compile:**
```
make
```

**To clean:**
```
make clean
```

**To run:**
```
sudo userspace/target/release/hello_srv hello
```
