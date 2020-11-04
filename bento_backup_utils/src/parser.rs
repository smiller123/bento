use std::error::Error;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io;
use std::fs;

#[derive(Debug)]
enum Event {
    Open { pid: u64, flags: u64, inode: u64 },
    Close { pid: u64, inode: u64 },
    Create { pid: u64, path: String, mode: u64, flags: u64, inode: u64, parent: u64 },
    Mkdir { pid: u64, path: String, mode: u64, inode: u64, parent: u64 },
    SymLink { pid: u64, path_1: String, path_2: String },
    Unlink { r#type: String, pid: u64, path: String, inode: u64, parent: u64},
    UnlinkDeleted { r#type: String, pid: u64, path: String, inode: u64, parent: u64},
    Rename { parent_inode: u64, old_name: String, newparent_inode: u64, new_name: String, moved_inode: u64, swapped_inode: u64, overwritten_inode: u64},
}

pub fn halves_if_even(i: i32) -> Result<i32, Box<dyn Error>> {
    if i % 2 == 0 {
        Ok(i / 2)
    } else {
        Err(From::from("44"))
    }
}



// Parse token in the format of key:value
fn parse_key_value(token: &str) -> Result<(&str, &str), Box<dyn Error>>{
    let vec: Vec<&str> = token.split(':').collect();

    if vec.len() == 2 {
        let pair = (vec[0].trim(), vec[1].trim());
        Ok(pair)
    } else {
        Err(From::from("ParseError"))
    }
}


fn parse_open(kv_maps: HashMap<&str, &str>) -> Result<Event, Box<dyn Error>> {
    let pid: u64;
    let flags: u64;
    let inode: u64;

    match kv_maps.get(&"pid"){
        Some(v) => pid = v.parse::<u64>().unwrap(),
        _ => return Err(From::from("ParseError"))
    }

    match kv_maps.get(&"flags"){
        Some(v) => flags = v.parse::<u64>().unwrap(),
        _ => return Err(From::from("ParseError"))
    }

    match kv_maps.get(&"inode"){
        Some(v) => inode = v.parse::<u64>().unwrap(),
        _ => return Err(From::from("ParseError"))
    }

    Ok(Event::Open {
        pid: pid,
        flags: flags,
        inode: inode,
    })
}

fn parse_mkdir(kv_maps: HashMap<&str, &str>) -> Result<Event, Box<dyn Error>> {
    let pid: u64;
    let path: String;
    let mode: u64;
    let inode: u64;
    let parent: u64;

    match kv_maps.get(&"pid"){
        Some(v) => pid = v.parse::<u64>().unwrap(),
        _ => return Err(From::from("ParseError"))
    }

    match kv_maps.get(&"path"){
        Some(v) => path = v.to_string(),
        _ => return Err(From::from("ParseError"))
    }

    match kv_maps.get(&"mode"){
        Some(v) => mode = v.parse::<u64>().unwrap(),
        _ => return Err(From::from("ParseError"))
    }

    match kv_maps.get(&"inode"){
        Some(v) => inode = v.parse::<u64>().unwrap(),
        _ => return Err(From::from("ParseError"))
    }

    match kv_maps.get(&"parent"){
        Some(v) => parent = v.parse::<u64>().unwrap(),
        _ => return Err(From::from("ParseError"))
    }

    Ok(Event::Mkdir {
        pid: pid,
        path: path,
        mode: mode,
        inode: inode,
        parent: parent,
    })
}

fn parse_close(kv_maps: HashMap<&str, &str>) -> Result<Event, Box<dyn Error>> {
    let pid: u64;
    let inode: u64;

    match kv_maps.get(&"pid"){
        Some(v) => pid = v.parse::<u64>().unwrap(),
        _ => return Err(From::from("ParseError"))
    }

    match kv_maps.get(&"inode"){
        Some(v) => inode = v.parse::<u64>().unwrap(),
        _ => return Err(From::from("ParseError"))
    }

    Ok(Event::Close {
        pid: pid,
        inode: inode,
    })
}

fn parse_create(kv_maps: HashMap<&str, &str>) -> Result<Event, Box<dyn Error>> {
    let pid: u64;
    let path: String;
    let mode: u64;
    let flags: u64;
    let inode: u64;
    let parent: u64;

    match kv_maps.get(&"pid"){
        Some(v) => pid = v.parse::<u64>().unwrap(),
        _ => return Err(From::from("ParseError"))
    }

    match kv_maps.get(&"path"){
        Some(v) => path = v.to_string(),
        _ => return Err(From::from("ParseError"))
    }

    match kv_maps.get(&"mode"){
        Some(v) => mode = v.parse::<u64>().unwrap(),
        _ => return Err(From::from("ParseError"))
    }

    match kv_maps.get(&"flags"){
        Some(v) => flags = v.parse::<u64>().unwrap(),
        _ => return Err(From::from("ParseError"))
    }

    match kv_maps.get(&"inode"){
        Some(v) => inode = v.parse::<u64>().unwrap(),
        _ => return Err(From::from("ParseError"))
    }

    match kv_maps.get(&"parent"){
        Some(v) => parent = v.parse::<u64>().unwrap(),
        _ => return Err(From::from("ParseError"))
    }

    Ok(Event::Create {
        pid: pid,
        path: path,
        mode: mode,
        flags: flags,
        inode: inode,
        parent: parent
    })
}

fn parse_symlink(kv_maps: HashMap<&str, &str>) -> Result<Event, Box<dyn Error>> {
    let pid: u64;
    let path_1: String;
    let path_2: String;

    match kv_maps.get(&"pid"){
        Some(v) => pid = v.parse::<u64>().unwrap(),
        _ => return Err(From::from("ParseError"))
    }

    match kv_maps.get(&"path_1"){
        Some(v) => path_1 = v.to_string(),
        _ => return Err(From::from("ParseError"))
    }

    match kv_maps.get(&"path_2"){
        Some(v) => path_2 = v.to_string(),
        _ => return Err(From::from("ParseError"))
    }

    Ok(Event::SymLink {
        pid: pid,
        path_1: path_1,
        path_2: path_2,
    })
}

fn parse_rename(line: String) -> Result<Event, Box<dyn Error>> {
    let line = &line[7..]; // remove prefix "rename:"
    println!("debug {:?}", line);
    let vec: Vec<&str> = line.split(',').collect();
    let parent_inode: u64 = vec[0].trim().parse::<u64>().unwrap();
    let old_name: String = vec[1].trim().to_string();
    let newparent_inode: u64 = vec[2].trim().parse::<u64>().unwrap();
    let new_name: String = vec[3].trim().to_string();
    let moved_inode: u64 = vec[4].trim().parse::<u64>().unwrap();
    // TODO: the values are optional
    let swapped_inode: u64 = vec[4].trim().parse::<u64>().unwrap();
    let overwritten_inode: u64 = vec[4].trim().parse::<u64>().unwrap();

    Ok(Event::Rename {
        parent_inode: parent_inode,
        old_name: old_name,
        newparent_inode: newparent_inode,
        new_name: new_name,
        moved_inode: moved_inode,
        swapped_inode: swapped_inode,
        overwritten_inode: overwritten_inode,
    })
}

fn parse_unlink(kv_maps: HashMap<&str, &str>) -> Result<Event, Box<dyn Error>> {
    let r#type: String;
    let pid: u64;
    let path: String;
    let inode: u64;
    let parent: u64;

    match kv_maps.get(&"type"){
        Some(v) => r#type = v.to_string(),
        _ => return Err(From::from("ParseError"))
    }
    match kv_maps.get(&"pid"){
        Some(v) => pid = v.parse::<u64>().unwrap(),
        _ => return Err(From::from("ParseError"))
    }

    match kv_maps.get(&"path"){
        Some(v) => path = v.to_string(),
        _ => return Err(From::from("ParseError"))
    }

    match kv_maps.get(&"inode"){
        Some(v) => inode = v.parse::<u64>().unwrap(),
        _ => return Err(From::from("ParseError"))
    }

    match kv_maps.get(&"parent"){
        Some(v) => parent = v.parse::<u64>().unwrap(),
        _ => return Err(From::from("ParseError"))
    }

    Ok(Event::Unlink {
        r#type: r#type,
        pid: pid,
        path: path,
        inode: inode,
        parent: parent
    })
}

fn parse_unlink_deleted(kv_maps: HashMap<&str, &str>) -> Result<Event, Box<dyn Error>> {
    let r#type: String;
    let pid: u64;
    let path: String;
    let inode: u64;
    let parent: u64;

    match kv_maps.get(&"type"){
        Some(v) => r#type = v.to_string(),
        _ => return Err(From::from("ParseError"))
    }
    match kv_maps.get(&"pid"){
        Some(v) => pid = v.parse::<u64>().unwrap(),
        _ => return Err(From::from("ParseError"))
    }

    match kv_maps.get(&"path"){
        Some(v) => path = v.to_string(),
        _ => return Err(From::from("ParseError"))
    }

    match kv_maps.get(&"inode"){
        Some(v) => inode = v.parse::<u64>().unwrap(),
        _ => return Err(From::from("ParseError"))
    }

    match kv_maps.get(&"parent"){
        Some(v) => parent = v.parse::<u64>().unwrap(),
        _ => return Err(From::from("ParseError"))
    }

    Ok(Event::UnlinkDeleted {
        r#type: r#type,
        pid: pid,
        path: path,
        inode: inode,
        parent: parent
    })
}

fn parse_event(line: &str) -> Result<Event, Box<dyn Error>> {
    let mut kv_maps = HashMap::new();

    let vec: Vec<&str> = line.split(',').collect();
    vec.iter()
        .for_each(|p| {
            let parsed_token = parse_key_value(p);
            match parsed_token {
                Ok(v) => kv_maps.insert(v.0, v.1),
                _ => None,
            };
        });


    println!("{:?}", kv_maps);
    // parse open/close/create/symlink
    match kv_maps.get(&"op") {
        Some(&"open") => return parse_open(kv_maps),
        Some(&"close") => return parse_close(kv_maps),
        Some(&"create") => return parse_create(kv_maps),
        Some(&"symlink") => return parse_symlink(kv_maps),
        Some(&"mkdir") => return parse_mkdir(kv_maps),
        Some(&"unlink") => return parse_unlink(kv_maps),
        Some(&"unlink_deleted") => return parse_unlink_deleted(kv_maps),
        _ => (),
    };

    // parse rename
    println!("debug {:?}", kv_maps);
    match kv_maps.get(&"rename") {
        Some(_) => return parse_rename(line.to_string()),
        None => (),
    };

    Err(From::from("error"))
}

fn update_inode_map(inode_map: &mut HashMap<u64, String>, events: &Vec<Event>) {
    for event in events {
        match event {
            Event::Create { pid: _, path, mode: _, flags: _, inode, parent } => {
                match inode_map.get(&parent) {
                    Some(parent_path) => {
                        let full_path = format!("{}/{}", parent_path, path).to_string();
                        println!("inserted {} {}", inode, full_path);
                        inode_map.insert(*inode, full_path);
                    },
                    _ => println!("inode key {} is not found", *parent)
                }
            },
            Event::Rename {parent_inode, old_name: _, newparent_inode, new_name, moved_inode, swapped_inode, overwritten_inode} => {
                match inode_map.get(&newparent_inode) {
                    Some(parent_path) => {
                        let full_path = format!("{}/{}", parent_path, new_name).to_string();
                        println!("inserted {} {}", moved_inode, full_path);
                        inode_map.insert(*moved_inode, full_path);
                    },
                    _ => println!("inode key {} is not found", *newparent_inode)
                }
            },
            Event::Mkdir { pid: _, path, mode, inode, parent } => {
                inode_map.insert(*inode, path.to_string());
            },
            _ => (),
        }
    }
}

fn files_to_update(inode_map: &HashMap<u64, String>, events: &Vec<Event>) -> HashSet<String> {
    let mut files = HashSet::<String>::new();
    for event in events {
        match event {
            Event::Close { inode, ..} => {
                match inode_map.get(inode) {
                    Some(v) => { files.insert(v.to_string()); },
                    None => { println!("inode num {} not found in map", inode); }
                }
            },
            Event::Create { inode, ..} => {
                match inode_map.get(inode) {
                    Some(v) => { files.insert(v.to_string()); },
                    None => { println!("inode num {} not found in map", inode); }
                }
            },
            // TOOD(nmonsees): this fn should return a map of filename to an action,
            // to specify whether the file should be deleted, copied over, etc.
            Event::UnlinkDeleted { inode, ..} => {
                match inode_map.get(inode) {
                    Some(v) => { files.insert(v.to_string()); },
                    None => { println!("inode num {} not found in map", inode); }
                }
            },
            // TODO(nmonsees): this will need to handle cases where a rename overwrites vs. swaps,
            // which I think can be handled just by whether a swapped inode exists or not?
            Event::Rename { old_name, new_name, ..} => {
                files.insert(old_name.to_string());
                files.insert(new_name.to_string());
            },
            _ => (),
        }
    }
    files
}

fn read_lin_file(file_name: &str) -> Result<String, io::Error> {
    fs::read_to_string(file_name)
}

fn main(){
    let mut events = Vec::<Event>::new();
    let mut inode_map = HashMap::new();

    let lin = read_lin_file(".lin");
    let lin = match lin {
        Ok(file) => file,
        Err(error) => panic!("Problem opening the file: {:?}", error),
    };

    let vec: Vec<&str> = lin.split('\n').collect();
    vec.iter()
        .for_each(|p| {
            if !p.is_empty() {
                let result = parse_event(p);
                match result {
                    Ok(event) => {println!("ok {:?}", event); events.push(event); },
                    Err(_e) => {println!("error {:?}", p);},
                }
            }
        });

    inode_map.insert(1, "".to_string());
    update_inode_map(&mut inode_map, &events);

    for (key, value) in &inode_map {
        println!("{}: {}", key, value);
    }
    let files = files_to_update(&inode_map, &events);
    files.iter().for_each(|f| { println!("file to update {:?}", f); });
    

    // println!("{:?}", parse_key_value("op: open"));
    // println!("{:?}", parse_key_value("op: open: ss"));
    // println!("{:?}", parse_key_value("op:"));
    // println!("{:?}", parse_key_value("op"));

    // println!("{:?}", parse_event("op: open, pid: 0, flags: 0, inode: 0"));
    // println!("{:?}", parse_event("op: open, pid: 0, flags: 0, inoded: 0"));
    // println!("{:?}", parse_event("op: close, pid: 0, inode: 0"));
    // println!("{:?}", parse_event("op: close, pid: 0, inode: 0, random: 6666"));
    // println!("{:?}", parse_event("op: create, pid: 0, path: hello.txt, mode: 33152, flags: 164034, inode: 0"));
    // println!("{:?}", parse_event("op: symlink, pid: 0, path_1: hello.txt, path_2: test.txt"));
    // println!("{:?}", parse_event("rename: 1, hello.txt, 1, hello.txt~"));

    // Ok(("op", "open"))
    // Err("ParseError")
    // Ok(("op", ""))
    // Err("ParseError")
    // {"pid": "0", "op": "open", "flags": "0", "inode": "0"}
    // Ok(Open { pid: 0, flags: 0, inode: 0 })
    // {"flags": "0", "inoded": "0", "pid": "0", "op": "open"}
    // Err("ParseError")
    // {"op": "close", "pid": "0", "inode": "0"}
    // Ok(Close { pid: 0, inode: 0 })
    // {"pid": "0", "op": "close", "random": "6666", "inode": "0"}
    // Ok(Close { pid: 0, inode: 0 })
    // {"pid": "0", "path": "hello.txt", "flags": "164034", "inode": "0", "op": "create", "mode": "33152"}
    // Ok(Create { pid: 0, path: "hello.txt", mode: 33152, flags: 164034, inode: 0 })
    // {"op": "symlink", "path_1": "hello.txt", "path_2": "test.txt", "pid": "0"}
    // Ok(SymLink { pid: 0, path_1: "hello.txt", path_2: "test.txt" })
    // {"rename": "1"}
    // debug {"rename": "1"}
    // debug " 1, hello.txt, 1, hello.txt~"
    // Ok(Rename { parent_inode: 1, old_name: "hello.txt", newparent_inode: 1, new_name: "hello.txt~" })
}
