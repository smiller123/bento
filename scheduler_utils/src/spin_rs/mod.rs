//use spin::RwLock;
use std::collections::VecDeque;
use std::collections::HashMap;
use std::time;
use std::thread;
use std::sync::{Condvar, Mutex};
use once_cell::sync::Lazy;

static GLOBAL_LINES: Lazy<Mutex<VecDeque<VecDeque<String>>>> = Lazy::new(||Mutex::new(VecDeque::new()));

pub struct RwLock<T: ?Sized> {
    //lines: spin::RwLock<VecDeque<String>>,
    lines: Mutex<VecDeque<String>>,
    condvar: HashMap<String, Condvar>,
    lock: spin::RwLock<T>,
    //lock: UnsafeCell<Option<RsRwSemaphore>>,
    //data: UnsafeCell<T>,
}

pub fn register_lines(all_lines: VecDeque<VecDeque<String>>) {
    let mut lock = (*GLOBAL_LINES).lock().unwrap();
    *lock = all_lines;
}


impl<T> RwLock<T> {
    //pub fn new(user_data: T, lines: VecDeque<String>) -> Self {
    pub fn new(user_data: T) -> Self {
        let mut map = HashMap::new();
        let lock = (*GLOBAL_LINES).lock();
        let lines = lock.unwrap().pop_front().unwrap();
        for line in lines.clone() {
            let mut split = line.split_whitespace();
            let _command = split.next().unwrap();
            let thread = split.next().unwrap();
            //println!("tpid {}", thread);
            let tpid = String::from(thread);
            if !map.contains_key(&tpid) {
                map.insert(tpid, Condvar::new());
            }
        }
        RwLock {
            //lines: spin::RwLock::new(lines),
            lines: Mutex::new(lines),
            //condvar: Condvar::new(),
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
            //let line = {
            //let line_lock = self.lines.read();
            let line_lock = self.lines.lock().unwrap();
            let line = line_lock.front().unwrap();
              //  let line = line_lock.front().unwrap().clone();
             //   line
            //};
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

                //self.condvar.wait(line_lock);
                //let dur = time::Duration::from_millis(10);
                //thread::sleep(dur);
                continue;
            }
            let next_thread = split.next().unwrap();
            if next_thread != thread::current().name().unwrap() {
                //println!("read expected got {} {}", thread::current().name().unwrap(), line);
                let curr = thread::current();
                let threadid = curr.name().unwrap();
                //let thread = thread::current().name().unwrap();
                let tpid = String::from(threadid);
                //let tpid = thread::current().name().unwrap();
                if let Some(var) = self.condvar.get(&tpid) {
                    var.wait(line_lock);
                }
                //self.condvar.wait(line_lock);
                //let dur = time::Duration::from_millis(10);
                //thread::sleep(dur);
                //println!("waking up");
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
            //let tpid: u64 = next_thread.parse().unwrap();
            //println!("read locking {:?}", line);
            if let Some(var) = self.condvar.get(&tpid) {
                var.notify_one();
            }
        }
        //self.condvar.notify_all();
        self.lock.read()
        //unsafe {
        //    let _ = down_read(&*self.lock.get());
        //}
        //Ok(RwLockReadGuard {
        //    lock: self,
        //})
    }

    #[inline]
    pub fn write(&self) -> spin::RwLockWriteGuard<T> {
        let mut waiting = true;
        while waiting {
        //    let line = {
            //let line_lock = self.lines.read();
            let line_lock = self.lines.lock().unwrap();
            //let line = line_lock.front().unwrap().clone();
            let line = line_lock.front().unwrap();
         //       line
          //  };
            //let line_lock = self.lines.read();
            //let line = line_lock.front();
            //println!("expected {}", line.unwrap());
            let mut split = line.split_whitespace();
            let command = split.next().unwrap();
            if command != "write_lock:" {
                //println!("not write");
                //let tpid: u64 = thread::current().name().unwrap().parse().unwrap();
                let curr = thread::current();
                let threadid = curr.name().unwrap();
                //let threadid = thread::current().name().unwrap();
                let tpid = String::from(threadid);
                if let Some(var) = self.condvar.get(&tpid) {
                    var.wait(line_lock);
                }
                //self.condvar.wait(line_lock);
                //let dur = time::Duration::from_millis(10);
                //thread::sleep(dur);
                continue;
            }
            let next_thread = split.next().unwrap();
            if next_thread != thread::current().name().unwrap() {
                //println!("write expected {}", next_thread);
                //let tpid: u64 = thread::current().name().unwrap().parse().unwrap();
                let curr = thread::current();
                let threadid = curr.name().unwrap();
                let tpid = String::from(threadid);
                if let Some(var) = self.condvar.get(&tpid) {
                    var.wait(line_lock);
                }
                //self.condvar.wait(line_lock);
                //let dur = time::Duration::from_millis(10);
                //thread::sleep(dur);
                //println!("waking up");
                continue;
            }
            //self.lines.pop_front();
            waiting = false;
        }
        //let mut line_lock = self.lines.lock().unwrap();
        //let line = line_lock.pop_front();
        //println!("write locking {:?}", line);
        //self.condvar.notify_all();
        let mut line_lock = self.lines.lock().unwrap();
        let line = line_lock.pop_front();
        if let Some(line) = line_lock.front() {
            let mut split = line.split_whitespace();
            let command = split.next().unwrap();
            let next_thread = split.next().unwrap();
            //let tpid: u64 = next_thread.parse().unwrap();
            let tpid = String::from(next_thread);
            //println!("read locking {:?}", line);
            if let Some(var) = self.condvar.get(&tpid) {
                var.notify_one();
            }
        }
        self.lock.write()
        //unsafe {
        //    let _ = down_read(&*self.lock.get());
        //}
        //Ok(RwLockReadGuard {
        //    lock: self,
        //})
    }
}
