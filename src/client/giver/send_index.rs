use std::{error, fs::File, net::SocketAddr, sync::Arc, thread, time::Duration};

use log::info;
use tokio::net::UdpSocket;

use crate::{giver::get_buf, punch_hole, read_position, recv, u8s_to_u64};

fn get_file_buf_from_msg_num(
    msg: u64,
    file: &File,
    buf_size: u64,
    buf: &mut [u8],
) -> Result<usize, Box<dyn error::Error>> {
    let amt = read_position(&file, buf, msg * buf_size)?;

    Ok(amt)
}

pub async fn send_file_index(
    sock: Arc<UdpSocket>,
    file_name: String,
    reciever: SocketAddr,
) -> Result<(), Box<dyn error::Error>> {
    info!("reciever ip: {}", reciever);

    punch_hole(&sock, reciever).await?;

    thread::sleep(Duration::from_millis(1000));

    // Udp messages should be 508 bytes
    // 8 of those bytes are used for checking order of recieving bytes
    // The rest 500 bytes are used to send the file
    // The file gets send 500 bytes
    let input_file = File::open(file_name)?;
    let file_len = input_file.metadata()?.len();
    let mut offset = 0;
    let mut msg_num: u64 = 0;
    info!(
        "will send {} bytes in {} packets",
        file_len,
        file_len / 500 + 1
    );
    loop {
        let mut file_buf = [0u8; 500];
        let amt = read_position(&input_file, &mut file_buf, offset)?;

        let buf = get_buf(&msg_num, &file_buf[0..amt]);

        #[cfg(feature = "sim_wan")]
        {
            let num = rand::random::<u8>();

            if num <= 127 {
                sock.send_to(&buf, reciever).await?;
            }
        }
        #[cfg(not(feature = "sim_wan"))]
        sock.send_to(&buf, reciever).await?;

        offset += 500;
        if offset >= file_len {
            break;
        }

        msg_num += 1;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    info!("done sending file");

    // Send message telling client it's done
    // TODO Make sure this gets through
    sock.send_to(&[5], reciever).await?;

    // Get missed messages
    let mut buf = [0u8; 508];

    let amt = recv(&sock, &reciever, &mut buf).await?;
    // This will be an array of u64s with missed things
    // The first will be a message
    let buf = &buf[0..amt];
    if buf[0] != 6 {
        // 6 is missed
        return Ok(());
    }

    let missed = &buf[1..];
    for i in 0..(missed.len() / 8) {
        // Convert bytes to offset
        let missed_msg = u8s_to_u64(&missed[i..i + 8])?;
        let mut buf = [0u8; 500];
        // Read from file
        let amt = get_file_buf_from_msg_num(missed_msg, &input_file, 500, &mut buf)?;
        let buf = &buf[0..amt];

        #[cfg(feature = "sim_wan")]
        {
            let num = rand::random::<u8>();

            if num <= 127 {
                sock.send_to(buf, reciever).await?;
            }
        }
        #[cfg(not(feature = "sim_wan"))]
        sock.send_to(buf, reciever).await?;
    }

    Ok(())
}
