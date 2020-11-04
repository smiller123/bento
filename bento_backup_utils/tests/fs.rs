// cargo build && cargo test -- --test-threads=1 --nocapture

use std::error::Error;
use std::path::{Path};
use std::panic;
use std::fs as rust_fs;

extern crate fs_extra;

#[path = "../src/fs.rs"] mod fs;

const TEST_FOLDER: &'static str = "./tmp";

#[allow(dead_code)]
fn setup() {
    if Path::new(TEST_FOLDER).exists() {
        rust_fs::remove_dir_all(TEST_FOLDER).unwrap();
    }
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
        fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_file_eq"), false).unwrap();

        let content1 = "content";
        fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_file_eq/test1.txt"), &content1).unwrap();
        assert!(Path::new(TEST_FOLDER).join("test_file_eq/test1.txt").exists());
        let content2 = "content";
        fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_file_eq/test2.txt"), &content2).unwrap();
        assert!(Path::new(TEST_FOLDER).join("test_file_eq/test2.txt").exists());

        let read1 = fs_extra::file::read_to_string(&Path::new(TEST_FOLDER).join("test_file_eq/test1.txt")).unwrap();
        assert_eq!(read1, content1);
        let read2 = fs_extra::file::read_to_string(&Path::new(TEST_FOLDER).join("test_file_eq/test2.txt")).unwrap();
        assert_eq!(read2, content2);

        assert!(file_eq(Path::new(TEST_FOLDER).join("test_file_eq/test1.txt"), Path::new(TEST_FOLDER).join("test_file_eq/test2.txt")).unwrap());
    })
}

// This test checks if calling copy would create a folder or not.
#[test]
fn test_copy_no_files() {
    run_test(||{
        // Check the folders before copy
        assert!(!Path::new(TEST_FOLDER).join("test_copy_no_files/src").exists());
        assert!(!Path::new(TEST_FOLDER).join("test_copy_no_files/target").exists());

        // run copy and check ok
        let file_list = Vec::new();
        let result = fs::copy(file_list, &Path::new(TEST_FOLDER).join("test_copy_no_files/src"), &Path::new(TEST_FOLDER).join("test_copy_no_files/target"));
        assert!(result.is_ok());

        // Check the folders after copy
        assert!(!Path::new(TEST_FOLDER).join("test_copy_no_files/src").exists());
        assert!(Path::new(TEST_FOLDER).join("test_copy_no_files/target").exists());
    })
}

// This test checks if calling copy to an existing folder would
// cause an error or not.
#[test]
fn test_copy_to_existing_folder() {
    run_test(||{
        // create folder and check if there's no folder
        assert!(!Path::new(TEST_FOLDER).join("test_copy_to_existing_folder/target").exists());
        fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_folder/target"), false).unwrap();

        // check the folders before copy
        assert!(!Path::new(TEST_FOLDER).join("test_copy_to_existing_folder/src").exists());
        assert!(Path::new(TEST_FOLDER).join("test_copy_to_existing_folder/target").exists());

        // run copy and expect an error
        let file_list = Vec::new();
        let result = fs::copy(file_list, &Path::new(TEST_FOLDER).join("test_copy_to_existing_folder/src"), &Path::new(TEST_FOLDER).join("test_copy_to_existing_folder/target"));
        assert!(result.is_ok());

        // Check the folders after copy
        assert!(!Path::new(TEST_FOLDER).join("test_copy_to_existing_folder/src").exists());
        assert!(Path::new(TEST_FOLDER).join("test_copy_to_existing_folder/target").exists());
    })
}

#[test]
fn test_simple_copy() {
    run_test(||{
        // create source directory
        fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_simple_copy/src"), false).unwrap();

        // create test1.txt
        let content1 = "content";
        fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_simple_copy/src/test1.txt"), &content1).unwrap();

        // create test2.txt
        let content2 = "content";
        fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_simple_copy/src/test2.txt"), &content2).unwrap();

        // create file list
        let mut file_list = Vec::new();
        let path1 = "test1.txt";
        let path2 = "test2.txt";
        file_list.push(path2.to_string());
        file_list.push(path1.to_string());

        // run copy and expect an error
        let result = fs::copy(file_list, &Path::new(TEST_FOLDER).join("test_simple_copy/src"), &Path::new(TEST_FOLDER).join("test_simple_copy/target"));
        assert!(result.is_ok());

        // check target files exist
        let path1_target = Path::new(TEST_FOLDER).join("test_simple_copy/target/test1.txt");
        let path2_target = Path::new(TEST_FOLDER).join("test_simple_copy/target/test2.txt");
        assert!(path1_target.exists());
        assert!(path2_target.exists());

        // compare contents
        let path1_src = Path::new(TEST_FOLDER).join("test_simple_copy/src/test1.txt");
        let path2_src = Path::new(TEST_FOLDER).join("test_simple_copy/src/test2.txt");
        assert!(file_eq(path1_src, path1_target).unwrap());
        assert!(file_eq(path2_src, path2_target).unwrap());
    })
}

#[test]
fn test_copy_nested_folder1() {
    run_test(||{
        // create source directory
        fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_nested_folder1/src"), false).unwrap();

        // create test1.txt
        let content1 = "content";
        fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_nested_folder1/src/folder1"), false).unwrap();
        fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_copy_nested_folder1/src/folder1/test1.txt"), &content1).unwrap();

        // create test2.txt
        let content2 = "content";
        fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_copy_nested_folder1/src/test2.txt"), &content2).unwrap();

        // create file list
        let mut file_list = Vec::new();
        let path1 = "folder1/test1.txt";
        let path2 = "test2.txt";
        file_list.push(path2.to_string());
        file_list.push(path1.to_string());

        // run copy and expect an error
        let result = fs::copy(file_list, &Path::new(TEST_FOLDER).join("test_copy_nested_folder1/src"), &Path::new(TEST_FOLDER).join("test_copy_nested_folder1/target"));
        assert!(result.is_ok());

        // check target files exist
        let path1_target = Path::new(TEST_FOLDER).join("test_copy_nested_folder1/target/folder1/test1.txt");
        let path2_target = Path::new(TEST_FOLDER).join("test_copy_nested_folder1/target/test2.txt");
        assert!(path1_target.exists());
        assert!(path2_target.exists());

        // compare contents
        let path1_src = Path::new(TEST_FOLDER).join("test_copy_nested_folder1/src/folder1/test1.txt");
        let path2_src = Path::new(TEST_FOLDER).join("test_copy_nested_folder1/src/test2.txt");
        assert!(file_eq(path1_src, path1_target).unwrap());
        assert!(file_eq(path2_src, path2_target).unwrap());
    })
}

#[test]
fn test_copy_nested_folder2() {
    run_test(||{
        // create source directory
        fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_nested_folder2/src"), false).unwrap();

        // create test1.txt
        let content1 = "content";
        fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_nested_folder2/src/folder1/folder1_1"), false).unwrap();
        fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_copy_nested_folder2/src/folder1/folder1_1/test1.txt"), &content1).unwrap();

        // create test2.txt
        let content2 = "content";
        fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_nested_folder2/src/folder2"), false).unwrap();
        fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_copy_nested_folder2/src/folder2/test2.txt"), &content2).unwrap();

        // create an empty directory
        fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_nested_folder2/src/folder3"), false).unwrap();

        // create file list
        let mut file_list = Vec::new();
        file_list.push("folder1/folder1_1/test1.txt".to_string());
        file_list.push("folder2/test2.txt".to_string());
        file_list.push("folder3/".to_string());

        // run copy and expect an error
        let result = fs::copy(file_list, &Path::new(TEST_FOLDER).join("test_copy_nested_folder2/src"), &Path::new(TEST_FOLDER).join("test_copy_nested_folder2/target"));
        assert!(result.is_ok());

        // check target files exist
        let path1_target = Path::new(TEST_FOLDER).join("test_copy_nested_folder2/target/folder1/folder1_1/test1.txt");
        let path2_target = Path::new(TEST_FOLDER).join("test_copy_nested_folder2/target/folder2/test2.txt");

        assert!(path1_target.exists());
        assert!(path2_target.exists());

        // check target empty directory
        assert!(Path::new(TEST_FOLDER).join("test_copy_nested_folder2/target/folder3").exists());
        assert!(Path::new(TEST_FOLDER).join("test_copy_nested_folder2/target/folder3").is_dir());

        // compare contents
        let path1_src = Path::new(TEST_FOLDER).join("test_copy_nested_folder2/src/folder1/folder1_1/test1.txt");
        let path2_src = Path::new(TEST_FOLDER).join("test_copy_nested_folder2/src/folder2/test2.txt");
        assert!(file_eq(path1_src, path1_target).unwrap());
        assert!(file_eq(path2_src, path2_target).unwrap());
    })
}

#[test]
fn test_copy_to_existing_nested_folder() {
    run_test(||{
        // create source directory
        fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target"), false).unwrap();

        // create test1.txt
        let content1 = "content";
        fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/src/folder1/folder1_1"), false).unwrap();
        fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target/folder1/folder1_1"), false).unwrap();
        fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/src/folder1/folder1_1/test1.txt"), &content1).unwrap();
        fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target/folder1/folder1_1/test1.txt"), &content1).unwrap();

        // create test2.txt
        let content2 = "content";
        fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/src/folder2"), false).unwrap();
        fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target/folder2"), false).unwrap();
        fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/src/folder2/test2.txt"), &content2).unwrap();
        fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target/folder2/test2.txt"), &content2).unwrap();

        // create an empty directory
        fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/src/folder3"), false).unwrap();
        fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target/folder3"), false).unwrap();

        // create file list
        let mut file_list = Vec::new();
        file_list.push("folder1/folder1_1/test1.txt".to_string());
        file_list.push("folder2/test2.txt".to_string());
        file_list.push("folder3/".to_string());

        // run copy and expect an error
        let result = fs::copy(file_list, &Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/src"), &Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target"));
        assert!(result.is_ok());

        // check target files exist
        let path1_target = Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target/folder1/folder1_1/test1.txt");
        let path2_target = Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target/folder2/test2.txt");

        assert!(path1_target.exists());
        assert!(path2_target.exists());

        // check target empty directory
        assert!(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target/folder3").exists());
        assert!(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target/folder3").is_dir());

        // compare contents
        let path1_src = Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/src/folder1/folder1_1/test1.txt");
        let path2_src = Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/src/folder2/test2.txt");
        assert!(file_eq(path1_src, path1_target).unwrap());
        assert!(file_eq(path2_src, path2_target).unwrap());
    })
}