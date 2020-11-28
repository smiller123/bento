use std::error::Error;
use std::path::Path;
use std::path::PathBuf;
use std::fs;
use std::{thread};
use std::sync::mpsc::{self, TryRecvError};

extern crate fs_extra;
use fs_extra::file::{TransitProcess, CopyOptions};

// copies contents of file at file_path to target_dir
// 
// Arguments:
// file_path,  absolute file path on local copy
// source_dir, mount point for local copy
// target_dir,  root directory of remote directory to copy to
pub fn copy(source_path: PathBuf, source_dir: &Path, target_dir: &Path) -> Result<(), Box<dyn Error>> {
    // Create the target directory
    fs_extra::dir::create_all(&target_dir, false).unwrap();
    println!("target_dir: {:?}", target_dir);
    println!("source_path: {:?}", source_path);

    // let target_path = target_dir.join(&source_path);
    let target_path = target_dir.join(source_path.strip_prefix(source_dir).expect("source_dir is not a prefix of given source_path"));
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

    Ok(())
}

pub fn delete(source_path: PathBuf, source_dir: &Path, target_dir: &Path) -> Result<(), Box<dyn Error>> {
    // Check that all files must exist
    let target_path = target_dir.join(source_path.strip_prefix(source_dir).expect("source_dir is not a prefix of given source_path"));

    // Check file existence
    if !target_path.exists() {
        return Err(From::from("delete: file does not exist. This operation won't remove anything."));
    }

    // Check remove permissions
    let perm = fs::metadata(&target_path)?.permissions();
    if perm.readonly() {
        return Err(From::from("delete: no write permission. This operation won't remove anything."));
    }

    // Directory
    if target_path.is_dir() {
        fs_extra::dir::remove(target_path)?;
    // File
    } else {
        fs_extra::file::remove(target_path)?;
    }
    Ok(())
}


