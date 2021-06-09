Bento enables fast development of Linux kernel file systems. File systems are written in safe Rust by implementing a safe API and using safe wrappers around kernel functions. These safe interfaces are as close as possible to existing userspace (primarily standard library) interfaces, so a file system can be recompiled as a FUSE file system by only changing the `bento` includes to userspace Rust library and/or `bento_utils` includes. File systems register themselves with the BentoFS module when inserted and can be dynamically upgraded with only 10ms of service interruption by having the upgrade version reregister with the BentoFS module. More information about the system design can be found in our [paper at FAST 2021](https://www.usenix.org/system/files/fast21-miller.pdf). Please also check out our [video from FAST](https://www.usenix.org/conference/fast21/presentation/miller) for an introduction to the framework.

### Before Running the Code
To pull bentofs and datablack-rs, run: `git submodule update --init --recursive` in the root directory. 
To update, run `git submodule update --recursive --remote`.

### Directories

#### BentoFS module
BentoFS is a C kernel module that interfaces between the VFS layer and the file system and exposes a safer API for Bento file systems. It's implemented as a VFS file system, handling calls from VFS and forwarding them to the appropriate file system. File systems register themselves with the BentoFS module, so the BentoFS module must be inserted before any file system module.

#### Bento library
The `bento` Rust library exposes Safe Rust interfaces to Bento file systems. Bento file systems implement the `BentoFileSystem` trait provided in the library. The `bento` library receives calls from BentoFS and translates these into safe calls to `BentoFileSystem` methods. The `bento` library also exposes safe wrappers around kernel types and functions, such as the `RwLock` for the kernel read-write semaphore and `TcpStream` and `TcpListener` for the kernel TCP bindings. Additionally, `bento` implements the Rust global allocator, so Bento file systems can use Rust’s `alloc` crate.

#### Bento Utils library
The `bento_utils` library exposes functionality needed for userspace Bento file systems. For the most part, interfaces provided in the `bento` library mirror existing userspace Rust libraries, most often the standard library, so a Bento file system can be compiled as a FUSE file system just by changing `bento` include statements to Rust library include statements. The `bento_utils` library provides userspace implementations of the remaining interfaces that aren't based on existing Rust libraries.

Example file systems are provided. Instructions for compiling each module are included in the READMEs in each subdirectory.

### Project Structure

#### Libraries
* **bentofs**: Module to enable file systems written using the FUSE low-level interface in the kernel. Compiled and inserted as a separate kernel module.
* **bento**: Rust library that exposes safe Rust bindings to a kernel file system. Included by the file systems, so not compiled separately.
* **bento_utils**: Rust library to expose kernel-specific implementations to userspace Bento file systems.
* **datablock-rs**: Rust library that enables safe, efficient reading of basic data structures from byte arrays. Included by the file systems, so not compiled separately.

#### File Systems
* **hello_ll**: Basic file system. Compiled and inserted as a separate kernel module.
* **hello_ll2**: Redeployable version of hello_ll. Nearly identical to hello_ll except for changes in kernel/lib.rs.
* **hello_client** and **hello_srv**: Networked version of hello_ll.
* **xv6fs**: Rust reimplementation of the xv6 file system. Compiled and inserted as a separate kernel module.
* **xv6fs2**: Redeployable version of xv6fs. Nearly identical to xv6fs except for changes in kernel/lib.rs.
* **xv6fs_prov**: Version of xv6fs with file provenance tracking.

