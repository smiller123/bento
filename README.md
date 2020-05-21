This is a library for writing a kernel file system using the FUSE lowlevel interface.

### Project Structure

#### Libraries
* **bentofs**: Module to enable file systems written using the FUSE low-level interface in the kernel. Compiled and inserted as a separate kernel module.
* **bento**: Rust library that exposes safe Rust bindings to a kernel file system. Included by the file systems, so not compiled separately.
* **datablock-rs**: Rust library that enables safe, efficient reading of basic data structures from byte arrays. Included by the file systems, so not compiled separately.

#### File Systems
* **hello_ll**: Basic file system. Compiled and inserted as a separate kernel module.
* **xv6fs**: Rust reimplementation of the xv6 file system. Compiled and inserted as a separate kernel module.

To pull bentofs and datablack-rs, run: `git submodule update --init --recursive` in the root directory. 
To update, run `git submodule update --recursive --remote`.

Instructions for compiling each module are included in the READMEs in each subdirectory.

More information about the system design can be found in our [report on ArXiv](https://arxiv.org/abs/2005.09723).
