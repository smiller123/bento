use std::env;
use std::path::{Path, PathBuf};
use std::{thread, time};

mod parser;
mod fs;

fn run_utility() {
    let mut events = Vec::<parser::Event>::new();
    loop {
        // read .lin from mount point
        // parse events
        // update inode_map, produce files to update
        // for each, match on action
        // perform copy/delete
        thread::sleep(time::Duration::from_millis(1000));
    }
}

// main script for backup utility, runs in a loop, fetching updates from .lin, updating
// to backup fs
fn main() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);

    run_utility();
}
