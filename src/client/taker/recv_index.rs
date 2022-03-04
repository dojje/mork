use std::{
    error,
    fs::{remove_file, File, OpenOptions},
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};

use log::{debug, info};
use tokio::{net::UdpSocket, time};

use crate::{read_position, recv, u8s_to_u64, write_position};

fn get_offset(msg_num: u64) -> u64 {
    msg_num * 500
}

fn get_pos_of_num(num: u64) -> (u64, u8) {
    let cell = num / 8;
    let cellpos = num % 8;

    (cell, cellpos as u8)
}

fn get_num_of_pos(byte: u64, pos: u8) -> u64 {
    byte * 8 + pos as u64
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

pub async fn recv_file_index(
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
        let mut buf = [0; 508];
        tokio::select! {
            _ = wait_time => {
                info!("No message has been recieved for 2000ms, exiting!");
                break;
            }

            amt = recv(&sock, &ip, &mut buf) => {
                let amt = amt?;
                let buf = &buf[0..amt];

                debug!("got msg with type: {}", buf[0]);

                // Skip if the first iteration is a hole punch msg
                if buf.len() == 1 && buf[0] == 255 {
                    debug!("msg was just a holepunch");
                    continue;
                }
                else if buf.len() == 1 && buf[0] == 5 {
                    // Done sending
                    break;
                }

                // Get msg num
                let msg_num = u8s_to_u64(&buf[0..8])?;

                debug!("msg num: {}", msg_num);
                let msg_offset = get_offset(msg_num);

                // Write the data of the msg to file
                let rest = &buf[8..];
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

    // TODO Creat function for getting dropped messages
    let dropped = get_dropped("filesender_recv_index", recv_size)?;
    for drop in dropped {
        debug!("dropped: {}", drop);
    }

    remove_file("filesender_recv_index")?;
    Ok(())
}

fn get_dropped(index_filename: &str, file_len: u64) -> Result<Vec<u64>, Box<dyn error::Error>> {
    let file = File::open(index_filename)?;

    let mut dropped: Vec<u64> = Vec::new();

    // let mut byte = num / 8;
    // let mut pos = num % 8;

    for byte in 0..file.metadata()?.len() {
        // For every byte
        let mut buf = [0u8];
        read_position(&file, &mut buf, byte)?;
        let bin = to_binary(buf[0]);

        let mut bit_pos = 0;
        for bit in bin {
            let num = get_num_of_pos(byte, bit_pos);
            if num > file_len / 500 + 1 {
                // Return if it has checked every bit
                info!("num {} is too much", num);
                return Ok(dropped);
            }
            if !bit {
                dropped.push(num);
                if dropped.len() == 62 {
                    return Ok(dropped);
                }
            }

            bit_pos += 1;
        }
    }

    Ok(dropped)
}
