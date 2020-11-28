use serial_test::serial;
use std::panic;
use std::path::PathBuf;
use std::collections::HashMap;
use crate::parser::Event;
use crate::parser::Action;

#[path = "../src/parser.rs"] mod parser;

#[allow(dead_code)]
fn setup() -> () {
}

#[allow(dead_code)]
fn teardown() {
}

fn run_test<T>(test: T) -> ()
where
    T: FnOnce() -> () + panic::UnwindSafe
{
    setup();
    let result = panic::catch_unwind(|| {
        test();
    });
    teardown();
    assert!(result.is_ok())
}

#[test]
#[serial]
fn test_create() {
    run_test(||{
        let mut inode_map = HashMap::<u64, PathBuf>::new();
        let mut events = Vec::<Event>::new();

        let parent_inode = 2;
        let parent_dir = "\\test_dir";
        let parent_path = PathBuf::from(parent_dir);
        inode_map.insert(parent_inode, parent_path);

        let inode = 3;
        let file_name = "file_name";
        events.push(Event::Create {
            pid: 0,
            path: file_name.to_string(),
            mode: 0,
            flags: 0,
            inode: inode,
            parent: parent_inode
        });

        parser::update_inode_map(&mut inode_map, &events);

        let path: PathBuf = [parent_dir, file_name].iter().collect();
        assert_eq!(inode_map[&inode], path);
    })
}

#[test]
#[serial]
fn test_rename() {
    run_test(||{
        let mut inode_map = HashMap::<u64, PathBuf>::new();
        let mut events = Vec::<Event>::new();

        let parent_inode = 2;
        let parent_dir = "/test_dir";
        let parent_path = PathBuf::from(parent_dir);
        inode_map.insert(parent_inode, parent_path);

        let inode = 3;
        let file_name = "file_name";
        let path: PathBuf = [parent_dir, file_name].iter().collect();
        inode_map.insert(inode, path);

        let new_parent_inode = 4;
        let new_parent_dir = "/new_dir";
        let new_parent_path = PathBuf::from(new_parent_dir);
        inode_map.insert(new_parent_inode, new_parent_path);

        let new_file_name = "new_name";
        events.push(Event::Rename{
            parent_inode,
            old_name: file_name.to_string(),
            newparent_inode: new_parent_inode,
            new_name: new_file_name.to_string(),
            moved_inode: Some(inode),
            swapped_inode: None,
            overwritten_inode: None,
        });

        parser::update_inode_map(&mut inode_map, &events);

        let new_path: PathBuf = [new_parent_dir, new_file_name].iter().collect();
        assert_eq!(inode_map[&inode], new_path);


        let updates = parser::files_to_update(&inode_map, &events);
        if updates.len() != 2 {
            println!("updates.len() == {}", updates.len());
            for (file, action) in &updates {
                println!("{}: {:?}", file.to_str().unwrap(), action);
            }
        }
        assert!(updates.len() == 2);
    })
}

#[test]
#[serial]
fn test_rename_swapped() {
    run_test(||{
        let mut inode_map = HashMap::<u64, PathBuf>::new();
        let mut events = Vec::<Event>::new();

        let parent_inode = 2;
        let parent_dir = "/test_dir";
        let parent_path = PathBuf::from(parent_dir);
        inode_map.insert(parent_inode, parent_path);

        let inode = 3;
        let file_name = "file_name";
        let path: PathBuf = [parent_dir, file_name].iter().collect();
        inode_map.insert(inode, path);

        let new_parent_inode = 4;
        let new_parent_dir = "/new_dir";
        let new_parent_path = PathBuf::from(new_parent_dir);
        inode_map.insert(new_parent_inode, new_parent_path);

        let swapped_inode = 5;
        let swapped_file_name = "file_name_2";
        let swapped_path: PathBuf = [new_parent_dir, swapped_file_name].iter().collect();
        inode_map.insert(swapped_inode, swapped_path);

        let new_file_name = "new_name";
        events.push(Event::Rename{
            parent_inode,
            old_name: file_name.to_string(),
            newparent_inode: new_parent_inode,
            new_name: new_file_name.to_string(),
            moved_inode: Some(inode),
            swapped_inode: Some(swapped_inode),
            overwritten_inode: None,
        });

        parser::update_inode_map(&mut inode_map, &events);

        let new_path: PathBuf = [new_parent_dir, new_file_name].iter().collect();
        assert_eq!(inode_map[&inode], new_path);
        let swapped_new_path: PathBuf = [parent_dir, swapped_file_name].iter().collect();
        assert_eq!(inode_map[&swapped_inode], swapped_new_path);

        let updates = parser::files_to_update(&inode_map, &events);
        assert!(updates.len() == 2);
    })
}

#[allow(unused_variables,unused_mut)] // TODO: Remove this
#[test]
#[serial]
fn test_update_delete_file() {
    run_test(||{
        let mut inode_map = HashMap::<u64, PathBuf>::new();
        let mut events = Vec::<Event>::new();

        let parent_inode = 2;
        let parent_dir = "/test_dir";
        let parent_path = PathBuf::from(parent_dir);
        inode_map.insert(parent_inode, parent_path);

        let inode = 3;
        let file_name = "file_name";
        let path: PathBuf = [parent_dir, file_name].iter().collect();
        let path_copy = path.clone();
        inode_map.insert(inode, path);

        // create file, push update, close
        events.push(Event::Create{
            pid: 0,
            path: file_name.to_string(),
            mode: 0,
            flags: 0,
            inode: inode,
            parent: parent_inode
        });

        events.push(Event::Close{
            pid: 0,
            inode: inode
        });

        events.push(Event::UnlinkDeleted {
            r#type: file_name.to_string(),
            pid: 0,
            path: file_name.to_string(),
            inode: inode,
            parent: parent_inode,
        });


        let mut updates = parser::files_to_update(&inode_map, &events);
        assert!(updates.len() == 1);

        // match_action(updates, &path_copy.as_path(), Action::Delete);

        match updates.get(&path_copy) {
            Some(Action::Delete) => (),
            Some(Action::Update) => panic!("expected Delete, found update instead"),
            None => panic!("path not found in updates"),
        }
    })
}

// fn match_action(files_map: &HashMap::<&Path, Action>, file: &Path, action: Action) {

// }

#[allow(unused_variables,unused_mut)]
#[test]
#[serial]
fn test_create_update_file() {
    run_test(||{
        let mut inode_map = HashMap::<u64, PathBuf>::new();
        let mut events = Vec::<Event>::new();

        let parent_inode = 2;
        let parent_dir = "/test_dir";
        let parent_path = PathBuf::from(parent_dir);
        inode_map.insert(parent_inode, parent_path);

        let inode = 3;
        let file_name = "file_name";
        let path: PathBuf = [parent_dir, file_name].iter().collect();
        let path_copy = path.clone();
        inode_map.insert(inode, path);

        // create file, push update, close
        events.push(Event::Create{
            pid: 0,
            path: file_name.to_string(),
            mode: 0,
            flags: 0,
            inode: inode,
            parent: parent_inode
        });

        events.push(Event::Close{
            pid: 0,
            inode: inode
        });

        let updates = parser::files_to_update(&inode_map, &events);
        assert!(updates.len() == 1);
        match updates.get(&path_copy) {
            Some(Action::Update) => (),
            Some(action) => panic!("expected Action::Update, found another action instead"),
            _ => panic!("path not found in updates"),
        }
    })
}

#[test]
#[serial]
fn test_mkdir() {
    run_test(||{
        let mut inode_map = HashMap::<u64, PathBuf>::new();
        let mut events = Vec::<Event>::new();

        let parent_inode = 2;
        let parent_dir = "/test_dir";
        let parent_path = PathBuf::from(parent_dir); inode_map.insert(parent_inode, parent_path);

        let inode = 3;
        let dir = "new_dir";
        events.push(Event::Mkdir{
            pid: 0,
            path: dir.to_string(),
            mode: 0,
            inode: inode,
            parent: parent_inode,
        });

        parser::update_inode_map(&mut inode_map, &events);

        let path: PathBuf = [parent_dir, dir].iter().collect();
        assert_eq!(inode_map[&inode], path);
    })
}
