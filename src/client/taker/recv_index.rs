use std::{
    error,
    fs::{remove_file, File, OpenOptions},
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};

use log::{debug, info};
use tokio::{net::UdpSocket, time};

use crate::{
    read_position, recv,
    taker::{from_binary, get_offset, get_pos_of_num, to_binary},
    write_position, u8s_to_u64,
};

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
        tokio::select! {
            _ = wait_time => {
                info!("No message has been recieved for 2000ms, exiting!");
                break;
            }
            msg_buf = recv(&sock, &ip) => {
                let msg_buf = msg_buf?;
                debug!("got msg with type: {}", msg_buf[0]);

                // Skip if the first iteration is a hole punch msg
                if msg_buf.len() == 1 && msg_buf[0] == 255 {
                    debug!("msg was just a holepunch");
                    continue;
                }
                else if msg_buf.len() == 1 && msg_buf[0] == 5 {
                    // Done sending
                    break;
                }

                // Get msg num
                let msg_num = u8s_to_u64(&msg_buf[0..8])?;

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
