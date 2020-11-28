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
    let mut prev_size: usize = 0;  // index to read new .lin contents from
    loop {
        println!("in loop!");
        let mut events = Vec::<parser::Event>::new();
        assert!(events.len() == 0);

        // TODO(nmonsees): ideally this should read in only prev_size..lin_size.len() from disk, but I can't find
        // an fs function to do that
        let lin_contents = parser::read_lin_file(lin_file.to_str().unwrap()).expect("Unable to read lin file from mount point");
        let lin_slice = lin_contents.get(prev_size..lin_contents.len()).unwrap();  // new events from previous scan

        println!("lin_contents len: {}", lin_contents.len());
        println!("lin_contents len from parser: {}", parser::get_lin_size(lin_file.to_str().unwrap()));
        println!("lin_slice len: {}", lin_slice.len());

        parser::populate_events(&mut events, String::from(lin_slice));  // TODO(nmonsees): will print error, need to change in parser
        parser::update_inode_map(&mut inode_map, &events);

        let files: HashMap<PathBuf, parser::Action> = parser::files_to_update(&inode_map, &events);

        for (file, action) in &files {
            match action {
                // TODO(nmonsees): unsure how to avoid the file clone here, passing a ref doesn't
                // guarantee lifetime, but this does seem not great
                parser::Action::Update => { fs::copy(file.clone(), mount_point, remote).expect("Unable to perform copy to remote"); },
                parser::Action::Delete => { fs::delete(file.clone(), mount_point, remote).expect("Unable to perform deletion from remote"); },
            }
        }


        // fs actions will add log entries to .lin, so need to grab prev_size after performing
        // copy/deletes to remote

        // prev_size = parser::get_lin_size(lin_file.to_str().unwrap()) as usize;
        // println!("prev_size metadata: {}", prev_size);

        // TODO(nmonsees): it turns out that the metadata isn't immediately synchronized after
        // writes to the log, so the hack in place here is just to read in the whole contents from
        // disk again, and grab the len
        //
        // This works, but it would be great to have a sync on the .lin to just fetch the metadata
        let lin_post_contents = parser::read_lin_file(lin_file.to_str().unwrap()).expect("Unable to read lin file from mount point");
        prev_size = lin_post_contents.len();
        println!("prev_size from file: {}", prev_size);

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
