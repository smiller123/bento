use std::env;
use std::path::{Path, PathBuf};
use std::{thread, time};
use std::collections::HashMap;


mod parser;
mod fs;

fn run_utility(mount_point: &Path, remote: &Path, scan_frequency: &u64) {
    let inode_map = HashMap::<u64, PathBuf>::new();
    // assert!(mount_point.is_dir());
    // assert!(remote.is_dir());
    let lin_file = mount_point.join(".lin");
    loop {
        println!("in loop!");
        // println!("mount_point to_str {:?}", mount_point.to_str().unwrap());
        let mut events = Vec::<parser::Event>::new();
        assert!(events.len() == 0);

        // TODO(nmonsees): eventually this should just return the subset based on prev size
        let lin_contents = parser::read_lin_file(lin_file.to_str().unwrap()).expect("Unable to read lin file from mount point");
        
        thread::sleep(time::Duration::from_millis(*scan_frequency));
    }
}

// main script for backup utility, runs in a loop, fetching updates from .lin, updating
// to backup fs
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        panic!("Arguments: <path to file system mount point> <path to remote backup> <backup frequency in milliseconds>");
    }

    let mount_point = Path::new(&args[1]);
    let remote = Path::new(&args[2]);
    let scan_frequency = &args[3].parse::<u64>().unwrap();

    run_utility(&mount_point, &remote, &scan_frequency);
}
