use log::{info};
use shared::{
    messages::{i_have_code::IHaveCode, ip_for_code::IpForCode, Message},
    send_msg,
};

#[cfg(target = "windows")]
use std::os::windows::prelude::FileExt;

use std::{
    error::{Error},
    fs::{File},
    net::SocketAddr,
    sync::Arc,
};
use tokio::{net::UdpSocket};

use crate::{
    ensure_global_ip, punch_hole, recv,
    taker::{recv_burst::recv_file_burst, recv_index::recv_file_index}, SendMethod,
};

mod recv_burst;
mod recv_index;

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
