use log::{debug, info};
use shared::{
    messages::{i_have_code::IHaveCode, ip_for_code::IpForCode, Message},
    send_msg,
};
#[cfg(target = "windows")]
use std::os::windows::prelude::FileExt;
use std::{
    error::{self, Error},
    fs::{remove_file, File, OpenOptions},
    io::Write,
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};
use tokio::{net::UdpSocket, time};

use crate::{ensure_global_ip, punch_hole, read_position, recv, write_position, SendMethod};

pub async fn reciever(
    code: String,
    sock: Arc<UdpSocket>,
    server_addr: SocketAddr,
    output: Option<String>,
    send_method: SendMethod,
) -> Result<(), Box<dyn Error>> {
    // Send message to server
    let i_have_code = IHaveCode::new(code);
    send_msg(&sock, &i_have_code, &server_addr).await?;

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
        }
        SendMethod::Confirm => todo!(),
        SendMethod::Index => {
            recv_file_index(&mut file, sock, ip, ip_for_code.file_len).await?;
        }
    }

    Ok(())
}

fn get_msg_num(msg_buf: &[u8]) -> u64 {
    let num_bytes: [u8; 8] = [
        msg_buf[0], msg_buf[1], msg_buf[2], msg_buf[3], msg_buf[4], msg_buf[5], msg_buf[6],
        msg_buf[7],
    ];

    let msg_num = u64::from_be_bytes(num_bytes);

    msg_num
}

async fn recv_file_burst(
    file: &mut File,
    sock: Arc<UdpSocket>,
    ip: SocketAddr,
) -> Result<(), Box<dyn error::Error>> {
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

fn get_pos_of_num(num: u64) -> (u64, u8) {
    let cell = num / 8;
    let cellpos = num % 8;

    (cell, cellpos as u8)
}

fn to_binary(mut num: u8) -> [bool; 8] {
    let mut arr = [false; 8];

    if num >= 128 {
        arr[0] = true;
        num -= 128;
    }
    if num >= 64 {
        arr[1] = true;
        num -= 64;
    }
    if num >= 32 {
        arr[2] = true;
        num -= 32;
    }
    if num >= 16 {
        arr[3] = true;
        num -= 16;
    }
    if num >= 8 {
        arr[4] = true;
        num -= 8;
    }
    if num >= 4 {
        arr[5] = true;
        num -= 4;
    }
    if num >= 2 {
        arr[6] = true;
        num -= 2;
    }
    if num >= 1 {
        arr[7] = true;
        // num -= 1;
    }

    // TODO make this function work

    arr
}

fn from_binary(bin: [bool; 8]) -> u8 {
    let mut num = 0;
    if bin[0] {
        num += 128;
    }
    if bin[1] {
        num += 64;
    }
    if bin[2] {
        num += 32;
    }
    if bin[3] {
        num += 16;
    }
    if bin[4] {
        num += 8;
    }
    if bin[5] {
        num += 4;
    }
    if bin[6] {
        num += 2;
    }
    if bin[7] {
        num += 1;
    }

    num
}

async fn recv_file_index(
    file: &mut File,
    sock: Arc<UdpSocket>,
    ip: SocketAddr,
    recv_size: u64,
) -> Result<(), Box<dyn error::Error>> {
    // TODO Create file for keeping track of messages
    // When the giver think it's done it should say that to the taker
    // the taker should check that it has recieved all packets
    // If not, the taker should send what messages are unsent
    // If there are too many for one message the other ones should be sent in the iteration

    // Create index file
    // TODO Check so that file doesn't already exist
    let index_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("filesender_recv_index")?;

    // Populate file with 0:s
    index_file.set_len(recv_size / 500 / 8 + 1)?;
    debug!("created index file");

    loop {
        let wait_time = time::sleep(Duration::from_millis(2000));
        tokio::select! {
            _ = wait_time => {
                info!("No message has been recieved for 2000ms, exiting!");
                break;
            }
            msg_buf = recv(&sock, &ip) => {
                let msg_buf = msg_buf?;
                debug!("got msg");

                // Skip if the first iteration is a hole punch msg
                if msg_buf.len() == 1 && msg_buf[0] == 255 {
                    debug!("msg was just a holepunch");
                    continue;
                }

                // Get msg num
                let msg_num = get_msg_num(&msg_buf);
                debug!("msg num: {}", msg_num);
                let msg_offset = get_offset(msg_num);

                // Write the data of the msg to file
                let rest = &msg_buf[8..];
                write_position(file, &rest, msg_offset).unwrap();
                debug!("wrote data");

                // Get position in index file
                let (offset, pos_in_offset) = get_pos_of_num(msg_num);
                debug!("offset: {}", offset);
                debug!("pos in offset: {}", pos_in_offset);

                // Read offset position from index file
                let mut offset_buf = [0u8; 1];
                read_position(&index_file, &mut offset_buf, offset)?;
                debug!("current offset data: {}", offset_buf[0]);

                // Change the offset
                let mut offset_binary = to_binary(offset_buf[0]);
                offset_binary[pos_in_offset as usize] = true;
                let offset_buf = from_binary(offset_binary);
                debug!("new offset data: {}", offset_buf);

                // Write the offset
                write_position(&index_file, &[offset_buf], offset)?;

                debug!("wrote new offset");
            }
        }
    }

    remove_file("filesender_recv_index")?;
    Ok(())
}
