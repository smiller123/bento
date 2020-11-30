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
use std::error::Error;
use std::path::Path;
use std::fs;

extern crate fs_extra;

pub fn copy(file_list: Vec<String>, base_dir: &Path, target_dir: &Path) -> Result<(), Box<dyn Error>> {
    // Create the target directory
    fs_extra::dir::create_all(&target_dir, false).unwrap();

    for path in file_list.iter() {
        let source_path = base_dir.join(path);
        let target_path = target_dir.join(path);
        // println!("Copy from {:?} ==> to {:?}", source_path.to_str(), target_path.to_str());

        // If the source path is a directory, simply create a directory.
        if source_path.is_dir() {
            fs_extra::dir::create_all(&target_path, false).unwrap();

            let options = fs_extra::dir::CopyOptions {
                overwrite: true,
                skip_exist: false,
                buffer_size: 64000,
                copy_inside: true,
                content_only: true,
                depth: 0,
            };
            if fs_extra::dir::copy(&source_path, &target_path, &options).is_err() {
                println!("Error in copying directory")
            }

        // Otherwise, copy/overwrite the file.
        } else {
            let target_parent_dir = target_path.parent().unwrap();
            fs_extra::dir::create_all(&target_parent_dir, false).unwrap();

            let options = fs_extra::file::CopyOptions {
                overwrite: true,
                skip_exist: false,
                buffer_size: 64000,
            };
            if fs_extra::file::copy(&source_path, &target_path, &options).is_err() {
                println!("Error in copying file")
            }
        }
    }

    Ok(())
}

pub fn delete(file_list: Vec<String>, target_dir: &Path) -> Result<(), Box<dyn Error>> {
    // Check that all files must exist
    for path in file_list.iter() {
        let target_path = target_dir.join(path);

        if target_path.exists() {
            // Check remove permissions
            let perm = fs::metadata(target_path)?.permissions();
            if perm.readonly() {
                return Err(From::from("delete: no write permission. This operation won't remove anything."));
            }
        }
    }

    // Do the actual remove
    for path in file_list.iter() {
        let target_path = target_dir.join(path);
        // println!("Remove {:?}", target_path.to_str());

        // Directory
        if target_path.is_dir() {
            fs_extra::dir::remove(target_path)?;
        // File
        } else {
            fs_extra::file::remove(target_path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic;
    use serial_test::serial;

    const TEST_FOLDER: &'static str = "./tmp";

    fn setup() {
        if Path::new(TEST_FOLDER).exists() {
            fs::remove_dir_all(TEST_FOLDER).unwrap();
        }
    }

    fn teardown() {
        if Path::new(TEST_FOLDER).exists() {
            fs::remove_dir_all(TEST_FOLDER).unwrap();
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
    #[serial]
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
    #[serial]
    fn test_copy_no_files() {
        run_test(||{
            // Check the folders before copy
            assert!(!Path::new(TEST_FOLDER).join("test_copy_no_files/src").exists());
            assert!(!Path::new(TEST_FOLDER).join("test_copy_no_files/target").exists());

            // run copy and check ok
            let file_list = Vec::new();
            let result = copy(file_list, &Path::new(TEST_FOLDER).join("test_copy_no_files/src"), &Path::new(TEST_FOLDER).join("test_copy_no_files/target"));
            assert!(result.is_ok());

            // Check the folders after copy
            assert!(!Path::new(TEST_FOLDER).join("test_copy_no_files/src").exists());
            assert!(Path::new(TEST_FOLDER).join("test_copy_no_files/target").exists());
        })
    }

    // This test checks if calling copy to an existing folder would
    // cause an error or not.
    #[test]
    #[serial]
    fn test_copy_to_existing_folder() {
        run_test(||{
            // create folder and check if there's no folder
            assert!(!Path::new(TEST_FOLDER).join("test_copy_to_existing_folder/target").exists());
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_folder/target"), false).unwrap();

            // check the folders before copy
            assert!(!Path::new(TEST_FOLDER).join("test_copy_to_existing_folder/src").exists());
            assert!(Path::new(TEST_FOLDER).join("test_copy_to_existing_folder/target").exists());

            // run copy and expect ok
            let file_list = Vec::new();
            let result = copy(file_list, &Path::new(TEST_FOLDER).join("test_copy_to_existing_folder/src"), &Path::new(TEST_FOLDER).join("test_copy_to_existing_folder/target"));
            assert!(result.is_ok());

            // Check the folders after copy
            assert!(!Path::new(TEST_FOLDER).join("test_copy_to_existing_folder/src").exists());
            assert!(Path::new(TEST_FOLDER).join("test_copy_to_existing_folder/target").exists());
        })
    }

    #[test]
    #[serial]
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

            // run copy and expect ok
            let result = copy(file_list, &Path::new(TEST_FOLDER).join("test_simple_copy/src"), &Path::new(TEST_FOLDER).join("test_simple_copy/target"));
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
    #[serial]
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

            // run copy and expect ok
            let result = copy(file_list, &Path::new(TEST_FOLDER).join("test_copy_nested_folder1/src"), &Path::new(TEST_FOLDER).join("test_copy_nested_folder1/target"));
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
    #[serial]
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

            // run copy and expect ok
            let result = copy(file_list, &Path::new(TEST_FOLDER).join("test_copy_nested_folder2/src"), &Path::new(TEST_FOLDER).join("test_copy_nested_folder2/target"));
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
    #[serial]
    fn test_copy_to_existing_nested_folder() {
        run_test(||{
            // create source directory
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/src"), false).unwrap();
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target"), false).unwrap();

            // create test1.txt
            let content1 = "content";
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/src/folder1/folder1_1"), false).unwrap();
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target/folder1/folder1_1"), false).unwrap();
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/src/folder1/folder1_1/test1.txt"), &content1).unwrap();
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target/folder1/folder1_1/test1.txt"), &content1).unwrap();

            // create test2.txt
            let content2 = "content";
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/src/folder2/"), false).unwrap();
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target/folder2/"), false).unwrap();
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

            // run copy and expect ok
            let result = copy(file_list, &Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/src"), &Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target"));
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

    #[test]
    #[serial]
    fn test_overwrite() {
        run_test(||{
            // create source directory
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_overwrite/src"), false).unwrap();
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_overwrite/target"), false).unwrap();

            // create test1.txt
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_overwrite/src/test.txt"), &"new_content").unwrap();
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_overwrite/target/test.txt"), &"old_content").unwrap();

            // create file list
            let mut file_list = Vec::new();
            file_list.push("test.txt".to_string());

            // run copy and expect ok
            let result = copy(file_list, &Path::new(TEST_FOLDER).join("test_overwrite/src"), &Path::new(TEST_FOLDER).join("test_overwrite/target"));
            assert!(result.is_ok());

            // check target files exist
            let path_target = Path::new(TEST_FOLDER).join("test_overwrite/target/test.txt");
            assert!(path_target.exists());

            // compare contents
            let path_src = Path::new(TEST_FOLDER).join("test_overwrite/src/test.txt");
            assert!(file_eq(path_src, path_target).unwrap());
        })
    }

    #[test]
    #[serial]
    fn test_overwrite_to_existing_nested_folder() {
        run_test(||{
            // create source directory
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/src"), false).unwrap();
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target"), false).unwrap();

            // create test1.txt
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/src/folder1/folder1_1"), false).unwrap();
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target/folder1/folder1_1"), false).unwrap();
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/src/folder1/folder1_1/test1.txt"), &"new_content1").unwrap();
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target/folder1/folder1_1/test1.txt"), &"old_content1").unwrap();

            // create test2.txt
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/src/folder2/"), false).unwrap();
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target/folder2/"), false).unwrap();
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/src/folder2/test2.txt"), &"new_content2").unwrap();
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target/folder2/test2.txt"), &"old_content2").unwrap();

            // create an empty directory
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/src/folder3"), false).unwrap();
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target/folder3"), false).unwrap();

            // create file list
            let mut file_list = Vec::new();
            file_list.push("folder1/folder1_1/test1.txt".to_string());
            file_list.push("folder2/test2.txt".to_string());
            file_list.push("folder3/".to_string());

            // run copy and expect ok
            let result = copy(file_list, &Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/src"), &Path::new(TEST_FOLDER).join("test_copy_to_existing_nested_folder/target"));
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

    #[test]
    #[serial]
    fn test_remove_no_target() {
        run_test(||{
            // create target directory
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_remove_no_target/target"), false).unwrap();

            // create test1.txt
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_remove_no_target/target/test1.txt"), &"content1").unwrap();

            // create test2.txt
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_remove_no_target/target/test2.txt"), &"content2").unwrap();

            // create file list
            let mut file_list = Vec::new();
            file_list.push("test1.txt".to_string());
            file_list.push("test2.txt".to_string());
            file_list.push("test3.txt".to_string());

            // run delete and expect err
            let result = delete(file_list, &Path::new(TEST_FOLDER).join("test_remove_no_target/target"));
            assert!(result.is_ok());

            // check target files' existence
            let path1_target = Path::new(TEST_FOLDER).join("test_remove_no_target/target/test1.txt");
            let path2_target = Path::new(TEST_FOLDER).join("test_remove_no_target/target/test2.txt");

            assert!(!path1_target.exists());
            assert!(!path2_target.exists());
        })
    }

    #[test]
    #[serial]
    fn test_remove_empty() {
        run_test(||{
            // create target directory
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_remove_empty/target"), false).unwrap();

            // create test1.txt
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_remove_empty/target/test1.txt"), &"content1").unwrap();

            // create test2.txt
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_remove_empty/target/test2.txt"), &"content2").unwrap();

            // create file list
            let file_list = Vec::new();

            // run delete and expect ok
            let result = delete(file_list, &Path::new(TEST_FOLDER).join("test_remove_empty/target"));
            assert!(result.is_ok());

            // check target files' existence
            let path1_target = Path::new(TEST_FOLDER).join("test_remove_empty/target/test1.txt");
            let path2_target = Path::new(TEST_FOLDER).join("test_remove_empty/target/test2.txt");

            assert!(path1_target.exists());
            assert!(path2_target.exists());
        })
    }

    #[test]
    #[serial]
    fn test_remove_simple_file() {
        run_test(||{
            // create target directory
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_remove_simple_file/target"), false).unwrap();

            // create test1.txt
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_remove_simple_file/target/test1.txt"), &"content1").unwrap();

            // create test2.txt
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_remove_simple_file/target/test2.txt"), &"content2").unwrap();

            // create file list
            let mut file_list = Vec::new();
            file_list.push("test1.txt".to_string());

            // run delete and expect ok
            let result = delete(file_list, &Path::new(TEST_FOLDER).join("test_remove_simple_file/target"));
            assert!(result.is_ok());

            // check target files' existence
            let path1_target = Path::new(TEST_FOLDER).join("test_remove_simple_file/target/test1.txt");
            let path2_target = Path::new(TEST_FOLDER).join("test_remove_simple_file/target/test2.txt");

            assert!(!path1_target.exists());
            assert!(path2_target.exists());
        })
    }

    #[test]
    #[serial]
    fn test_remove_file_no_permission() {
        run_test(||{
            // create target directory
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_remove_file_no_permission/target"), false).unwrap();

            // create test1.txt
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_remove_file_no_permission/target/test1.txt"), &"content1").unwrap();

            // create test2.txt
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_remove_file_no_permission/target/test2.txt"), &"content2").unwrap();
            let mut perms = fs::metadata(&Path::new(TEST_FOLDER).join("test_remove_file_no_permission/target/test2.txt")).unwrap().permissions();
            perms.set_readonly(true);
            fs::set_permissions(&Path::new(TEST_FOLDER).join("test_remove_file_no_permission/target/test2.txt"), perms).unwrap();

            // create file list
            let mut file_list = Vec::new();
            file_list.push("test1.txt".to_string());
            file_list.push("test2.txt".to_string());

            // run delete and expect ok
            let result = delete(file_list, &Path::new(TEST_FOLDER).join("test_remove_file_no_permission/target"));
            assert!(result.is_err());

            // check target files' existence
            let path1_target = Path::new(TEST_FOLDER).join("test_remove_file_no_permission/target/test1.txt");
            let path2_target = Path::new(TEST_FOLDER).join("test_remove_file_no_permission/target/test2.txt");

            assert!(path1_target.exists());
            assert!(path2_target.exists());
        })
    }

    #[test]
    #[serial]
    fn test_remove_empty_dir() {
        run_test(||{
            // create target directory
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_remove_empty_dir/target"), false).unwrap();

            // create test1.txt
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_remove_empty_dir/target/test1.txt"), &"content1").unwrap();

            // create test2.txt
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_remove_empty_dir/target/test2.txt"), &"content2").unwrap();

            // create a folder test3
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_remove_empty_dir/target/test3"), false).unwrap();

            // create file list
            let mut file_list = Vec::new();
            file_list.push("test3".to_string());

            // run delete and expect ok
            let result = delete(file_list, &Path::new(TEST_FOLDER).join("test_remove_empty_dir/target"));
            assert!(result.is_ok());

            // check target files' existence
            let path1_target = Path::new(TEST_FOLDER).join("test_remove_empty_dir/target/test1.txt");
            let path2_target = Path::new(TEST_FOLDER).join("test_remove_empty_dir/target/test2.txt");
            let path3_target = Path::new(TEST_FOLDER).join("test_remove_empty_dir/target/test3");

            assert!(path1_target.exists());
            assert!(path2_target.exists());
            assert!(!path3_target.exists());
        })
    }

    #[test]
    #[serial]
    fn test_remove_non_empty_dir() {
        run_test(||{
            // create target directory
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_remove_non_empty_dir/target"), false).unwrap();

            // create test1.txt
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_remove_non_empty_dir/target/test1.txt"), &"content1").unwrap();

            // create test2.txt
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_remove_non_empty_dir/target/test2.txt"), &"content2").unwrap();

            // create a folder test3 and test3/test4.txt
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_remove_non_empty_dir/target/test3"), false).unwrap();
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_remove_non_empty_dir/target/test3/test4.txt"), &"content4").unwrap();

            // create file list
            let mut file_list = Vec::new();
            file_list.push("test3".to_string());

            // run delete and expect ok
            let result = delete(file_list, &Path::new(TEST_FOLDER).join("test_remove_non_empty_dir/target"));
            assert!(result.is_ok());

            // check target files' existence
            let path1_target = Path::new(TEST_FOLDER).join("test_remove_non_empty_dir/target/test1.txt");
            let path2_target = Path::new(TEST_FOLDER).join("test_remove_non_empty_dir/target/test2.txt");
            let path3_target = Path::new(TEST_FOLDER).join("test_remove_non_empty_dir/target/test3");
            let path4_target = Path::new(TEST_FOLDER).join("test_remove_non_empty_dir/target/test3/test4.txt");

            assert!(path1_target.exists());
            assert!(path2_target.exists());
            assert!(!path3_target.exists());
            assert!(!path4_target.exists());
        })
    }

    #[test]
    #[serial]
    fn test_remove_nested_file() {
        run_test(||{
            // create target directory
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_remove_nested_file/target"), false).unwrap();

            // create file
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_remove_nested_file/target/1/2/3/4/5"), false).unwrap();
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_remove_nested_file/target/1/2/3/4/5/test.txt"), &"5").unwrap();

            // create file list
            let mut file_list = Vec::new();
            file_list.push("1/2/3/4/5/test.txt".to_string());

            // run delete and expect ok
            let result = delete(file_list, &Path::new(TEST_FOLDER).join("test_remove_nested_file/target"));
            assert!(result.is_ok());

            // check target files' existence
            assert!(Path::new(TEST_FOLDER).join("test_remove_nested_file/target/1/2/3/4/5").exists());
            assert!(!Path::new(TEST_FOLDER).join("test_remove_nested_file/target/1/2/3/4/5/test.txt").exists());
        })
    }

    #[test]
    #[serial]
    fn test_remove_nested_dir() {
        run_test(||{
            // create target directory
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_remove_nested_dir/target"), false).unwrap();

            // create file
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_remove_nested_dir/target/1/2/3/4/5"), false).unwrap();
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_remove_nested_dir/target/1/2/3/4/5/test.txt"), &"5").unwrap();

            // create file list
            let mut file_list = Vec::new();
            file_list.push("1/2/3".to_string());

            // run delete and expect ok
            let result = delete(file_list, &Path::new(TEST_FOLDER).join("test_remove_nested_dir/target"));
            assert!(result.is_ok());

            // check target files' existence
            assert!(Path::new(TEST_FOLDER).join("test_remove_nested_dir/target/1/2").exists());
            assert!(!Path::new(TEST_FOLDER).join("test_remove_nested_dir/target/1/2/3").exists());
            assert!(!Path::new(TEST_FOLDER).join("test_remove_nested_dir/target/1/2/3/4/5/test.txt").exists());
        })
    }

    #[test]
    #[serial]
    fn test_remove_parent_before() {
        run_test(||{
            // create target directory
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_remove_parent_before/target"), false).unwrap();

            // create file
            fs_extra::dir::create_all(Path::new(TEST_FOLDER).join("test_remove_parent_before/target/1/2/3/4/5"), false).unwrap();
            fs_extra::file::write_all(&Path::new(TEST_FOLDER).join("test_remove_parent_before/target/1/2/3/4/5/test.txt"), &"5").unwrap();

            // create file list
            let mut file_list = Vec::new();
            file_list.push("1/2/3".to_string());
            file_list.push("1/2/3/4".to_string());
            file_list.push("1/2/3/4/5".to_string());
            file_list.push("1/2/3/4/5/test.txt".to_string());

            // run delete and expect ok
            let result = delete(file_list, &Path::new(TEST_FOLDER).join("test_remove_parent_before/target"));
            assert!(result.is_ok());

            // check target files' existence
            assert!(Path::new(TEST_FOLDER).join("test_remove_parent_before/target/1/2").exists());
            assert!(!Path::new(TEST_FOLDER).join("test_remove_parent_before/target/1/2/3").exists());
            assert!(!Path::new(TEST_FOLDER).join("test_remove_parent_before/target/1/2/3/4/5/test.txt").exists());
        })
    }
}