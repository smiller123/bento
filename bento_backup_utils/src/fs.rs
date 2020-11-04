use std::error::Error;
// use std::io;
// use std::fs;
use std::path::Path;
use std::{thread};
use std::sync::mpsc::{self, TryRecvError};

extern crate fs_extra;
use fs_extra::file::{TransitProcess, CopyOptions};

#[allow(dead_code)]
pub fn copy(file_list: Vec<String>, base_dir: &Path, target_dir: &Path) -> Result<(), Box<dyn Error>> {
    // Create the target directory
    fs_extra::dir::create_all(&target_dir, false).unwrap();

    for path in file_list.iter() {
        let source_path = base_dir.join(path);
        let target_path = target_dir.join(path);
        println!("Copy from {:?} ==> to {:?}", source_path.to_str(), target_path.to_str());

        // If the source path is a directory, simply create a directory.
        if source_path.is_dir() {
            fs_extra::dir::create_all(&target_path, false).unwrap();

        // Otherwise, copy/overwrite the file.
        } else {
            let target_parent_dir = target_path.parent().unwrap();
            fs_extra::dir::create_all(&target_parent_dir, false).unwrap();

            let options = CopyOptions {
                overwrite: true,
                skip_exist: false,
                buffer_size: 1,
            };
            let (tx, rx) = mpsc::channel();
            thread::spawn(move || {
                let handler = |process_info: TransitProcess| {
                    tx.send(process_info).unwrap();
                    // thread::sleep(time::Duration::from_millis(500));
                    // fs_extra::dir::TransitProcessResult::ContinueOrAbort;
                };
                fs_extra::file::copy_with_progress(&source_path, &target_path, &options, handler).unwrap();
            });

            loop {
                match rx.try_recv() {
                    Ok(process_info) => {
                        println!("{} of {} bytes",
                                process_info.copied_bytes,
                                process_info.total_bytes);
                    }
                    Err(TryRecvError::Disconnected) => {
                        println!("finished");
                        break;
                    }
                    Err(TryRecvError::Empty) => {}
                }
            }
        }
    }

    Ok(())
}
