use std::{net::SocketAddr, error::{Error, self}, fs::File, io::Write, sync::Arc, time::Duration, os::windows::prelude::FileExt, };

use log::info;
use shared::{messages::{i_have_code::IHaveCode, ip_for_code::IpForCode, Message}, send_msg};
use tokio::{net::UdpSocket, time};

use crate::{punch_hole, recv, ensure_global_ip, SendMethod};


pub async fn reciever(code: String, sock: Arc<UdpSocket>, server_addr: SocketAddr, output: Option<String>, send_method: SendMethod) -> Result<(), Box<dyn Error>> {
    // Send message to server
    let i_have_code = IHaveCode::new(code);
    send_msg(&sock, &i_have_code, server_addr).await?;

    let msg_buf = recv(&sock, &server_addr).await?;

    let ip_for_code = IpForCode::from_raw(msg_buf.as_slice())?;
    let ip = ensure_global_ip(ip_for_code.ip, &server_addr);
    info!("file name: {}", &ip_for_code.file_name);
    info!("file length: {}", &ip_for_code.file_len);
    info!("other ip: {}", &ip);

    // TODO Check if file fits on disk

    // Punch hole
    punch_hole(&sock, ip).await?;

    // Use custom output if specified
    // Else use the filename provided by the server
    let filename = match output {
        Some(filename) => filename,
        None => ip_for_code.file_name,
    };
    let mut file = File::create(filename).unwrap();

    info!("ready to recieve");

    match send_method {
        SendMethod::Burst => {
            recv_file_burst(&mut file, sock, ip).await?;
        },
        SendMethod::Confirm => todo!(),
        SendMethod::Index => {
            recv_file_index(&mut file, sock, ip).await?;
        },
    }
    
    Ok(())
}

fn get_msg_num(msg_buf: &[u8]) -> u64 {
    let num_bytes: [u8; 8] = [
        msg_buf[0],
        msg_buf[1],
        msg_buf[2],
        msg_buf[3],
        msg_buf[4],
        msg_buf[5],
        msg_buf[6],
        msg_buf[7]
    ];

    let msg_num = u64::from_be_bytes(num_bytes);

    msg_num
}

async fn recv_file_burst(file: &mut File, sock: Arc<UdpSocket>, ip: SocketAddr) -> Result<(), Box<dyn error::Error>> {
    loop {
        let wait_time = time::sleep(Duration::from_millis(2000));
        tokio::select! {
            _ = wait_time => {
                info!("No message has been recieved for 2000ms, exiting!");
                break;
            }
            msg_buf = recv(&sock, &ip) => {
                let msg_buf = msg_buf?;

                if msg_buf.len() == 1 && msg_buf[0] == 255 {
                    // Skip if the first iteration is a hole punch msg
                    continue;
                }

                file.write(&msg_buf).unwrap();
            }
        }
    }

    Ok(())
}

fn get_offset(msg_num: u64) -> u64 {
    msg_num * 500
}

fn write_msg_with_num(buf: &[u8], file: &mut File) {
    let msg_num = get_msg_num(buf);
    let msg_offset = get_offset(msg_num);

    let rest = &buf[8..];
    file.seek_write(&rest, msg_offset).unwrap();
}

async fn recv_file_index(file: &mut File, sock: Arc<UdpSocket>, ip: SocketAddr) -> Result<(), Box<dyn error::Error>> {
    // TODO Create file for keeping track of messages
    // When the giver think it's done it should say that to the taker
    // the taker should check that it has recieved all packets
    // If not, the taker should send what messages are unsent
    // If there are too many for one message the other ones should be sent in the iteration
    loop {
        let wait_time = time::sleep(Duration::from_millis(2000));
        tokio::select! {
            _ = wait_time => {
                info!("No message has been recieved for 2000ms, exiting!");
                break;
            }
            msg_buf = recv(&sock, &ip) => {
                let msg_buf = msg_buf?;

                if msg_buf.len() == 1 && msg_buf[0] == 255 {
                    // Skip if the first iteration is a hole punch msg
                    continue;
                }

                write_msg_with_num(&msg_buf, file);
            }
        }
    }

    Ok(())
}
