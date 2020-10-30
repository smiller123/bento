use std::error::Error;
use std::collections::HashMap;

#[derive(Debug)]
enum Event {
    Open { pid: u64, flags: u64, inode: u64 },
    Close { pid: u64, inode: u64 },
    Create { pid: u64, path: String, mode: u64, flags: u64, inode: u64 },
    SymLink { pid: u64, path_1: String, path_2: String },
    Rename { parent_inode: u64, old_name: String, newparent_inode: u64, new_name: String},
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

    Ok(Event::Create {
        pid: pid,
        path: path,
        mode: mode,
        flags: flags,
        inode: inode,
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

    Ok(Event::Rename {
        parent_inode: parent_inode,
        old_name: old_name,
        newparent_inode: newparent_inode,
        new_name: new_name,
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

fn update_inode_map(inode_map: &mut HashMap<u64, String>, events: &Vec<&Event>) {
    for event in events {
        match event {
            Event::Create { pid: _, path, mode: _, flags: _, inode } => {
                inode_map.insert(*inode, path.to_string());
            },
            Event::Rename {parent_inode, old_name: _, newparent_inode, new_name } => {
                // does old name need to be removed?
                inode_map.remove(&parent_inode);
                inode_map.insert(*newparent_inode, new_name.to_string());
            }
            _ => (),
        }
    }
}

fn main(){
    println!("{:?}", parse_key_value("op: open"));
    println!("{:?}", parse_key_value("op: open: ss"));
    println!("{:?}", parse_key_value("op:"));
    println!("{:?}", parse_key_value("op"));

    println!("{:?}", parse_event("op: open, pid: 0, flags: 0, inode: 0"));
    println!("{:?}", parse_event("op: open, pid: 0, flags: 0, inoded: 0"));
    println!("{:?}", parse_event("op: close, pid: 0, inode: 0"));
    println!("{:?}", parse_event("op: close, pid: 0, inode: 0, random: 6666"));
    println!("{:?}", parse_event("op: create, pid: 0, path: hello.txt, mode: 33152, flags: 164034, inode: 0"));
    println!("{:?}", parse_event("op: symlink, pid: 0, path_1: hello.txt, path_2: test.txt"));
    println!("{:?}", parse_event("rename: 1, hello.txt, 1, hello.txt~"));

    let mut events = Vec::<&Event>::new();
    let mut inode_map = HashMap::new();
    let event_list = [
        parse_event("op: create, pid: 0, path: hello.txt, mode: 33152, flags: 164034, inode: 0"),
        parse_event("op: open, pid: 0, flags: 0, inode: 0"),
        parse_event("op: close, pid: 0, inode: 0"),
        parse_event("rename: 1, hello.txt, 1, hello.txt~"),
        parse_event("op: symlink, pid: 0, path_1: hello.txt, path_2: test.txt"),
    ];

    for event in event_list.iter() {
        match event {
            Ok(e) => events.push(e),
            Err(e) => panic!("Parser error: {:?}", e)
        }
    }

    update_inode_map(&mut inode_map, &events);

    for (key, value) in inode_map {
        println!("{}: {}", key, value);
    }

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