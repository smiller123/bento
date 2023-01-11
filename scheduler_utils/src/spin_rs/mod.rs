//use spin::RwLock;
use std::collections::VecDeque;
use std::time;
use std::thread;

pub struct RwLock<T: ?Sized> {
    lines: spin::RwLock<VecDeque<String>>,
    lock: spin::RwLock<T>,
    //lock: UnsafeCell<Option<RsRwSemaphore>>,
    //data: UnsafeCell<T>,
}

impl<T> RwLock<T> {
    pub fn new(user_data: T, lines: VecDeque<String>) -> Self {
        RwLock {
            lines: spin::RwLock::new(lines),
            lock: spin::RwLock::new(user_data),
        }
    }
}

impl<T: ?Sized> RwLock<T> {
    #[inline]
    pub fn read(&self) -> spin::RwLockReadGuard<T> {
        let mut waiting = true;
        while waiting {
            let line_lock = self.lines.read();
            let line = line_lock.front();
            let mut split = line.unwrap().split_whitespace();
            let command = split.next().unwrap();
            if command != "read_lock:" {
                let dur = time::Duration::from_millis(10);
                thread::sleep(dur);
                continue;
            }
            let next_thread = split.next().unwrap();
            if next_thread != thread::current().name().unwrap() {
                let dur = time::Duration::from_millis(10);
                thread::sleep(dur);
                println!("waking up");
                continue;
            }
            waiting = false;
        }
        let mut line_lock = self.lines.write();
        line_lock.pop_front();
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
            let line_lock = self.lines.read();
            let line = line_lock.front();
            println!("expected {}", line.unwrap());
            let mut split = line.unwrap().split_whitespace();
            let command = split.next().unwrap();
            if command != "write_lock:" {
                let dur = time::Duration::from_millis(10);
                thread::sleep(dur);
                continue;
            }
            let next_thread = split.next().unwrap();
            if next_thread != thread::current().name().unwrap() {
                println!("expected {}, got {}", next_thread, thread::current().name().unwrap());
                let dur = time::Duration::from_millis(10);
                thread::sleep(dur);
                println!("waking up");
                continue;
            }
            //self.lines.pop_front();
            waiting = false;
        }
        let mut line_lock = self.lines.write();
        line_lock.pop_front();
        self.lock.write()
        //unsafe {
        //    let _ = down_read(&*self.lock.get());
        //}
        //Ok(RwLockReadGuard {
        //    lock: self,
        //})
    }
}
