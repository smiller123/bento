use std::env;
use std::path::{Path, PathBuf};
use std::{thread, time};
use std::collections::HashMap;


mod parser;
mod fs;

fn run_utility(mount_point: &Path, remote: &Path, scan_frequency: &u64) {
    assert!(mount_point.is_dir());
    assert!(remote.is_dir());

    let mut inode_map = HashMap::<u64, PathBuf>::new();

    // add root to map, since its creation isn't logged
    inode_map.insert(1, PathBuf::from(mount_point));

    let lin_file = mount_point.join(".lin");
    loop {
        println!("in loop!");
        let mut events = Vec::<parser::Event>::new();
        assert!(events.len() == 0);

        // TODO(nmonsees): eventually this should just return the subset based on prev size
        let lin_contents = parser::read_lin_file(lin_file.to_str().unwrap()).expect("Unable to read lin file from mount point");
        parser::populate_events(&mut events, lin_contents);  // TODO(nmonsees): will print error, need to change in parser

        //events.iter().for_each(|e| { println!("event! {:?}", e) });

        parser::update_inode_map(&mut inode_map, &events);
        for (key, value) in &inode_map {
            println!("{}: {}", key, value.display());
        }

        let files = parser::files_to_update(&inode_map, &events);
        for (key, value) in &files {
            println!("{}: {:?}", key.to_str().unwrap(), value);
        }

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
