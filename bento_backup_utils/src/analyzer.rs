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
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::parser as parser;

#[derive(Eq, PartialEq, Debug)]
pub enum Action {
    Update,
    Delete,
}

pub fn files_to_update(inode_map: &mut HashMap<u64, PathBuf>, events: &[parser::Event]) -> HashMap<PathBuf, Action> {
    let mut files = HashMap::<PathBuf, Action>::new();
    for event in events {
        match event {
            parser::Event::Close { inode, ..} => {
                if let Some(v) = inode_map.get(inode) {
                    files.insert(v.clone(), Action::Update);
                }
            },
            parser::Event::Create { pid: _, path, mode: _, flags: _, inode, parent } => {
                // update inode map
                match inode_map.get(&parent) {
                    Some(parent_path) => {
                        let full_path = Path::new(parent_path).join(path);
                        inode_map.insert(*inode, full_path);
                    },
                    _ => ()
                }
                // add update action
                if let Some(v) = inode_map.get(inode) {
                    files.insert(v.clone(), Action::Update);
                }
            },
            parser::Event::UnlinkDeleted { inode, ..} => {
                if let Some(v) = inode_map.get(inode) {
                    files.insert(v.clone(), Action::Delete);
                }
            },
            parser::Event::Mkdir { pid: _, path, mode: _, inode, parent } => {
                // update inode map
                match inode_map.get(&parent) {
                    Some(parent_path) => {
                        let full_path = Path::new(parent_path).join(path);
                        inode_map.insert(*inode, full_path);
                    },
                    _ => ()
                }

                // add update action
                if let Some(v) = inode_map.get(inode) {
                    files.insert(v.clone(), Action::Update);
                }
            }

            parser::Event::Rename {parent_inode, old_name, newparent_inode, new_name, moved_inode, swapped_inode, overwritten_inode} => {
                // update inode map
                match inode_map.get(&newparent_inode) {
                    Some(parent_path) => {
                        let full_path = Path::new(parent_path).join(new_name);
                        inode_map.insert(moved_inode.unwrap(), full_path);
                    },
                    _ => ()
                }

                if overwritten_inode.is_some() {
                    let inode = overwritten_inode.unwrap();
                    inode_map.remove(&inode);
                }

                if swapped_inode.is_some() {
                    let swapped_inode = swapped_inode.unwrap();
                    match inode_map.get(&parent_inode) {
                        Some(dest_parent_path) => {
                            match inode_map.get(&swapped_inode) {
                                Some(src_path) => {
                                    let file_name = Path::new(src_path).file_name().unwrap();
                                    let full_path = Path::new(dest_parent_path).join(file_name);
                                    inode_map.insert(swapped_inode, full_path);
                                },
                                _ => ()
                            }
                        },
                        _ => ()
                    }
                }

                // add update/delete action
                // otherwise check swapped inode?
                if let Some(v) = inode_map.get(&parent_inode) {
                    files.insert(v.join(old_name), Action::Delete);
                }

                if moved_inode.is_some() {
                    if let Some(v) = inode_map.get(&moved_inode.unwrap()) {
                        files.insert(v.clone(), Action::Update);
                    }
                } else {
                    panic!("moved_inode not in rename event");
                }
            },
            _ => (),
        }
    }
    files
}

pub fn populate_events(events: &mut Vec::<parser::Event>, lin: String) {
    let vec: Vec<&str> = lin.split('\n').collect();
    vec.iter()
        .for_each(|p| {
            if !p.is_empty() {
                let result = parser::parse_event(p);
                match result {
                    Ok(event) => { events.push(event); },
                    Err(_e) => (),
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Event;

    #[test]
    fn test_create() {
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

        let files_to_update_map = files_to_update(&mut inode_map, &events);

        let path: PathBuf = [parent_dir, file_name].iter().collect();
        assert_eq!(inode_map[&inode], path);

        assert_eq!(files_to_update_map.len(), 1);
        assert!(files_to_update_map.contains_key(&path), true);
        assert!(matches!(files_to_update_map.get(&path).unwrap(), Action::Update));
    }

    #[test]
    fn test_rename() {
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

        let files_to_update_map = files_to_update(&mut inode_map, &events);

        // check inode map
        let new_path: PathBuf = [new_parent_dir, new_file_name].iter().collect();
        assert_eq!(inode_map[&inode], new_path);

        // check actions
        assert_eq!(files_to_update_map.len(), 2);
        assert!(files_to_update_map.contains_key(&PathBuf::from("/test_dir/file_name")), true);
        assert!(matches!(files_to_update_map.get(&PathBuf::from("/test_dir/file_name")).unwrap(), Action::Delete));
        assert!(files_to_update_map.contains_key(&PathBuf::from("/new_dir/new_name")), true);
        assert!(matches!(files_to_update_map.get(&PathBuf::from("/new_dir/new_name")).unwrap(), Action::Update));
    }

    #[test]
    fn test_rename_swapped() {
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

        let files_to_update_map = files_to_update(&mut inode_map, &events);

        let new_path: PathBuf = [new_parent_dir, new_file_name].iter().collect();
        assert_eq!(inode_map[&inode], new_path);
        let swapped_new_path: PathBuf = [parent_dir, swapped_file_name].iter().collect();
        assert_eq!(inode_map[&swapped_inode], swapped_new_path);

        assert_eq!(files_to_update_map.len(), 2);
        assert!(files_to_update_map.contains_key(&PathBuf::from("/test_dir/file_name")), true);
        assert!(matches!(files_to_update_map.get(&PathBuf::from("/test_dir/file_name")).unwrap(), Action::Delete));
        assert!(files_to_update_map.contains_key(&PathBuf::from("/new_dir/new_name")), true);
        assert!(matches!(files_to_update_map.get(&PathBuf::from("/new_dir/new_name")).unwrap(), Action::Update));
    }

    #[test]
    fn test_update_delete_file() {
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

        let files_to_update_map = files_to_update(&mut inode_map, &events);
        assert_eq!(files_to_update_map.len(), 1);
        assert!(files_to_update_map.contains_key(&PathBuf::from("/test_dir/file_name")), true);
        assert!(matches!(files_to_update_map.get(&PathBuf::from("/test_dir/file_name")).unwrap(), Action::Delete));
    }

    #[test]
    fn test_create_update_file() {
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

        let files_to_update_map = files_to_update(&mut inode_map, &events);
        assert_eq!(files_to_update_map.len(), 1);
        assert!(files_to_update_map.contains_key(&PathBuf::from("/test_dir/file_name")), true);
        assert!(matches!(files_to_update_map.get(&PathBuf::from("/test_dir/file_name")).unwrap(), Action::Update));
    }

    #[test]
    fn test_mkdir() {
        let mut inode_map = HashMap::<u64, PathBuf>::new();
        let mut events = Vec::<Event>::new();

        let parent_inode = 2;
        let parent_dir = "/test_dir";
        let parent_path = PathBuf::from(parent_dir);
        inode_map.insert(parent_inode, parent_path);

        let inode = 3;
        let dir = "new_dir";
        events.push(Event::Mkdir{
            pid: 0,
            path: dir.to_string(),
            mode: 0,
            inode: inode,
            parent: parent_inode,
        });

        let files_to_update_map = files_to_update(&mut inode_map, &events);

        // check inode map
        let path: PathBuf = [parent_dir, dir].iter().collect();
        assert_eq!(inode_map[&inode], path);

        // check action
        assert_eq!(files_to_update_map.len(), 1);
        assert!(files_to_update_map.contains_key(&PathBuf::from("/test_dir/new_dir")), true);
        assert!(matches!(files_to_update_map.get(&PathBuf::from("/test_dir/new_dir")).unwrap(), Action::Update));
    }

    #[test]
    fn test_populate_events() {
        let mut events = Vec::<Event>::new();
        assert_eq!(events.len(), 0);

        // test populate empty string
        populate_events(&mut events, "".to_string());
        assert_eq!(events.len(), 0);

        // test populate with single-line event
        let single_line = "op: close, pid: 3052306432, inode: 3".to_string();
        populate_events(&mut events, single_line);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], Event::Close{
            pid: 3052306432,
            inode: 3,
        });

        let two_lines = "op: create, pid: 0, path: file_for_edit, \
                            mode: 33188, flags: 34881, inode: 4, parent: 1\n
                            op: close, pid: 3052306432, inode: 4".to_string();
        populate_events(&mut events, two_lines);
        assert_eq!(events.len(), 3);
        assert_eq!(events[1], Event::Create{
            pid: 0,
            path: "file_for_edit".to_string(),
            mode: 33188,
            flags: 34881,
            inode: 4,
            parent: 1,
        });
        assert_eq!(events[2], Event::Close{
            pid: 3052306432,
            inode: 4,
        });

        let broken_lines = "op: close, pid: 3052306432, inode: 3\n
                            dgdfgldfgpddgkfdfgmdkfgf\n
                            op: close, pid: 3052306432, inode: 3".to_string();
        populate_events(&mut events, broken_lines);
        assert_eq!(events.len(), 5);
        assert_eq!(events[3], Event::Close{
            pid: 3052306432,
            inode: 3,
        });
        assert_eq!(events[4], Event::Close{
            pid: 3052306432,
            inode: 3,
        });
    }
}