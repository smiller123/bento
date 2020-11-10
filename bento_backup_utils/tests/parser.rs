// use std::error::Error;
use std::collections::HashMap;

#[path = "../src/parser.rs"] mod parser;

#[test]
fn test_parse_key_value(){
    // no key
    assert!(parser::parse_key_value("test").is_err());
    assert!(parser::parse_key_value("").is_err());
    assert!(parser::parse_key_value(" ").is_err());
    assert!(parser::parse_key_value(" test ").is_err());

    // ok
    assert!(parser::parse_key_value("key:value").is_ok());
    assert_eq!(parser::parse_key_value("key:value").unwrap(), ("key", "value"));

    // ok: empty value
    assert!(parser::parse_key_value("key:").is_ok());
    assert_eq!(parser::parse_key_value("key:").unwrap(), ("key", ""));

    // ok: prefix with spaces
    assert!(parser::parse_key_value("key: value").is_ok());
    assert_eq!(parser::parse_key_value("key: value").unwrap(), ("key", "value"));

    // ok: prefix with spacdes
    assert!(parser::parse_key_value(" key: value").is_ok());
    assert_eq!(parser::parse_key_value(" key: value").unwrap(), ("key", "value"));

    // ok: prefix/suffix with spaces
    assert!(parser::parse_key_value(" key: value ").is_ok());
    assert_eq!(parser::parse_key_value(" key: value ").unwrap(), ("key", "value"));

    // more than one delimiters
    assert!(parser::parse_key_value("key: value: value2").is_err());
    assert!(parser::parse_key_value("key::").is_err());
    assert!(parser::parse_key_value("key: \"hello:\"").is_err());
}

#[test]
fn test_parse_open(){
    // empty
    let kv_maps = HashMap::new();
    assert!(parser::parse_open(kv_maps).is_err());

    // ok
    let mut kv_maps = HashMap::new();
    kv_maps.insert("pid", "111");
    kv_maps.insert("flags", "1");
    kv_maps.insert("inode", "1");
    let result = parser::parse_open(kv_maps);
    let expected = parser::Event::Open { pid: 111, flags: 1, inode: 1};
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), expected);

    // extra
    let mut kv_maps = HashMap::new();
    kv_maps.insert("pid", "111");
    kv_maps.insert("flags", "1");
    kv_maps.insert("inode", "1");
    kv_maps.insert("extra", "112312312");
    let result = parser::parse_open(kv_maps);
    let expected = parser::Event::Open { pid: 111, flags: 1, inode: 1};
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), expected);

    // non-int pid
    let mut kv_maps = HashMap::new();
    kv_maps.insert("pid", "aaa");
    kv_maps.insert("flags", "1");
    kv_maps.insert("inode", "1");
    let result = parser::parse_open(kv_maps);
    assert!(result.is_err());

    // non-int flags
    let mut kv_maps = HashMap::new();
    kv_maps.insert("pid", "111");
    kv_maps.insert("flags", "aaa");
    kv_maps.insert("inode", "1");
    let result = parser::parse_open(kv_maps);
    assert!(result.is_err());

    // non-int inode
    let mut kv_maps = HashMap::new();
    kv_maps.insert("pid", "111");
    kv_maps.insert("flags", "1");
    kv_maps.insert("inode", "aaa");
    let result = parser::parse_open(kv_maps);
    assert!(result.is_err());
}

#[test]
fn test_parse_close(){
    // empty
    let kv_maps = HashMap::new();
    assert!(parser::parse_close(kv_maps).is_err());

    // ok
    let mut kv_maps = HashMap::new();
    kv_maps.insert("pid", "111");
    kv_maps.insert("inode", "1");
    let result = parser::parse_close(kv_maps);
    let expected = parser::Event::Close { pid: 111, inode: 1};
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), expected);

    // extra
    let mut kv_maps = HashMap::new();
    kv_maps.insert("pid", "111");
    kv_maps.insert("inode", "1");
    kv_maps.insert("extra", "1sfdafd");
    let result = parser::parse_close(kv_maps);
    let expected = parser::Event::Close { pid: 111, inode: 1};
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), expected);

    // non-int pid
    let mut kv_maps = HashMap::new();
    kv_maps.insert("pid", "aaa");
    kv_maps.insert("inode", "1");
    let result = parser::parse_close(kv_maps);
    assert!(result.is_err());

    // non-int inode
    let mut kv_maps = HashMap::new();
    kv_maps.insert("pid", "111");
    kv_maps.insert("inode", "aaa");
    let result = parser::parse_close(kv_maps);
    assert!(result.is_err());
}

#[test]
fn test_parse_mkdir(){
    // empty
    let kv_maps = HashMap::new();
    assert!(parser::parse_mkdir(kv_maps).is_err());

    // ok
    let mut kv_maps = HashMap::new();
    kv_maps.insert("pid", "111");
    kv_maps.insert("path", "test.txt");
    kv_maps.insert("mode", "1");
    kv_maps.insert("inode", "111");
    kv_maps.insert("parent", "1234");
    let result = parser::parse_mkdir(kv_maps);
    let expected = parser::Event::Mkdir { pid: 111,
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
    let result = parser::parse_mkdir(kv_maps);
    let expected = parser::Event::Mkdir { pid: 111,
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
    let result = parser::parse_mkdir(kv_maps);
    assert!(result.is_err());

    // non-int mode
    let mut kv_maps = HashMap::new();
    kv_maps.insert("pid", "111");
    kv_maps.insert("path", "test.txt");
    kv_maps.insert("mode", "aa");
    kv_maps.insert("inode", "111");
    kv_maps.insert("parent", "1234");
    let result = parser::parse_mkdir(kv_maps);
    assert!(result.is_err());

    // non-int inode
    let mut kv_maps = HashMap::new();
    kv_maps.insert("pid", "111");
    kv_maps.insert("path", "test.txt");
    kv_maps.insert("mode", "1");
    kv_maps.insert("inode", "aaa");
    kv_maps.insert("parent", "1234");
    let result = parser::parse_mkdir(kv_maps);
    assert!(result.is_err());

    // non-int parent
    let mut kv_maps = HashMap::new();
    kv_maps.insert("pid", "111");
    kv_maps.insert("path", "test.txt");
    kv_maps.insert("mode", "1");
    kv_maps.insert("inode", "111");
    kv_maps.insert("parent", "aaa");
    let result = parser::parse_mkdir(kv_maps);
    assert!(result.is_err());
}

#[test]
fn test_parse_create(){
    // empty
    let kv_maps = HashMap::new();
    assert!(parser::parse_create(kv_maps).is_err());

    // ok
    let mut kv_maps = HashMap::new();
    kv_maps.insert("pid", "111");
    kv_maps.insert("path", "test.txt");
    kv_maps.insert("mode", "1");
    kv_maps.insert("flags", "11");
    kv_maps.insert("inode", "111");
    kv_maps.insert("parent", "1234");
    let result = parser::parse_create(kv_maps);
    let expected = parser::Event::Create {
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
    let result = parser::parse_create(kv_maps);
    let expected = parser::Event::Create {
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
    let result = parser::parse_create(kv_maps);
    assert!(result.is_err());

    // non-int mode
    let mut kv_maps = HashMap::new();
    kv_maps.insert("pid", "111");
    kv_maps.insert("path", "test.txt");
    kv_maps.insert("mode", "aa");
    kv_maps.insert("flags", "11");
    kv_maps.insert("inode", "111");
    kv_maps.insert("parent", "1234");
    let result = parser::parse_create(kv_maps);
    assert!(result.is_err());

    // non-int flags
    let mut kv_maps = HashMap::new();
    kv_maps.insert("pid", "111");
    kv_maps.insert("path", "test.txt");
    kv_maps.insert("mode", "aa");
    kv_maps.insert("flags", "11");
    kv_maps.insert("inode", "111");
    kv_maps.insert("parent", "1234");
    let result = parser::parse_create(kv_maps);
    assert!(result.is_err());

    // non-int inode
    let mut kv_maps = HashMap::new();
    kv_maps.insert("pid", "111");
    kv_maps.insert("path", "test.txt");
    kv_maps.insert("mode", "1");
    kv_maps.insert("flags", "11");
    kv_maps.insert("inode", "aaa");
    kv_maps.insert("parent", "1234");
    let result = parser::parse_create(kv_maps);
    assert!(result.is_err());

    // non-int parent
    let mut kv_maps = HashMap::new();
    kv_maps.insert("pid", "111");
    kv_maps.insert("path", "test.txt");
    kv_maps.insert("mode", "1");
    kv_maps.insert("flags", "11");
    kv_maps.insert("inode", "111");
    kv_maps.insert("parent", "aaa");
    let result = parser::parse_create(kv_maps);
    assert!(result.is_err());
}

#[test]
fn test_parse_symlink(){
    // empty
    let kv_maps = HashMap::new();
    assert!(parser::parse_symlink(kv_maps).is_err());

    // ok
    let mut kv_maps = HashMap::new();
    kv_maps.insert("pid", "111");
    kv_maps.insert("path_1", "test1.txt");
    kv_maps.insert("path_2", "test2.txt");
    let result = parser::parse_symlink(kv_maps);
    let expected = parser::Event::SymLink {
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
    let result = parser::parse_symlink(kv_maps);
    let expected = parser::Event::SymLink {
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
    let result = parser::parse_create(kv_maps);
    assert!(result.is_err());
}

#[test]
fn parse_optional_inode(){

    // empty string
    let inode: &str = "";
    let result = parser::parse_optional_inode(inode);
    assert!(result.is_none());

    // Some(<inode_num>)
    let inode_num = 2;
    let inode_str = "Some(2)";
    let result = parser::parse_optional_inode(inode_str);
    match result {
        Some(v) => assert_eq!(v, inode_num),
        _ => assert!(false)
    }

    // None
    let inode_str = "None";
    let result = parser::parse_optional_inode(inode_str);
    assert!(result.is_none());
    
    // invalid string
    let inode_str = "abcd";
    let result = parser::parse_optional_inode(inode_str);
    assert!(result.is_none());
}

#[test]
fn parse_rename(){
    // TODO
}

#[test]
fn parse_unlink(){
    // TODO
}

#[test]
fn parse_unlink_deleted(){
    // TODO
}

#[test]
fn parse_event(){
    // TODO
    assert!(parser::parse_event("op: open, pid: 0, flags: 0, inode: 0").is_ok());
    assert!(parser::parse_event("op: close, pid: 0, inode: 0").is_ok());
    assert!(parser::parse_event("op: close, pid: 0, inode: 0, random: 6666").is_ok());
    assert!(parser::parse_event("op: create, pid: 0, path: hello.txt, mode: 33152, flags: 164034, inode: 0, parent:10").is_ok());
    assert!(parser::parse_event("op: symlink, pid: 0, path_1: hello.txt, path_2: test.txt").is_ok());
}
