use std::collections::VecDeque;
use std::collections::HashMap;
use std::time;
use std::thread;
use std::sync::{Condvar, Mutex};
use once_cell::sync::Lazy;

static GLOBAL_LINES: Lazy<Mutex<VecDeque<VecDeque<String>>>> = Lazy::new(||Mutex::new(VecDeque::new()));

pub struct RwLock<T: ?Sized> {
    lines: Mutex<VecDeque<String>>,
    condvar: HashMap<String, Condvar>,
    lock: spin::RwLock<T>,
}

pub fn register_lines(all_lines: VecDeque<VecDeque<String>>) {
    let mut lock = (*GLOBAL_LINES).lock().unwrap();
    *lock = all_lines;
}


impl<T> RwLock<T> {
    pub fn new(user_data: T) -> Self {
        let mut map = HashMap::new();
        let lock = (*GLOBAL_LINES).lock();
        let lines = lock.unwrap().pop_front().unwrap();
        for line in lines.clone() {
            let mut split = line.split_whitespace();
            let _command = split.next().unwrap();
            let thread = split.next().unwrap();
            let tpid = String::from(thread);
            if !map.contains_key(&tpid) {
                map.insert(tpid, Condvar::new());
            }
        }
        RwLock {
            lines: Mutex::new(lines),
            condvar: map,
            lock: spin::RwLock::new(user_data),
        }

    }
}

impl<T: ?Sized> RwLock<T> {
    #[inline]
    pub fn read(&self) -> spin::RwLockReadGuard<T> {
        let mut waiting = true;
        while waiting {
            let line_lock = self.lines.lock().unwrap();
            let line = line_lock.front().unwrap();
            let mut split = line.split_whitespace();
            let command = split.next().unwrap();
            if command != "read_lock:" {
                //println!("not read");
                let curr = thread::current();
                let threadid = curr.name().unwrap();
                //let thread = thread::current().name().unwrap();
                let tpid = String::from(threadid);
                if let Some(var) = self.condvar.get(&tpid) {
                    var.wait(line_lock);
                }

                continue;
            }
            let next_thread = split.next().unwrap();
            if next_thread != thread::current().name().unwrap() {
                let curr = thread::current();
                let threadid = curr.name().unwrap();
                let tpid = String::from(threadid);
                if let Some(var) = self.condvar.get(&tpid) {
                    var.wait(line_lock);
                }
                continue;
            }
            waiting = false;
        }
        let mut line_lock = self.lines.lock().unwrap();
        let line = line_lock.pop_front();
        if let Some(line) = line_lock.front() {
            let mut split = line.split_whitespace();
            let command = split.next().unwrap();
            let next_thread = split.next().unwrap();
            let tpid = String::from(next_thread);
            if let Some(var) = self.condvar.get(&tpid) {
                var.notify_one();
            }
        }
        self.lock.read()
    }

    #[inline]
    pub fn write(&self) -> spin::RwLockWriteGuard<T> {
        let mut waiting = true;
        while waiting {
            let line_lock = self.lines.lock().unwrap();
            let line = line_lock.front().unwrap();
            let mut split = line.split_whitespace();
            let command = split.next().unwrap();
            if command != "write_lock:" {
                let curr = thread::current();
                let threadid = curr.name().unwrap();
                let tpid = String::from(threadid);
                if let Some(var) = self.condvar.get(&tpid) {
                    var.wait(line_lock);
                }
                continue;
            }
            let next_thread = split.next().unwrap();
            if next_thread != thread::current().name().unwrap() {
                let curr = thread::current();
                let threadid = curr.name().unwrap();
                let tpid = String::from(threadid);
                if let Some(var) = self.condvar.get(&tpid) {
                    var.wait(line_lock);
                }
                continue;
            }
            waiting = false;
        }
        let mut line_lock = self.lines.lock().unwrap();
        let line = line_lock.pop_front();
        if let Some(line) = line_lock.front() {
            let mut split = line.split_whitespace();
            let command = split.next().unwrap();
            let next_thread = split.next().unwrap();
            let tpid = String::from(next_thread);
            if let Some(var) = self.condvar.get(&tpid) {
                var.notify_one();
            }
        }
        self.lock.write()
    }
}
