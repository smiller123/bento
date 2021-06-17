use std::io::{BufRead, BufReader};
use std::path::PathBuf;
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
    "timespec64",
    "fuse_link_in",
    "sockaddr_in",
    "sockaddr_in6",
    "in_addr",
    "in6_addr",
    "sa_family_t",
    "socket",
    "sock_type",
    "sock_shutdown_cmd",
    "timeval",
    "__kernel_sock_timeval",
    "msghdr",
    "kvec",
    "proto",
    "inet_connection_sock",
    "net_proto_family",
    "net_protocol",
    "socket_state",
    "sock_flags",
    "sock_type",
    "socket_state",
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
    "vmalloc",
    "vfree",
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
    "proto_register",
    "proto_unregister",
    "sock_register",
    "sock_unregister",
    "sk_alloc",
    "sock_init_data",
    "skb_queue_purge",
    "kfree_skb",
    "sk_mem_reclaim",
    "dst_release",
    "lock_sock_nested",
    "release_sock",
    "sk_stream_kill_queues",
    "sock_common_setsockopt",
    "ntohs",
    "ns_capable",
];
const INCLUDED_VARS: &[&str] = &[
    "FMODE_READ",
    "FMODE_WRITE",
    "FMODE_EXCL",
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
    "ENOTEMPTY",
    "ENAMETOOLONG",
    "EOVERFLOW",
    "ENOTEMPTY",
    "ECONNREFUSED",
    "ECONNRESET",
    "EPIPE",
    "ENOTCONN",
    "ECONNABORTED",
    "EADDRNOTAVAIL",
    "EADDRINUSE",
    "EINTR",
    "ETIMEDOUT",
    "EWOULDBLOCK",
    "EADDRINUSE",
    "EACCES",
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
    "AF_INET",
    "AF_INET6",
    "IPPROTO_TCP",
    "SOL_SOCKET",
    "SO_REUSEADDR",
    "IP_TTL",
    "SO_ERROR",
    "TCP_NODELAY",
    "SO_RCVTIMEO_OLD",
    "SO_RCVTIMEO_NEW",
    "SO_SNDTIMEO_OLD",
    "SO_SNDTIMEO_NEW",
    "SLAB_TYPESAFE_BY_RCU",
    "MSG_PEEK",
    "PAGE_SIZE",
    "__this_module",
    "MAX_TCP_HEADER",
    "PF_BENTO",
    "AF_BENTO",
    "SS_UNCONNECTED",
    "PF_EXITING",
    "SHUTDOWN_MASK",
    "TCP_CLOSE",
    "CAP_NET_BIND_SERVICE",
    "SOCK_BINDADDR_LOCK",
    "SOCK_BINDPORT_LOCK",
    "SOCK_STREAM",
    "SOCK_RCU_FREE",
];
const OPAQUE_TYPES: &[&str] = &[
    // These need to be opaque because they're both packed and aligned, which rustc
    // doesn't support yet. See https://github.com/rust-lang/rust/issues/59154
    // and https://github.com/rust-lang/rust-bindgen/issues/1538
    "desc_struct",
    "xregs_state",
];

fn handle_kernel_version_cfg(bindings_path: &PathBuf) {
    let f = BufReader::new(fs::File::open(bindings_path).unwrap());
    let mut version = None;
    for line in f.lines() {
        let line = line.unwrap();
        if let Some(type_and_value) = line.split("pub const LINUX_VERSION_CODE").nth(1) {
            if let Some(value) = type_and_value.split('=').nth(1) {
                let raw_version = value.split(';').next().unwrap();
                version = Some(raw_version.trim().parse::<u64>().unwrap());
                break;
            }
        }
    }
    let version = version.expect("Couldn't find kernel version");
    let (major, minor) = match version.to_be_bytes() {
        [0, 0, 0, 0, 0, major, minor, _patch] => (major, minor),
        _ => panic!("unable to parse LINUX_VERSION_CODE {:x}", version),
    };

    if major >= 6 {
        panic!("Please update build.rs with the last 5.x version");
        // Change this block to major >= 7, copy the below block for
        // major >= 6, fill in unimplemented!() for major >= 5
    }
    if major >= 5 {
        for x in 0..=if major > 5 { unimplemented!() } else { minor } {
            println!("cargo:rustc-cfg=kernel_5_{}_0_or_greater", x);
        }
    }
    if major >= 4 {
        // We don't currently support anything older than 4.4
        for x in 4..=if major > 4 { 20 } else { minor } {
            println!("cargo:rustc-cfg=kernel_4_{}_0_or_greater", x);
        }
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

// Takes the CFLAGS from the kernel Makefile and changes all the include paths to be absolute
// instead of relative.
fn prepare_cflags(cflags: &str, kernel_dir: &str) -> Vec<String> {
    let cflag_parts = shlex::split(&cflags).unwrap();
    let mut cflag_iter = cflag_parts.iter();
    let mut kernel_args = vec![];
    while let Some(arg) = cflag_iter.next() {
        if arg.starts_with("-I") && !arg.starts_with("-I/") {
            kernel_args.push(format!("-I{}/{}", kernel_dir, &arg[2..]));
        } else if arg == "-include" {
            kernel_args.push(arg.to_string());
            let include_path = cflag_iter.next().unwrap();
            if include_path.starts_with('/') {
                kernel_args.push(include_path.to_string());
            } else {
                kernel_args.push(format!("{}/{}", kernel_dir, include_path));
            }
        } else {
            kernel_args.push(arg.to_string());
        }
    }
    kernel_args
}

fn main() {
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-env-changed=KDIR");
    println!("cargo:rerun-if-env-changed=c_flags");

    let kernel_dir = env::var("KDIR").expect("Must be invoked from kernel makefile");
    let kernel_cflags = env::var("c_flags").expect("Add 'export c_flags' to Kbuild");
    let kbuild_cflags_module =
        env::var("KBUILD_CFLAGS_MODULE").expect("Must be invoked from kernel makefile");

    let cflags = format!("{} {}", kernel_cflags, kbuild_cflags_module);
    let kernel_args = prepare_cflags(&cflags, &kernel_dir);

    let target = env::var("TARGET").unwrap();

    let mut builder = bindgen::Builder::default()
        .use_core()
        .ctypes_prefix("raw")
        .derive_default(true)
        .size_t_is_usize(true)
        .rustfmt_bindings(true);

    builder = builder.clang_arg(format!("--target={}", target));
    for arg in kernel_args.iter() {
        builder = builder.clang_arg(arg.clone());
    }

    println!("cargo:rerun-if-changed=src/bindings_helper.h");
    builder = builder.header("src/bindings_helper.h");

    for t in INCLUDED_TYPES {
        builder = builder.allowlist_type(t);
    }
    for f in INCLUDED_FUNCTIONS {
        builder = builder.allowlist_function(f);
    }
    for v in INCLUDED_VARS {
        builder = builder.allowlist_var(v);
    }
    for t in OPAQUE_TYPES {
        builder = builder.opaque_type(t);
    }
    let bindings = builder.generate().expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    handle_kernel_version_cfg(&out_path.join("bindings.rs"));
    handle_kernel_symbols_cfg(&PathBuf::from(&kernel_dir).join("Module.symvers"));

    let mut builder = cc::Build::new();
    builder.compiler(env::var("CC").unwrap_or_else(|_| "clang".to_string()));
    builder.target(&target);
    builder.warnings(false);
    println!("cargo:rerun-if-changed=src/helpers.c");
    builder.file("src/helpers.c");
    for arg in kernel_args.iter() {
        builder.flag(&arg);
    }
    builder.compile("helpers");
}

