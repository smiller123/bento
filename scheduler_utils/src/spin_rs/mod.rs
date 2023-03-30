//use spin::RwLock;
use std::collections::VecDeque;
use std::time;
use std::thread;
use std::sync::{Condvar, Mutex};

pub struct RwLock<T: ?Sized> {
    //lines: spin::RwLock<VecDeque<String>>,
    lines: Mutex<VecDeque<String>>,
    condvar: Condvar,
    lock: spin::RwLock<T>,
    //lock: UnsafeCell<Option<RsRwSemaphore>>,
    //data: UnsafeCell<T>,
}

impl<T> RwLock<T> {
    pub fn new(user_data: T, lines: VecDeque<String>) -> Self {
        RwLock {
            //lines: spin::RwLock::new(lines),
            lines: Mutex::new(lines),
            condvar: Condvar::new(),
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
                self.condvar.wait(line_lock);
                //let dur = time::Duration::from_millis(10);
                //thread::sleep(dur);
                continue;
            }
            let next_thread = split.next().unwrap();
            if next_thread != thread::current().name().unwrap() {
                //println!("read expected got {} {}", thread::current().name().unwrap(), line);
                self.condvar.wait(line_lock);
                //let dur = time::Duration::from_millis(10);
                //thread::sleep(dur);
                //println!("waking up");
                continue;
            }
            waiting = false;
        }
        let mut line_lock = self.lines.lock().unwrap();
        let line = line_lock.pop_front();
        //println!("read locking {:?}", line);
        self.condvar.notify_all();
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
                self.condvar.wait(line_lock);
                //let dur = time::Duration::from_millis(10);
                //thread::sleep(dur);
                continue;
            }
            let next_thread = split.next().unwrap();
            if next_thread != thread::current().name().unwrap() {
                //println!("write expected {}", next_thread);
                self.condvar.wait(line_lock);
                //let dur = time::Duration::from_millis(10);
                //thread::sleep(dur);
                //println!("waking up");
                continue;
            }
            //self.lines.pop_front();
            waiting = false;
        }
        let mut line_lock = self.lines.lock().unwrap();
        let line = line_lock.pop_front();
        //println!("write locking {:?}", line);
        self.condvar.notify_all();
        self.lock.write()
        //unsafe {
        //    let _ = down_read(&*self.lock.get());
        //}
        //Ok(RwLockReadGuard {
        //    lock: self,
        //})
    }
}
