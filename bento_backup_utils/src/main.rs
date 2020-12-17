// Bento Backup Utility
//
// Copyright 2020 Teerapat Jenrungrot, Pat Kosakanchit, Nicholas Monsees
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of this software
// and associated documentation files (the "Software"), to deal in the Software without restriction,
// including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all copies or substantial
// portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT
// LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
// WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE
// SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
//
use std::env;
use std::io;
use std::fs::File;
use std::io::{BufReader, SeekFrom};
use std::io::prelude::*;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs as rust_fs;
use std::time::Instant;

mod analyzer;
mod parser;
mod fs;

const LIN_FILE: &str = ".lin";
const OUTPUT_FILE: &str = ".backup";

pub fn read_file_and_seek(prev_size: u64, file_name: &str) -> Result<(String, u64), io::Error> {
    let mut file = File::open(file_name)?;
    file.seek(SeekFrom::Start(prev_size))?;
    let mut content = Vec::new();
    file.read_to_end(&mut content)?;
    let n_bytes = content.len() as u64;
    let safe_content = String::from_utf8_lossy(&content).into_owned();

    Ok((safe_content, n_bytes))
}

pub fn load_from_file(mount_point: &str) -> (HashMap::<u64, PathBuf>, u64) {
    // Open the file in read-only mode with buffer.
    let mut inode_map = HashMap::<u64, PathBuf>::new();
    inode_map.insert(1, PathBuf::from(mount_point));
    let default = (inode_map, 0);

    let file = File::open(Path::new(mount_point).join(OUTPUT_FILE));
    let file = match file {
        Ok(f) => f,
        _ => return default
    };

    let reader = BufReader::new(file);
    let output = serde_json::from_reader(reader);
    match output {
        Ok((a,b)) => (a, b),
        _ => default,
    }
}

pub fn save_to_file(mount_point: &str, inode_map: HashMap::<u64, PathBuf>, prev_size: u64) -> Result<(), Box<dyn Error>> {
    let output_file = File::create(Path::new(mount_point).join(OUTPUT_FILE))?;
    serde_json::to_writer(output_file, &(inode_map, prev_size))?;
    Ok(())
}

fn run_utility(mount_point: &Path, source_dir: &Path, remote: &Path) {
    assert!(mount_point.is_dir());
    assert!(source_dir.is_dir());
    if !remote.is_dir() {
        rust_fs::create_dir(Path::new(remote)).unwrap();
    }

    let timer = Instant::now();
    let (inode_map, prev_size) = load_from_file(mount_point.to_str().unwrap());
    let duration = timer.elapsed();
    println!("**Time elapsed in load_from_file() is: {:?}", duration);

    let mut inode_map = inode_map;
    let mut prev_size = prev_size;

    // add root to map, since its creation isn't logged
    let lin_file = mount_point.join(LIN_FILE);

    let mut events = Vec::<parser::Event>::new();
    assert!(events.is_empty());

    // This read in only prev_size..lin_size.len() from disk
    let timer = Instant::now();
    let (lin_contents, n_bytes) = read_file_and_seek(prev_size, lin_file.to_str().unwrap()).expect("Unable to read lin file from mount point");
    let duration = timer.elapsed();
    println!("**Time elapsed in read_file_and_seek() is: {:?}", duration);

    println!("Read a string of length {} (n_bytes = {})", lin_contents.len(), n_bytes);

    let timer = Instant::now();
    analyzer::populate_events(&mut events, lin_contents);  // TODO(nmonsees): will print error, need to change in parser
    let duration = timer.elapsed();
    println!("**Time elapsed in populate_events() is: {:?}", duration);

    let timer = Instant::now();
    let files: HashMap<PathBuf, analyzer::Action> = analyzer::files_to_update(&mut inode_map, &events);
    let duration = timer.elapsed();
    println!("**Time elapsed in files_to_update() is: {:?}", duration);

    let base_path = Path::new(source_dir);

    // Copy files
    let update_files = files.iter()
                        .filter(|(f, _)| f.as_path().starts_with(source_dir))
                        .filter(|(_, act)| matches!(act, analyzer::Action::Update))
                        .map(|(f, _)| f.as_path().strip_prefix(base_path).unwrap().display().to_string())
                        .collect();

    let timer = Instant::now();
    if fs::copy(update_files, source_dir, remote).is_err() {
        println!("Warning: Some files can't be copied. This may happen if \
                  you try to copy the files the are already deleted.");
    }
    let duration = timer.elapsed();
    println!("**Time elapsed in fs:copy is: {:?}", duration);

    // Delete files
    let delete_files = files.iter()
                        .filter(|(f, _)| f.as_path().starts_with(source_dir))
                        .filter(|(_, act)| matches!(act, analyzer::Action::Delete))
                        .map(|(f, _)| f.as_path().strip_prefix(base_path).unwrap().display().to_string())
                        .collect();

    let timer = Instant::now();
    if fs::delete(delete_files, remote).is_err() {
        println!("Warning: Some files can't be copied. This may happen if \
                  there is no files to remove at the remote or if \
                  you have no permission.");
    }
    let duration = timer.elapsed();
    println!("**Time elapsed in fs:delete is: {:?}", duration);

    // fs actions will add log entries to .lin, so need to grab prev_size after performing
    // copy/deletes to remote

    // read lin after back up
    prev_size += n_bytes;

    let timer = Instant::now();
    let (_, n_bytes) = read_file_and_seek(prev_size, lin_file.to_str().unwrap()).expect("Unable to read lin file from mount point");
    let duration = timer.elapsed();
    println!("**Time elapsed in read_file_and_seek is: {:?}", duration);
    
    prev_size += n_bytes;
    println!("Last byte read on this run: {:?}", prev_size);

    let timer = Instant::now();
    save_to_file(mount_point.to_str().unwrap(), inode_map, prev_size).unwrap();
    let duration = timer.elapsed();
    println!("**Time elapsed in save_to_file is: {:?}", duration);
}

// main script for backup utility which performs a single backup
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        panic!("Arguments: <path to file system mount point> <path to source dir> <path to remote backup>");
    }

    let mount_point = Path::new(&args[1]);
    let source_dir = Path::new(&args[2]);
    let remote = Path::new(&args[3]);

    run_utility(&mount_point, &source_dir, &remote);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs as rust_fs;
    use std::panic;
    use serial_test::serial;

    const TEST_FOLDER: &'static str = "./tmp";

    fn setup() {
        if Path::new(TEST_FOLDER).exists() {
            rust_fs::remove_dir_all(TEST_FOLDER).unwrap();
        }
        rust_fs::create_dir(Path::new(TEST_FOLDER)).unwrap();
    }

    fn teardown() {
        if Path::new(TEST_FOLDER).exists() {
            rust_fs::remove_dir_all(TEST_FOLDER).unwrap();
        }
    }
    fn run_test<T>(test: T) -> ()
    where
        T: FnOnce() -> () + panic::UnwindSafe
    {
        setup();
        let result = panic::catch_unwind(|| {
            test()
        });
        teardown();
        assert!(result.is_ok())
    }

    #[test]
    #[serial]
    fn test_save_to_file() {
        run_test(||{
            rust_fs::create_dir(Path::new(TEST_FOLDER).join("test_save_to_file")).unwrap();
            let mount_point = Path::new(TEST_FOLDER).join("test_save_to_file");
            let mount_point = mount_point.to_str().unwrap();

            // ok
            let mut inode_map = HashMap::<u64, PathBuf>::new();
            inode_map.insert(1, PathBuf::from(mount_point));
            let res = save_to_file(mount_point, inode_map, 0);
            assert!(res.is_ok());
        });
    }

    #[test]
    #[serial]
    fn test_load_from_file() {
        run_test(||{
            rust_fs::create_dir(Path::new(TEST_FOLDER).join("test_load_from_file")).unwrap();
            let mount_point = Path::new(TEST_FOLDER).join("test_load_from_file");
            let mount_point = mount_point.to_str().unwrap();

            // load from empty file
            let res = load_from_file(mount_point);
            let mut inode_map = HashMap::<u64, PathBuf>::new();
            inode_map.insert(1, PathBuf::from(mount_point));
            assert_eq!(inode_map.len(), res.0.len()); // compare length
            assert!(inode_map.keys().all(|k| { res.0.contains_key(k) & (inode_map.get(k).unwrap() == res.0.get(k).unwrap()) }));
            assert_eq!(0, res.1);

            // save to file
            let mut inode_map = HashMap::<u64, PathBuf>::new();
            inode_map.insert(1, PathBuf::from(mount_point));
            inode_map.insert(100, PathBuf::from("hello"));
            let res = save_to_file(mount_point, inode_map, 100);
            assert!(res.is_ok());

            // load from file
            let mut inode_map = HashMap::<u64, PathBuf>::new();
            inode_map.insert(1, PathBuf::from(mount_point));
            inode_map.insert(100, PathBuf::from("hello"));
            let res = load_from_file(mount_point);
            assert_eq!(inode_map, res.0);
            assert!(inode_map.keys().all(|k| { res.0.contains_key(k) & (inode_map.get(k).unwrap() == res.0.get(k).unwrap()) }));
            assert_eq!(100, res.1);
        });
    }

    #[test]
    #[serial]
    fn test_read_file_and_seek() {
        run_test(||{
            rust_fs::create_dir(Path::new(TEST_FOLDER).join("test_read_file_and_seek")).unwrap();
            let file_path = Path::new(TEST_FOLDER).join("test_read_file_and_seek").join(OUTPUT_FILE);

            // check file doesn't exist
            let ret = file_path.to_str().unwrap();
            assert!(!Path::new(ret).exists());

            // expect error when reading non-exist file.
            let ret = read_file_and_seek(0, file_path.to_str().unwrap());
            assert!(ret.is_err());

            // simple case
            rust_fs::write(file_path.to_str().unwrap(), b"abcdef").unwrap();
            let ret = read_file_and_seek(0, file_path.to_str().unwrap());
            assert!(ret.is_ok());
            let (output, n_bytes) = ret.unwrap();
            assert_eq!(output, "abcdef");
            assert_eq!(n_bytes, 6);

            // simple with shift
            rust_fs::write(file_path.to_str().unwrap(), b"abcdef").unwrap();
            let ret = read_file_and_seek(3, file_path.to_str().unwrap());
            assert!(ret.is_ok());
            let (output, n_bytes) = ret.unwrap();
            assert_eq!(output, "def");
            assert_eq!(n_bytes, 3);

            // multiple lines
            rust_fs::write(file_path.to_str().unwrap(), b"\n\n1234\n5678\n\n").unwrap();
            let ret = read_file_and_seek(0, file_path.to_str().unwrap());
            assert!(ret.is_ok());
            let (output, n_bytes) = ret.unwrap();
            assert_eq!(output, "\n\n1234\n5678\n\n");
            assert_eq!(n_bytes, 13);

            // multiple lines with shift
            rust_fs::write(file_path.to_str().unwrap(), b"\n\n1234\n5678\n\n").unwrap();
            let ret = read_file_and_seek(2, file_path.to_str().unwrap());
            assert!(ret.is_ok());
            let (output, n_bytes) = ret.unwrap();
            assert_eq!(output, "1234\n5678\n\n");
            assert_eq!(n_bytes, 11);

            // weird character
            let input_string = vec![72, 101, 108, 108, 111, 32, 240, 144, 128, 87, 111, 114, 108, 100];
            rust_fs::write(file_path.to_str().unwrap(), input_string).unwrap();
            let ret = read_file_and_seek(0, file_path.to_str().unwrap());
            assert!(ret.is_ok());
            let (output, n_bytes) = ret.unwrap();
            assert_eq!(output, "Hello �World");
            let ref_output_bytes: [u8; 14] = [72, 101, 108, 108, 111, 32, 239, 191, 189, 87, 111, 114, 108, 100];
            assert_eq!(output.as_bytes(), &ref_output_bytes);
            assert_eq!(n_bytes, 14);

            // weird character with shift
            //                      H   e    l    l    o   <sp> <err><err><err><err> W  o    r    l    d
            let input_string = vec![72, 101, 108, 108, 111, 32, 240, 144, 128, 254, 87, 111, 114, 108, 100];
            rust_fs::write(file_path.to_str().unwrap(), input_string).unwrap();
            let ret = read_file_and_seek(6, file_path.to_str().unwrap());
            assert!(ret.is_ok());
            let (output, n_bytes) = ret.unwrap();
            assert_eq!(output, "��World");
            let ref_output_bytes: [u8; 11] = [239, 191, 189, 239, 191, 189, 87, 111, 114, 108, 100];
            assert_eq!(output.as_bytes(), &ref_output_bytes);
            assert_eq!(n_bytes, 9);
        });
    }
}