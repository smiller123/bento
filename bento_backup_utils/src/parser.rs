use std::error::Error;
use std::collections::HashMap;

#[derive(Eq, PartialEq, Debug)]
pub enum Event {
    Open {
        pid: u64,
        flags: u64,
        inode: u64,
    },
    Close {
        pid: u64,
        inode: u64,
    },
    Mkdir {
        pid: u64,
        path: String,
        mode: u64,
        inode: u64,
        parent: u64,
    },
    Create {
        pid: u64,
        path: String,
        mode: u64,
        flags: u64,
        inode: u64,
        parent: u64,
    },
    SymLink {
        pid: u64,
        path_1: String,
        path_2: String,
    },
    Rename {
        parent_inode: u64,
        old_name: String,
        newparent_inode: u64,
        new_name: String,
        moved_inode: Option<u64>,
        swapped_inode: Option<u64>,
        overwritten_inode: Option<u64>,
    },
    Unlink {
        r#type: String,
        pid: u64,
        path: String,
        inode: u64,
        parent: u64,
    },
    UnlinkDeleted {
        r#type: String,
        pid: u64,
        path: String,
        inode: u64,
        parent: u64,
    },
}


// Parse token in the format of key:value
pub fn parse_key_value(token: &str) -> Result<(&str, &str), Box<dyn Error>>{
    let vec: Vec<&str> = token.split(':').collect();

    if vec.len() == 2 {
        let pair = (vec[0].trim(), vec[1].trim());
        Ok(pair)
    } else {
        Err(From::from("ParseError: expected a single ':'"))
    }
}

pub fn parse_open(kv_maps: HashMap<&str, &str>) -> Result<Event, Box<dyn Error>> {
    let pid: u64;
    let flags: u64;
    let inode: u64;

    match kv_maps.get(&"pid"){
        Some(v) => pid = v.parse::<u64>()?,
        _ => return Err(From::from("ParseError: pid not found"))
    }

    match kv_maps.get(&"flags"){
        Some(v) => flags = v.parse::<u64>()?,
        _ => return Err(From::from("ParseError: flags not found"))
    }

    match kv_maps.get(&"inode"){
        Some(v) => inode = v.parse::<u64>()?,
        _ => return Err(From::from("ParseError: inode not found"))
    }

    Ok(Event::Open {
        pid,
        flags,
        inode
    })
}

pub fn parse_close(kv_maps: HashMap<&str, &str>) -> Result<Event, Box<dyn Error>> {
    let pid: u64;
    let inode: u64;

    match kv_maps.get(&"pid"){
        Some(v) => pid = v.parse::<u64>()?,
        _ => return Err(From::from("ParseError: pid not found"))
    }

    match kv_maps.get(&"inode"){
        Some(v) => inode = v.parse::<u64>()?,
        _ => return Err(From::from("ParseError: inode not found"))
    }

    Ok(Event::Close {
        pid,
        inode,
    })
}

pub fn parse_mkdir(kv_maps: HashMap<&str, &str>) -> Result<Event, Box<dyn Error>> {
    let pid: u64;
    let path: String;
    let mode: u64;
    let inode: u64;
    let parent: u64;

    match kv_maps.get(&"pid"){
        Some(v) => pid = v.parse::<u64>()?,
        _ => return Err(From::from("ParseError: pid not found"))
    }

    match kv_maps.get(&"path"){
        Some(v) => path = v.to_string(),
        _ => return Err(From::from("ParseError: path not found"))
    }

    match kv_maps.get(&"mode"){
        Some(v) => mode = v.parse::<u64>()?,
        _ => return Err(From::from("ParseError: mode not found"))
    }

    match kv_maps.get(&"inode"){
        Some(v) => inode = v.parse::<u64>()?,
        _ => return Err(From::from("ParseError: inode not found"))
    }

    match kv_maps.get(&"parent"){
        Some(v) => parent = v.parse::<u64>()?,
        _ => return Err(From::from("ParseError: parent not found"))
    }

    Ok(Event::Mkdir {
        pid,
        path,
        mode,
        inode,
        parent,
    })
}

pub fn parse_create(kv_maps: HashMap<&str, &str>) -> Result<Event, Box<dyn Error>> {
    let pid: u64;
    let path: String;
    let mode: u64;
    let flags: u64;
    let inode: u64;
    let parent: u64;

    match kv_maps.get(&"pid"){
        Some(v) => pid = v.parse::<u64>()?,
        _ => return Err(From::from("ParseError: pid not found"))
    }

    match kv_maps.get(&"path"){
        Some(v) => path = v.to_string(),
        _ => return Err(From::from("ParseError: path not found"))
    }

    match kv_maps.get(&"mode"){
        Some(v) => mode = v.parse::<u64>()?,
        _ => return Err(From::from("ParseError: mode not found"))
    }

    match kv_maps.get(&"flags"){
        Some(v) => flags = v.parse::<u64>()?,
        _ => return Err(From::from("ParseError: flags not found"))
    }

    match kv_maps.get(&"inode"){
        Some(v) => inode = v.parse::<u64>()?,
        _ => return Err(From::from("ParseError: inode not found"))
    }

    match kv_maps.get(&"parent"){
        Some(v) => parent = v.parse::<u64>()?,
        _ => return Err(From::from("ParseError: parent not found"))
    }

    Ok(Event::Create {
        pid,
        path,
        mode,
        flags,
        inode,
        parent
    })
}

pub fn parse_symlink(kv_maps: HashMap<&str, &str>) -> Result<Event, Box<dyn Error>> {
    let pid: u64;
    let path_1: String;
    let path_2: String;

    match kv_maps.get(&"pid"){
        Some(v) => pid = v.parse::<u64>()?,
        _ => return Err(From::from("ParseError: pid not found"))
    }

    match kv_maps.get(&"path_1"){
        Some(v) => path_1 = v.to_string(),
        _ => return Err(From::from("ParseError: path_1 not found"))
    }

    match kv_maps.get(&"path_2"){
        Some(v) => path_2 = v.to_string(),
        _ => return Err(From::from("ParseError: path_2 not found"))
    }

    Ok(Event::SymLink {
        pid,
        path_1,
        path_2,
    })
}

pub fn parse_optional_inode(inode_str: &str) -> Result<Option<u64>, Box<dyn Error>> {
    let inode = inode_str.trim();
    if inode == "None" {
        Ok(None)
    } else {
        let inode = inode.strip_prefix("Some(");
        if inode.is_none() {
            return Err(From::from("ParseError: prefix 'Some(' not found"))
        }

        let inode = inode.unwrap().strip_suffix(")");
        if inode.is_none() {
            return Err(From::from("ParseError: suffix ')' not found"))
        }

        match inode.unwrap().parse::<u64>() {
            Ok(v) => Ok(Some(v)),
            _ => Err(From::from("ParseError: expect integer"))
        }
    }
}

pub fn parse_rename(line: String) -> Result<Event, Box<dyn Error>> {
    let pairs: &str;
    match line.strip_prefix("rename:") {
        Some(v) => pairs = v,
        _ => return Err(From::from("ParseError: prefix 'rename:' not found"))
    }

    let vec: Vec<&str> = pairs.split(',').collect();
    if vec.len() < 6 {
        return Err(From::from("ParseError: expect at least 6 variables"))
    }
    let parent_inode: u64 = vec[0].trim().parse::<u64>()?;
    let old_name: String = vec[1].trim().to_string();
    let newparent_inode: u64 = vec[2].trim().parse::<u64>()?;
    let new_name: String = vec[3].trim().to_string();
    let moved_inode: Option<u64> = parse_optional_inode(vec[4])?;
    let swapped_inode: Option<u64> = parse_optional_inode(vec[5])?;
    let overwritten_inode: Option<u64> = parse_optional_inode(vec[6])?;

    Ok(Event::Rename {
        parent_inode,
        old_name,
        newparent_inode,
        new_name,
        moved_inode,
        swapped_inode,
        overwritten_inode,
    })
}

pub fn parse_unlink(kv_maps: HashMap<&str, &str>) -> Result<Event, Box<dyn Error>> {
    let r#type: String;
    let pid: u64;
    let path: String;
    let inode: u64;
    let parent: u64;

    match kv_maps.get(&"type"){
        Some(v) => r#type = v.to_string(),
        _ => return Err(From::from("ParseError: type not found"))
    }
    match kv_maps.get(&"pid"){
        Some(v) => pid = v.parse::<u64>()?,
        _ => return Err(From::from("ParseError: pid not found"))
    }

    match kv_maps.get(&"path"){
        Some(v) => path = v.to_string(),
        _ => return Err(From::from("ParseError: path not found"))
    }

    match kv_maps.get(&"inode"){
        Some(v) => inode = v.parse::<u64>()?,
        _ => return Err(From::from("ParseError: inode not found"))
    }

    match kv_maps.get(&"parent"){
        Some(v) => parent = v.parse::<u64>()?,
        _ => return Err(From::from("ParseError: parent not found"))
    }

    Ok(Event::Unlink {
        r#type,
        pid,
        path,
        inode,
        parent
    })
}

pub fn parse_unlink_deleted(kv_maps: HashMap<&str, &str>) -> Result<Event, Box<dyn Error>> {
    let r#type: String;
    let pid: u64;
    let path: String;
    let inode: u64;
    let parent: u64;

    match kv_maps.get(&"type"){
        Some(v) => r#type = v.to_string(),
        _ => return Err(From::from("ParseError: type not found"))
    }
    match kv_maps.get(&"pid"){
        Some(v) => pid = v.parse::<u64>()?,
        _ => return Err(From::from("ParseError: pid not found"))
    }

    match kv_maps.get(&"path"){
        Some(v) => path = v.to_string(),
        _ => return Err(From::from("ParseError: path not found"))
    }

    match kv_maps.get(&"inode"){
        Some(v) => inode = v.parse::<u64>()?,
        _ => return Err(From::from("ParseError: inode not found"))
    }

    match kv_maps.get(&"parent"){
        Some(v) => parent = v.parse::<u64>()?,
        _ => return Err(From::from("ParseError: parent not found"))
    }

    Ok(Event::UnlinkDeleted {
        r#type,
        pid,
        path,
        inode,
        parent
    })
}

pub fn parse_event(line: &str) -> Result<Event, Box<dyn Error>> {
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
    if kv_maps.get(&"rename").is_some() { return parse_rename(line.to_string()) }

    Err(From::from("error"))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use super::*;

    #[test]
    fn test_parse_key_value(){
        // no key
        assert!(parse_key_value("test").is_err());
        assert!(parse_key_value("").is_err());
        assert!(parse_key_value(" ").is_err());
        assert!(parse_key_value(" test ").is_err());

        // ok
        assert!(parse_key_value("key:value").is_ok());
        assert_eq!(parse_key_value("key:value").unwrap(), ("key", "value"));

        // ok: empty value
        assert!(parse_key_value("key:").is_ok());
        assert_eq!(parse_key_value("key:").unwrap(), ("key", ""));

        // ok: prefix with spaces
        assert!(parse_key_value("key: value").is_ok());
        assert_eq!(parse_key_value("key: value").unwrap(), ("key", "value"));

        // ok: prefix with spacdes
        assert!(parse_key_value(" key: value").is_ok());
        assert_eq!(parse_key_value(" key: value").unwrap(), ("key", "value"));

        // ok: prefix/suffix with spaces
        assert!(parse_key_value(" key: value ").is_ok());
        assert_eq!(parse_key_value(" key: value ").unwrap(), ("key", "value"));

        // more than one delimiters
        assert!(parse_key_value("key: value: value2").is_err());
        assert!(parse_key_value("key::").is_err());
        assert!(parse_key_value("key: \"hello:\"").is_err());
    }

    #[test]
    fn test_parse_open(){
        // empty
        let kv_maps = HashMap::new();
        assert!(parse_open(kv_maps).is_err());

        // ok
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "111");
        kv_maps.insert("flags", "1");
        kv_maps.insert("inode", "1");
        let result = parse_open(kv_maps);
        let expected = Event::Open { pid: 111, flags: 1, inode: 1};
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);

        // extra
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "111");
        kv_maps.insert("flags", "1");
        kv_maps.insert("inode", "1");
        kv_maps.insert("extra", "112312312");
        let result = parse_open(kv_maps);
        let expected = Event::Open { pid: 111, flags: 1, inode: 1};
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);

        // non-int pid
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "aaa");
        kv_maps.insert("flags", "1");
        kv_maps.insert("inode", "1");
        let result = parse_open(kv_maps);
        assert!(result.is_err());

        // non-int flags
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "111");
        kv_maps.insert("flags", "aaa");
        kv_maps.insert("inode", "1");
        let result = parse_open(kv_maps);
        assert!(result.is_err());

        // non-int inode
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "111");
        kv_maps.insert("flags", "1");
        kv_maps.insert("inode", "aaa");
        let result = parse_open(kv_maps);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_close(){
        // empty
        let kv_maps = HashMap::new();
        assert!(parse_close(kv_maps).is_err());

        // ok
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "111");
        kv_maps.insert("inode", "1");
        let result = parse_close(kv_maps);
        let expected = Event::Close { pid: 111, inode: 1};
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);

        // extra
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "111");
        kv_maps.insert("inode", "1");
        kv_maps.insert("extra", "1sfdafd");
        let result = parse_close(kv_maps);
        let expected = Event::Close { pid: 111, inode: 1};
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);

        // non-int pid
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "aaa");
        kv_maps.insert("inode", "1");
        let result = parse_close(kv_maps);
        assert!(result.is_err());

        // non-int inode
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "111");
        kv_maps.insert("inode", "aaa");
        let result = parse_close(kv_maps);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_mkdir(){
        // empty
        let kv_maps = HashMap::new();
        assert!(parse_mkdir(kv_maps).is_err());

        // ok
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "111");
        kv_maps.insert("path", "test.txt");
        kv_maps.insert("mode", "1");
        kv_maps.insert("inode", "111");
        kv_maps.insert("parent", "1234");
        let result = parse_mkdir(kv_maps);
        let expected = Event::Mkdir { pid: 111,
                                            path: "test.txt".to_string(),
                                            mode: 1,
                                            inode: 111,
                                            parent: 1234,};
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);

        // extra
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "111");
        kv_maps.insert("path", "test.txt");
        kv_maps.insert("mode", "1");
        kv_maps.insert("inode", "111");
        kv_maps.insert("parent", "1234");
        kv_maps.insert("extra", "asdfsafk123123");
        let result = parse_mkdir(kv_maps);
        let expected = Event::Mkdir { pid: 111,
                                            path: "test.txt".to_string(),
                                            mode: 1,
                                            inode: 111,
                                            parent: 1234,};
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);

        // non-int pid
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "aa");
        kv_maps.insert("path", "test.txt");
        kv_maps.insert("mode", "1");
        kv_maps.insert("inode", "111");
        kv_maps.insert("parent", "1234");
        let result = parse_mkdir(kv_maps);
        assert!(result.is_err());

        // non-int mode
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "111");
        kv_maps.insert("path", "test.txt");
        kv_maps.insert("mode", "aa");
        kv_maps.insert("inode", "111");
        kv_maps.insert("parent", "1234");
        let result = parse_mkdir(kv_maps);
        assert!(result.is_err());

        // non-int inode
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "111");
        kv_maps.insert("path", "test.txt");
        kv_maps.insert("mode", "1");
        kv_maps.insert("inode", "aaa");
        kv_maps.insert("parent", "1234");
        let result = parse_mkdir(kv_maps);
        assert!(result.is_err());

        // non-int parent
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "111");
        kv_maps.insert("path", "test.txt");
        kv_maps.insert("mode", "1");
        kv_maps.insert("inode", "111");
        kv_maps.insert("parent", "aaa");
        let result = parse_mkdir(kv_maps);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_create(){
        // empty
        let kv_maps = HashMap::new();
        assert!(parse_create(kv_maps).is_err());

        // ok
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "111");
        kv_maps.insert("path", "test.txt");
        kv_maps.insert("mode", "1");
        kv_maps.insert("flags", "11");
        kv_maps.insert("inode", "111");
        kv_maps.insert("parent", "1234");
        let result = parse_create(kv_maps);
        let expected = Event::Create {
            pid: 111,
            path: "test.txt".to_string(),
            mode: 1,
            flags: 11,
            inode: 111,
            parent: 1234,
        };
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);

        // extra
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "111");
        kv_maps.insert("path", "test.txt");
        kv_maps.insert("mode", "1");
        kv_maps.insert("flags", "11");
        kv_maps.insert("inode", "111");
        kv_maps.insert("parent", "1234");
        kv_maps.insert("extra", "asdfsafk123123");
        let result = parse_create(kv_maps);
        let expected = Event::Create {
            pid: 111,
            path: "test.txt".to_string(),
            mode: 1,
            flags: 11,
            inode: 111,
            parent: 1234,
        };
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);

        // non-int pid
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "aa");
        kv_maps.insert("path", "test.txt");
        kv_maps.insert("mode", "1");
        kv_maps.insert("flags", "11");
        kv_maps.insert("inode", "111");
        kv_maps.insert("parent", "1234");
        let result = parse_create(kv_maps);
        assert!(result.is_err());

        // non-int mode
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "111");
        kv_maps.insert("path", "test.txt");
        kv_maps.insert("mode", "aa");
        kv_maps.insert("flags", "11");
        kv_maps.insert("inode", "111");
        kv_maps.insert("parent", "1234");
        let result = parse_create(kv_maps);
        assert!(result.is_err());

        // non-int flags
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "111");
        kv_maps.insert("path", "test.txt");
        kv_maps.insert("mode", "aa");
        kv_maps.insert("flags", "11");
        kv_maps.insert("inode", "111");
        kv_maps.insert("parent", "1234");
        let result = parse_create(kv_maps);
        assert!(result.is_err());

        // non-int inode
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "111");
        kv_maps.insert("path", "test.txt");
        kv_maps.insert("mode", "1");
        kv_maps.insert("flags", "11");
        kv_maps.insert("inode", "aaa");
        kv_maps.insert("parent", "1234");
        let result = parse_create(kv_maps);
        assert!(result.is_err());

        // non-int parent
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "111");
        kv_maps.insert("path", "test.txt");
        kv_maps.insert("mode", "1");
        kv_maps.insert("flags", "11");
        kv_maps.insert("inode", "111");
        kv_maps.insert("parent", "aaa");
        let result = parse_create(kv_maps);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_symlink(){
        // empty
        let kv_maps = HashMap::new();
        assert!(parse_symlink(kv_maps).is_err());

        // ok
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "111");
        kv_maps.insert("path_1", "test1.txt");
        kv_maps.insert("path_2", "test2.txt");
        let result = parse_symlink(kv_maps);
        let expected = Event::SymLink {
            pid: 111,
            path_1: "test1.txt".to_string(),
            path_2: "test2.txt".to_string(),
        };
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);

        // extra
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "111");
        kv_maps.insert("path_1", "test1.txt");
        kv_maps.insert("path_2", "test2.txt");
        kv_maps.insert("extra", "asdfsafk123123");
        let result = parse_symlink(kv_maps);
        let expected = Event::SymLink {
            pid: 111,
            path_1: "test1.txt".to_string(),
            path_2: "test2.txt".to_string(),
        };
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);

        // non-int pid
        let mut kv_maps = HashMap::new();
        kv_maps.insert("pid", "aaa");
        kv_maps.insert("path_1", "test1.txt");
        kv_maps.insert("path_2", "test2.txt");
        let result = parse_create(kv_maps);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_optional_inode(){
        // empty string
        let inode: &str = "";
        let result = parse_optional_inode(inode);
        assert!(result.is_err());

        // invalid string
        let inode_str = "abcd";
        let result = parse_optional_inode(inode_str);
        assert!(result.is_err());

        // Some(<inode_num>)
        let inode_str = "Some(abcd)";
        let result = parse_optional_inode(inode_str);
        assert!(result.is_err());

        // Some(<inode_num>)
        let inode_num = 2;
        let inode_str = "Some(2)";
        let result = parse_optional_inode(inode_str);
        match result {
            Ok(v) => assert_eq!(v, Some(inode_num)),
            _ => assert!(false)
        }

        // None
        let inode_str = "None";
        let result = parse_optional_inode(inode_str);
        match result {
            Ok(v) => assert_eq!(v, None),
            _ => assert!(false)
        }
    }

    #[test]
    fn test_parse_rename(){
        // empty
        let line = "".to_string();
        assert!(parse_rename(line).is_err());

        // missing values
        let line = "rename: 3, f1, 1, f3, Some(5)".to_string();
        let result = parse_rename(line);
        assert!(result.is_err());

        // ok
        let line = "rename: 3, f1, 1, f3, Some(5), None, Some(7)".to_string();
        let result = parse_rename(line);
        let expected = Event::Rename {
            parent_inode: 3,
            old_name: "f1".to_string(),
            newparent_inode: 1,
            new_name: "f3".to_string(),
            moved_inode: Some(5),
            swapped_inode: None,
            overwritten_inode: Some(7),
        };
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);

        //extra
        let line = "rename: 3, f1, 1, f3, Some(5), None, Some(7), f, 3".to_string();
        let result = parse_rename(line);
        let expected = Event::Rename {
            parent_inode: 3,
            old_name: "f1".to_string(),
            newparent_inode: 1,
            new_name: "f3".to_string(),
            moved_inode: Some(5),
            swapped_inode: None,
            overwritten_inode: Some(7),
        };
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);

        // non-int parent inode
        let line = "rename: x, f1, 1, f3, Some(5), None, Some(7)".to_string();
        let result = parse_rename(line);
        assert!(result.is_err());

        // non-int inode
        let line = "rename: 4, f1, x, f3, Some(5), None, Some(7)".to_string();
        let result = parse_rename(line);
        assert!(result.is_err());

        // non-int moved inode
        let line = "rename: 4, f1, 5, f3, Some(hello), None, Some(7)".to_string();
        let result = parse_rename(line);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unlink(){
        // empty
        let kv_maps = HashMap::new();
        assert!(parse_unlink(kv_maps).is_err());

        // ok
        let mut kv_maps = HashMap::new();
        kv_maps.insert("type", "file");
        kv_maps.insert("pid", "2");
        kv_maps.insert("path", "path_to_file");
        kv_maps.insert("inode", "2");
        kv_maps.insert("parent", "3");
        let result = parse_unlink(kv_maps);
        let expected = Event::Unlink {
            r#type: "file".to_string(),
            pid: 2,
            path: "path_to_file".to_string(),
            inode: 2,
            parent: 3,
        };
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);

        // extra
        let mut kv_maps = HashMap::new();
        kv_maps.insert("type", "file");
        kv_maps.insert("pid", "2");
        kv_maps.insert("path", "path_to_file");
        kv_maps.insert("inode", "2");
        kv_maps.insert("parent", "3");
        kv_maps.insert("extra", "4");
        let result = parse_unlink(kv_maps);
        let expected = Event::Unlink{
            r#type: "file".to_string(),
            pid: 2,
            path: "path_to_file".to_string(),
            inode: 2,
            parent: 3,
        };
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);

        // non-int pid
        let mut kv_maps = HashMap::new();
        kv_maps.insert("type", "file");
        kv_maps.insert("pid", "hello2");
        kv_maps.insert("path", "path_to_file");
        kv_maps.insert("inode", "2");
        kv_maps.insert("parent", "3");
        let result = parse_unlink(kv_maps);
        assert!(result.is_err());

        // non-int inode
        let mut kv_maps = HashMap::new();
        kv_maps.insert("type", "file");
        kv_maps.insert("pid", "2");
        kv_maps.insert("path", "path_to_file");
        kv_maps.insert("inode", "$$$");
        kv_maps.insert("parent", "3");
        let result = parse_unlink(kv_maps);
        assert!(result.is_err());

        //non-int parent
        let mut kv_maps = HashMap::new();
        kv_maps.insert("type", "file");
        kv_maps.insert("pid", "2");
        kv_maps.insert("path", "path_to_file");
        kv_maps.insert("inode", "2");
        kv_maps.insert("parent", "parent");
        let result = parse_unlink(kv_maps);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unlink_deleted(){
        // empty
        let kv_maps = HashMap::new();
        assert!(parse_unlink_deleted(kv_maps).is_err());

        // ok
        let mut kv_maps = HashMap::new();
        kv_maps.insert("type", "file");
        kv_maps.insert("pid", "2");
        kv_maps.insert("path", "path_to_file");
        kv_maps.insert("inode", "2");
        kv_maps.insert("parent", "3");
        let result = parse_unlink_deleted(kv_maps);
        let expected = Event::UnlinkDeleted {
            r#type: "file".to_string(),
            pid: 2,
            path: "path_to_file".to_string(),
            inode: 2,
            parent: 3,
        };
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);

        // extra
        let mut kv_maps = HashMap::new();
        kv_maps.insert("type", "file");
        kv_maps.insert("pid", "2");
        kv_maps.insert("path", "path_to_file");
        kv_maps.insert("inode", "2");
        kv_maps.insert("parent", "3");
        kv_maps.insert("extra", "4");
        let result = parse_unlink_deleted(kv_maps);
        let expected = Event::UnlinkDeleted {
            r#type: "file".to_string(),
            pid: 2,
            path: "path_to_file".to_string(),
            inode: 2,
            parent: 3,
        };
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected);

        // non-int pid
        let mut kv_maps = HashMap::new();
        kv_maps.insert("type", "file");
        kv_maps.insert("pid", "hello2");
        kv_maps.insert("path", "path_to_file");
        kv_maps.insert("inode", "2");
        kv_maps.insert("parent", "3");
        let result = parse_unlink_deleted(kv_maps);
        assert!(result.is_err());

        // non-int inode
        let mut kv_maps = HashMap::new();
        kv_maps.insert("type", "file");
        kv_maps.insert("pid", "2");
        kv_maps.insert("path", "path_to_file");
        kv_maps.insert("inode", "$$$");
        kv_maps.insert("parent", "3");
        let result = parse_unlink_deleted(kv_maps);
        assert!(result.is_err());

        //non-int parent
        let mut kv_maps = HashMap::new();
        kv_maps.insert("type", "file");
        kv_maps.insert("pid", "2");
        kv_maps.insert("path", "path_to_file");
        kv_maps.insert("inode", "2");
        kv_maps.insert("parent", "parent");
        let result = parse_unlink_deleted(kv_maps);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_event(){
        assert!(parse_event("op: open, pid: 0, flags: 0, inode: 0").is_ok());
        assert!(parse_event("op: close, pid: 0, inode: 0").is_ok());
        assert!(parse_event("op: close, pid: 0, inode: 0, random: 6666").is_ok());
        assert!(parse_event("op: create, pid: 0, path: hello.txt, mode: 33152, flags: 164034, inode: 0, parent:10").is_ok());
        assert!(parse_event("op: symlink, pid: 0, path_1: hello.txt, path_2: test.txt").is_ok());
        assert!(parse_event("rename: 3, f1, 1, f3, Some(5), None, Some(7)").is_ok());
        assert!(parse_event("op: unlink_deleted, type: file, pid: 38567432, path: delete_file, inode: 8, parent: 1").is_ok());
        assert!(parse_event("op: unlink, type: file, pid: 38567432, path: delete_file, inode: 8, parent: 1").is_ok());

        assert!(parse_event("").is_err());
        // Unknown op
        assert!(parse_event("op: new_op, hi: 1234").is_err());
        // Invalid key
        assert!(parse_event("op: open, id: 0, flags: 0, inode: None").is_err());
        // Missing a value
        assert!(parse_event("rename: 3, 1, f3, Some(5), None, Some(7)").is_err());
    }
}