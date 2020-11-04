// cargo build && cargo test -- --test-threads=1 --nocapture

use std::error::Error;
use std::path::{Path, PathBuf};
use std::panic;
use std::fs as rust_fs;

extern crate fs_extra;

#[path = "../src/fs.rs"] mod fs;

const TEST_FOLDER: &'static str = "./tmp";

#[allow(dead_code)]
fn setup() {
    rust_fs::remove_dir_all(TEST_FOLDER).unwrap();
}

#[allow(dead_code)]
fn teardown() {
    rust_fs::remove_dir_all(TEST_FOLDER).unwrap();
}

fn run_test<T>(test: T) -> ()
where
    T: FnOnce() -> () + panic::UnwindSafe
{
    setup();
    let result = panic::catch_unwind(|| {
        test()
    });
    // teardown();
    assert!(result.is_ok())
}

fn file_eq<P,Q>(file1: P, file2: Q) -> Result<bool, Box<dyn Error>>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let content1 = fs_extra::file::read_to_string(file1)?;
    let content2 = fs_extra::file::read_to_string(file2)?;
    Ok(content1 == content2)
}

// This test creates simple files test1.txt and test2.txt inside a folder
// named test_file_eq with the same content. It checks if two files have
// the same content.
#[test]
fn test_file_eq() {
    run_test(||{
        let mut test_file1 = PathBuf::from(TEST_FOLDER);
        test_file1.push("test_file_eq");
        test_file1.push("test1.txt");
        let mut test_file2 = PathBuf::from(TEST_FOLDER);
        test_file2.push("test_file_eq");
        test_file2.push("test2.txt");
        fs_extra::dir::create_all(test_file1.parent().unwrap(), true).unwrap();
        fs_extra::dir::create_all(test_file2.parent().unwrap(), true).unwrap();

        let content1 = "content";
        fs_extra::file::write_all(&test_file1, &content1).unwrap();
        assert!(test_file1.exists());
        let content2 = "content";
        fs_extra::file::write_all(&test_file2, &content2).unwrap();
        assert!(test_file2.exists());

        let read1 = fs_extra::file::read_to_string(&test_file1).unwrap();
        assert_eq!(read1, content1);
        let read2 = fs_extra::file::read_to_string(&test_file2).unwrap();
        assert_eq!(read2, content2);

        assert!(file_eq(test_file1, test_file2).unwrap());
    })
}

// This test checks if calling copy would create a folder or not.
#[test]
fn test_copy_no_files() {
    run_test(||{
        let file_list = Vec::new();
        let mut target_dir = PathBuf::from(TEST_FOLDER);
        target_dir.push("test_copy_no_files");

        // Get base dir
        let mut base_dir = PathBuf::from(TEST_FOLDER);
        base_dir.push("test_copy_no_files");
        base_dir.push("src");
        assert!(!target_dir.exists()); // check there's a folder

        // Check if target_dir folder doesn't exist
        let mut target_dir = PathBuf::from(TEST_FOLDER);
        target_dir.push("test_copy_no_files");
        target_dir.push("target");
        assert!(!target_dir.exists()); // check there's a folder

        // run copy and check ok
        let result = fs::copy(file_list, &base_dir, &target_dir);
        assert!(result.is_ok());

        // Check if target_dir folder exists
        let mut target_dir = PathBuf::from(TEST_FOLDER);
        target_dir.push("test_copy_no_files");
        target_dir.push("target");
        assert!(target_dir.exists()); // check there's a folder
    })
}

// This test checks if calling copy to an existing folder would
// cause an error or not.
#[test]
fn test_copy_to_existing_folder() {
    run_test(||{
        // Get base dir
        let mut base_dir = PathBuf::from(TEST_FOLDER);
        base_dir.push("test_copy_to_existing_folder");
        base_dir.push("src");

        let file_list = Vec::new();
        let mut target_dir = PathBuf::from(TEST_FOLDER);
        target_dir.push("test_copy_to_existing_folder");
        target_dir.push("target");

        // create folder and check if there's no folder
        assert!(!target_dir.exists());
        fs_extra::dir::create_all(target_dir, true).unwrap();

        // check there's a folder
        let mut target_dir = PathBuf::from(TEST_FOLDER);
        target_dir.push("test_copy_to_existing_folder");
        target_dir.push("target");
        assert!(target_dir.exists());

        // run copy and expect an error
        let mut target_dir = PathBuf::from(TEST_FOLDER);
        target_dir.push("test_copy_to_existing_folder");
        let result = fs::copy(file_list, &base_dir, &target_dir);
        assert!(result.is_err());
    })
}

#[test]
fn test_sample_copy1() {
    run_test(||{
        // Get base dir
        let mut base_dir = PathBuf::from(TEST_FOLDER);
        base_dir.push("test_sample_copy1");
        base_dir.push("src");

        // create source directory
        let mut src = PathBuf::from(TEST_FOLDER);
        src.push("test_sample_copy1");
        src.push("src");
        fs_extra::dir::create_all(src, true).unwrap();

        // create test1.txt
        let content1 = "content";
        let mut test1 = PathBuf::from(TEST_FOLDER);
        test1.push("test_sample_copy1");
        test1.push("src");
        test1.push("test1.txt");
        fs_extra::file::write_all(&test1, &content1).unwrap();

        // create test2.txt
        let content2 = "content";
        let mut test2 = PathBuf::from(TEST_FOLDER);
        test2.push("test_sample_copy1");
        test2.push("src");
        test2.push("test2.txt");
        fs_extra::file::write_all(&test2, &content2).unwrap();

        // create file list
        let mut file_list = Vec::new();
        let path1 = "test1.txt";
        let path2 = "test2.txt";
        file_list.push(path2.to_string());
        file_list.push(path1.to_string());
        println!("file_list = {:?}", file_list);

        // run copy and expect an error
        let mut target_dir = PathBuf::from(TEST_FOLDER);
        target_dir.push("test_sample_copy1");
        target_dir.push("target");
        let result = fs::copy(file_list, &base_dir, &target_dir);
        assert!(result.is_ok());

        // check target files exist
        let path1_target = Path::new(TEST_FOLDER).join("test_sample_copy1/target/test1.txt");
        let path2_target = Path::new(TEST_FOLDER).join("test_sample_copy1/target/test2.txt");
        assert!(path1_target.exists());
        assert!(path2_target.exists());

        // compare contents
        let path1_src = Path::new(TEST_FOLDER).join("test_sample_copy1/src/test1.txt");
        let path2_src = Path::new(TEST_FOLDER).join("test_sample_copy1/src/test2.txt");
        assert!(file_eq(path1_src, path1_target).unwrap());
        assert!(file_eq(path2_src, path2_target).unwrap());
    })
}