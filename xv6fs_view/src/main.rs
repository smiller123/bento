use std::net::*;
use std::io::{Read, Write};
use std::str;
use std::thread;
use std::time::Duration;
use std::sync::Arc;
use std::sync::Mutex;

const PRIMARY_PORT: u16 = 1234;
const HB_PORT: u16 = 8888;
const DEBUG: bool = false;

fn main() {
    println!("view server running..");

    let view_srv_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, PRIMARY_PORT);

    // bind to socket
    let view_listener = match TcpListener::bind(SocketAddr::V4(view_srv_addr)) {
        Ok(x) => x,
        Err(_) => {
            return;
        },
    };


    let primary_missed_hb = Arc::new(Mutex::new(0));
    let backup_missed_hb = Arc::new(Mutex::new(0));

    let hb_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, HB_PORT);
    let hb_listener = match TcpListener::bind(SocketAddr::V4(hb_addr)) {
        Ok(x) => x,
        Err(_) => {
            return;
        },
    };


    println!("accepting primary server..");
    // accept primary server connection
    let mut primary_srv_connection = match view_listener.accept() {
        Ok((stream, _)) => stream,
        Err(_) => {

            println!("primary accept error");
            return;
        }
    }; 
    println!("..OK");

    println!("setting primary hb connection..");
    accept_hb_connection(&hb_listener, &primary_missed_hb);
    println!("..OK");


    let mut is_primary_alive: bool = true;
    
    println!("accepting backup server..");
    // accept secondary server connection
    let mut backup_srv_connection = match view_listener.accept() {
        Ok((stream, _)) => stream,
        Err(_) => {

            println!("backup accept error");
            return;
        }
    };  
    println!("..OK");
    println!("setting backup hb connection..");
    accept_hb_connection(&hb_listener, &backup_missed_hb);
    println!("..OK");
    let mut is_backup_alive: bool = true;

    let mut count = 0;
    if DEBUG {
        loop {
            if is_primary_alive {
                let primary_count_missed = *primary_missed_hb.lock().unwrap();
                if primary_count_missed > 5 {
                    println!("primary died");
                    is_primary_alive = false;
                    primary_srv_connection.shutdown(Shutdown::Both);
                }
            } else {
                if count < 10 {
                    let msg = "test";
                    print!("count: {} - ", count);
                    match primary_srv_connection.write(msg.as_bytes()) {
                        Ok(x) => println!("write after primary died: {} ", x),
                        Err(_) => println!("err writing after primary died"),
                    };
                    count += 1;
                }
            }
            if is_backup_alive {
                let backup_count_missed = *backup_missed_hb.lock().unwrap();
                if backup_count_missed > 5 {
                    println!("backup died");
                    is_backup_alive = false;
                }
            }

        }
    }
    if DEBUG {
        let mut primary_buf = [0; 4096];
        let mut backup_buf = [0; 4096];

        let primary_read_size = match primary_srv_connection.read(&mut primary_buf) {
            Ok(x) if x == 0 => {
                x
            },
            Ok(x) => {
                x
            },
            Err(_) => {
                println!("error reading from primary server");
                return;
            },
        };
        if primary_read_size != 0 {
            let primary_buf_str = str::from_utf8(&primary_buf[0..primary_read_size]).unwrap();
            println!("From primary: {} ", primary_buf_str);
        }

        let backup_read_size = match backup_srv_connection.read(&mut backup_buf) {
            Ok(x) if x == 0 => {
                x
            },
            Ok(x) => {
                x
            },
            Err(_) => {
                println!("error reading from backup server");
                return;
            },
        };
        if backup_read_size != 0 {
            let backup_buf_str = str::from_utf8(&backup_buf[0..backup_read_size]).unwrap();
            println!("From backup: {} ", backup_buf_str);
        }
    }

    // accept client connection
    println!("accepting client connection");
    let mut client_connection = match view_listener.accept() {
        Ok((stream, _)) => stream,
        Err(_) => {

            println!("client accept error");
            return;
        }
    };  
    println!("..OK");

    println!("ready for operations..");
    // loop and listen to client commands
    while is_primary_alive || is_backup_alive {
        if is_primary_alive {
            let primary_count_missed = *primary_missed_hb.lock().unwrap();
            if primary_count_missed > 5 {
                println!("primary died");
                is_primary_alive = false;
                let _ = primary_srv_connection.shutdown(Shutdown::Both);
            }
        }  
        if is_backup_alive {
            let backup_count_missed = *backup_missed_hb.lock().unwrap();
            if backup_count_missed > 5 {
                println!("backup died");
                is_backup_alive = false;
                let _ = backup_srv_connection.shutdown(Shutdown::Both);
            }
        }
        // read from client
        let mut buf = [0; 4096];
        //connection = match 
        let size = match client_connection.read(&mut buf) {
            Ok(x) if x == 0 => {
                is_primary_alive = false;
                break;
            },
            Ok(x) => {
                x
            },
            Err(_) => {
                is_primary_alive = false;
                break;
            },
        };
        
        // send to primary
        if is_primary_alive {
            if is_backup_alive {
                match send_rcv_from_srv(&mut primary_srv_connection, &mut client_connection, &buf[0..size], false) {
                    Ok(_) => (),
                    Err(_) => {
                        is_primary_alive = false;
                    },
                };
            } else { // backup is dead, reply directly to client
                match send_rcv_from_srv(&mut primary_srv_connection, &mut client_connection, &buf[0..size], true) {
                    Ok(_) => (),
                    Err(_) => {
                        is_primary_alive = false;
                    },
                };

            }
        }
        if is_backup_alive {
            match send_rcv_from_srv(&mut backup_srv_connection, &mut client_connection, &buf[0..size], true) {
                Ok(_) => (),
                Err(_) => {
                    if is_primary_alive {
                        println!("backup stream err while primary is alive");
                    } else {
                        println!("backup stream err while backup is primary");
                    }
                },
            };
        }

    }
    
    let _ = client_connection.shutdown(Shutdown::Both);
    let _ = primary_srv_connection.shutdown(Shutdown::Both);
    let _ = backup_srv_connection.shutdown(Shutdown::Both);
    println!("shutting down..");
}

fn accept_hb_connection (hb_listener: &TcpListener, missed_hb: &Arc<Mutex<u32>>) {
    println!("accepting hb connection..");
    let hb_connection = match hb_listener.accept() {
        Ok((stream, _)) => Some(stream),
        Err(_) => {
            println!("..FAILED");
            return;
        }
    };
    let missed_hb_clone = missed_hb.clone();
    thread::spawn(move || {
        let hb_connection = Some(hb_connection.unwrap());
        let hb_missed_count = missed_hb_clone;
        println!("hb thread running..");
        loop {
            let mut hb_buf = [0; 4096];
            let hb_read_size = match hb_connection.as_ref().unwrap().read(&mut hb_buf) {
                Ok(x) if x == 0 => 0,
                Ok(x) => {
                    x
                },
                Err(_) => {
                    let _ = hb_connection.unwrap().shutdown(Shutdown::Both);
                    println!("read from primary hb failed");
                    return;
                },
            };
            if hb_read_size == 0 {
                // increase missed heart beat
                *hb_missed_count.lock().unwrap() += 1;

            } else {

                *hb_missed_count.lock().unwrap() = 0;
            }
            thread::sleep(Duration::from_micros(1000));
        }

    });
}

fn send_rcv_from_srv(srv_stream: &mut TcpStream, client_stream: &mut TcpStream, msg_bytes: &[u8], reply_to_client: bool) -> Result<(), ()> {
    // send op to srv
    match srv_stream.write(msg_bytes) {
        Ok(_) => (),
        Err(_) => return Err(()),
    };

    // get result from backup
    let mut srv_resp = [0 as u8; 4096];
    let srv_resp_size = match srv_stream.read(&mut srv_resp) {
        Ok(x) => x,
        Err(_) => {
            println!("srv_stream - read err");
            return Err(());
        }
    };

    if srv_resp_size == 0 {
        return Err(());
    }
    // if the srv is primary, then reply to client
    if reply_to_client {
        client_stream.write(&srv_resp[0..srv_resp_size]);
    }

    Ok(())
}
