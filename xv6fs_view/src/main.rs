use std::net::*;
use std::io::{Read, Write};
use std::str;

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

        //connection = match 
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
    loop {

        // read from client
        let mut buf = [0; 4096];
        //connection = match 
        let size = match client_connection.read(&mut buf) {
            Ok(x) if x == 0 => {
                break;
            },
            Ok(x) => {
                x
            },
            Err(_) => {
                let _ = client_connection.shutdown(Shutdown::Both);
                break;
            },
        };
        let buf_str = str::from_utf8(&buf[0..size]).unwrap();
        
        // send to primary
        if is_primary_alive {
            match send_rcv_from_srv(&mut primary_srv_connection, &mut client_connection, &buf[0..size], false) {
                Ok(_) => (),
                Err(_) => {
                    is_primary_alive = false;
                },
            };
        }
        if is_primary_alive {
            match send_rcv_from_srv(&mut backup_srv_connection, &mut client_connection, &buf[0..size], true) {
                Ok(_) => (),
                Err(_) => {
                    println!("backup stream err while primary is alive");
                },
            };
        } else { // backup is now primary
             match send_rcv_from_srv(&mut backup_srv_connection, &mut client_connection, &buf[0..size], true) {
                Ok(_) => (),
                Err(_) => {
                   println!("backup stream err while backup is primary");
                },
            };
           
        }
    

    }
    
    let _ = client_connection.shutdown(Shutdown::Both);
    let _ = primary_srv_connection.shutdown(Shutdown::Both);
    let _ = backup_srv_connection.shutdown(Shutdown::Both);
    println!("shutting down..");
}

fn send_rcv_from_srv(srv_stream: &mut TcpStream, client_stream: &mut TcpStream, msg_bytes: &[u8], reply_to_client: bool) -> Result<(), ()> {
    // send op to srv
    srv_stream.write(msg_bytes);

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
 //   let op_msg = str::from_utf8(&srv_resp[0..srv_resp_size]).unwrap();
    //let op_vec: Vec<&str> = op_msg.split(' ').collect();
//    match *op_vec.get(0).unwrap() {
        //"Ok" => {
            //return Ok(());
        //},
        //"Add" => {
            //return Ok(());
        //},
        //"Err" => {
            //println!("backup op - Err 1");
            //let _ = client_stream.write(&srv_resp[0..srv_resp_size]);
            //return Err(());
        //},
        //_ => {
            //println!("backup op_msg: {}", op_msg);
            //let _ = client_stream.write(&srv_resp[0..srv_resp_size]);
            //return Err(());
        //},

    //};
}
//fn read_from_srv()
