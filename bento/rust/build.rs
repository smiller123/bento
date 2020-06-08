/// Build file borrowed from fishinabarrel/linux-kernel-module-rust on Github.
extern crate bindgen;
extern crate cc;
extern crate shlex;

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

const INCLUDED_TYPES: &[&str] = &[
    "file_system_type",
    "mode_t",
    "umode_t",
    "ctl_table",
    "fuse_dirent",
    "fuse_init_in",
    "fuse_init_out",
    "fuse_forget_in",
    "fuse_ioctl_in",
    "fuse_ioctl_out",
    "fuse_attr",
    "fuse_flush_in",
    "fuse_getxattr_in",
    "fuse_getxattr_out",
    "fuse_kstatfs",
    "fuse_statfs_out",
    "fuse_open_in",
    "fuse_open_out",
    "fuse_getattr_in",
    "fuse_setattr_in",
    "fuse_attr_out",
    "fuse_entry_out",
    "fuse_read_in",
    "fuse_write_in",
    "fuse_write_out",
    "fuse_create_in",
    "fuse_mknod_in",
    "fuse_mkdir_in",
    "fuse_lseek_in",
    "fuse_lseek_out",
    "fuse_fsync_in",
    "fuse_file_lock",
    "fuse_lk_in",
    "fuse_lk_out",
    "fuse_bmap_in",
    "fuse_bmap_out",
    "fuse_poll_in",
    "fuse_poll_out",
    "fuse_fallocate_in",
    "fuse_access_in",
    "fuse_setxattr_in",
    "fuse_rename2_in",
    "fuse_release_in",
    "fuse_opcode",
    "fuse_in_header",
    "fuse_out_header",
    "timespec",
    "fuse_link_in",
];
const INCLUDED_FUNCTIONS: &[&str] = &[
    "cdev_add",
    "cdev_init",
    "cdev_del",
    "register_filesystem",
    "unregister_filesystem",
    "__kmalloc",
    "krealloc",
    "kfree",
    "mount_nodev",
    "kill_litter_super",
    "register_sysctl",
    "unregister_sysctl_table",
    "access_ok",
    "_copy_to_user",
    "_copy_from_user",
    "alloc_chrdev_region",
    "unregister_chrdev_region",
    "wait_for_random_bytes",
    "get_random_bytes",
    "rng_is_initialized",
    "IS_ERR",
];
const INCLUDED_VARS: &[&str] = &[
    "EPERM",
    "ENOENT",
    "EIO",
    "EAGAIN",
    "ENOMEM",
    "EEXIST",
    "EINVAL",
    "ENOTDIR",
    "EISDIR",
    "ESPIPE",
    "EFAULT",
    "ENOSYS",
    "ENAMETOOLONG",
    "EOVERFLOW",
    "ENOTEMPTY",
    "__this_module",
    "FS_REQUIRES_DEV",
    "FS_BINARY_MOUNTDATA",
    "FS_HAS_SUBTYPE",
    "FS_USERNS_MOUNT",
    "FS_RENAME_DOES_D_MOVE",
    "BINDINGS_GFP_KERNEL",
    "KERN_INFO",
    "VERIFY_WRITE",
    "LINUX_VERSION_CODE",
    "SEEK_SET",
    "SEEK_CUR",
    "SEEK_END",
    "O_CREAT",
    "O_TRUNC",
    "FUSE_ASYNC_READ",
    "FUSE_POSIX_LOCKS",
    "FUSE_FILE_OPS",
    "FUSE_ATOMIC_O_TRUNC",
    "FUSE_EXPORT_SUPPORT",
    "FUSE_BIG_WRITES",
    "FUSE_DONT_MASK",
    "FUSE_SPLICE_WRITE",
    "FUSE_SPLICE_MOVE",
    "FUSE_SPLICE_READ",
    "FUSE_FLOCK_LOCKS",
    "FUSE_HAS_IOCTL_DIR",
    "FUSE_AUTO_INVAL_DATA",
    "FUSE_WRITEBACK_CACHE",
    "FUSE_NO_OPEN_SUPPORT",
    "FUSE_PARALLEL_DIROPS",
    "FUSE_HANDLE_KILLPRIV",
    "FUSE_POSIX_ACL",
    "I_NEW",
    "S_NOATIME",
    "ST_RDONLY",
    "DT_UNKNOWN",
    "DT_FIFO",
    "DT_CHR",
    "DT_DIR",
    "DT_BLK",
    "DT_REG",
    "DT_LNK",
    "DT_SOCK",
    "DT_WHT",
    "FOPEN_DIRECT_IO",
    "FOPEN_KEEP_CACHE",
    "FOPEN_NONSEEKABLE",
    "FUSE_MIN_READ_BUFFER",
    "__GFP_RECLAIM",
    "__GFP_IO",
    "__GFP_FS",
    "FATTR_MODE",
    "FATTR_UID",
    "FATTR_GID",
    "FATTR_SIZE",
    "FATTR_ATIME",
    "FATTR_MTIME",
    "FATTR_FH",
    "FATTR_ATIME_NOW",
    "FATTR_MTIME_NOW",
    "FATTR_LOCKOWNER",
    "FATTR_CTIME",
];
const OPAQUE_TYPES: &[&str] = &[
    // These need to be opaque because they're both packed and aligned, which rustc
    // doesn't support yet. See https://github.com/rust-lang/rust/issues/59154
    // and https://github.com/rust-lang/rust-bindgen/issues/1538
    "desc_struct",
    "xregs_state",
];

fn kernel_version_code(major: u8, minor: u8, patch: u8) -> u64 {
    ((major as u64) << 16) | ((minor as u64) << 8) | (patch as u64)
}

fn handle_kernel_version_cfg(bindings_path: &PathBuf) {
    let f = BufReader::new(fs::File::open(bindings_path).unwrap());
    let mut version = None;
    for line in f.lines() {
        let line = line.unwrap();
        if let Some(type_and_value) = line.split("pub const LINUX_VERSION_CODE").nth(1) {
            if let Some(value) = type_and_value.split("=").nth(1) {
                let raw_version = value.split(";").next().unwrap();
                version = Some(raw_version.trim().parse::<u64>().unwrap());
                break;
            }
        }
    }
    let version = version.expect("Couldn't find kernel version");
    if version >= kernel_version_code(4, 15, 0) {
        println!("cargo:rustc-cfg=kernel_4_15_0_or_greater")
    }
    if version >= kernel_version_code(4, 19, 0) {
        println!("cargo:rustc-cfg=kernel_4_19_0_or_greater")
    }
    if version >= kernel_version_code(4, 20, 0) {
        println!("cargo:rustc-cfg=kernel_4_20_0_or_greater")
    }
    if version >= kernel_version_code(5, 1, 0) {
        println!("cargo:rustc-cfg=kernel_5_1_0_or_greater")
    }
}

fn handle_kernel_symbols_cfg(symvers_path: &PathBuf) {
    let f = BufReader::new(fs::File::open(symvers_path).unwrap());
    for line in f.lines() {
        let line = line.unwrap();
        if let Some(symbol) = line.split_ascii_whitespace().nth(1) {
            if symbol == "setfl" {
                println!("cargo:rustc-cfg=kernel_aufs_setfl");
                break;
            }
        }
    }
}

fn add_env_if_present(cmd: &mut Command, var: &str) {
    if let Ok(val) = env::var(var) {
        cmd.env(var, val);
    }
}

fn main() {
    println!("build.rs running");
    println!("cargo:rerun-if-env-changed=KDIR");
    let kdir = env::var("KDIR").unwrap_or(format!(
        "/lib/modules/{}/build",
        std::str::from_utf8(&(Command::new("uname").arg("-r").output().unwrap().stdout))
            .unwrap()
            .trim()
    ));

    println!("cargo:rerun-if-env-changed=CLANG");
    println!("cargo:rerun-if-changed=kernel-cflags-finder/Makefile");
    let mut cmd = Command::new("make");
    cmd.arg("-C")
        .arg("kernel-cflags-finder")
        .arg("-s")
        .env_clear();
    add_env_if_present(&mut cmd, "KDIR");
    add_env_if_present(&mut cmd, "CLANG");
    add_env_if_present(&mut cmd, "PATH");
    let output = cmd.output().unwrap();
    if !output.status.success() {
        eprintln!("kernel-cflags-finder did not succeed");
        eprintln!("stdout: {}", std::str::from_utf8(&output.stdout).unwrap());
        eprintln!("stderr: {}", std::str::from_utf8(&output.stderr).unwrap());
        std::process::exit(1);
    }

    let target = env::var("TARGET").unwrap();

    let mut builder = bindgen::Builder::default()
        .use_core()
        .ctypes_prefix("raw")
        .derive_default(true)
        .rustfmt_bindings(true);

    builder = builder.clang_arg(format!("--target={}", target));
    for arg in shlex::split(std::str::from_utf8(&output.stdout).unwrap()).unwrap() {
        builder = builder.clang_arg(arg.to_string());
    }

    println!("cargo:rerun-if-changed=src/bindings_helper.h");
    builder = builder.header("src/bindings_helper.h");

    for t in INCLUDED_TYPES {
        builder = builder.whitelist_type(t);
    }
    for f in INCLUDED_FUNCTIONS {
        builder = builder.whitelist_function(f);
    }
    for v in INCLUDED_VARS {
        builder = builder.whitelist_var(v);
    }
    for t in OPAQUE_TYPES {
        builder = builder.opaque_type(t);
    }
    let bindings = builder.generate().expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    println!("rust out_path {:?}", out_path);
    handle_kernel_version_cfg(&out_path.join("bindings.rs"));
    handle_kernel_symbols_cfg(&PathBuf::from(&kdir).join("Module.symvers"));

    let mut builder = cc::Build::new();
    builder.compiler(env::var("CLANG").unwrap_or("clang".to_string()));
    builder.target(&target);
    builder.warnings(false);
    builder.file("src/helpers.c");
    for arg in shlex::split(std::str::from_utf8(&output.stdout).unwrap()).unwrap() {
        builder.flag(&arg);
    }
    builder.compile("helpers");
}
