use std::{error, fs::File, net::SocketAddr, sync::Arc, thread, time::Duration};

use log::{debug, info};
use tokio::net::UdpSocket;

use crate::{
    giver::{get_buf, send_unil_recv},
    punch_hole, read_position, u8s_to_u64,
};

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
    let total = if file_len % 500 == 0 {
        file_len / 500
    } else {
        file_len / 500 + 1
    };

    info!("will send {} bytes in {} packets", file_len, total);
    loop {
        let mut file_buf = [0u8; 500];
        let amt = read_position(&input_file, &mut file_buf, offset)?;

        let buf = get_buf(&msg_num, &file_buf[0..amt]);

        #[cfg(feature = "sim_wan")]
        {
            let num = rand::random::<u8>();

            if num <= 127 {
                sock.send_to(&buf, reciever).await?;
                debug!("msg {} was sent", msg_num);
            } else {
                debug!("msg {} was not sent", msg_num);

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

    let mut buf = [0u8; 508];
    let amt = send_unil_recv(&sock, &[5], &reciever, &mut buf, 1000).await?;

    // This will be an array of u64s with missed things
    // The first will be a message
    let buf = &buf[0..amt];
    if buf[0] != 6 {
        // 6 is missed
        debug!("file has successfully been sent");
        return Ok(());
    }

    debug!("sending dropped");
    let missed = &buf[1..];
    debug!("missed messages: {}", missed.len() / 8);
    for i in 0..(missed.len() / 8) {
        let j = i * 8;
        // Convert bytes to offset
        let missed_msg = u8s_to_u64(&missed[j..j + 8])?;
        debug!("dropped msg is {}", missed_msg);
        let mut file_buf = [0u8; 500];
        // Read from file
        let amt = get_file_buf_from_msg_num(missed_msg, &input_file, 500, &mut file_buf)?;
        let file_buf = &file_buf[0..amt];
        let buf = get_buf(&missed_msg, file_buf);

        #[cfg(feature = "sim_wan")]
        {
            let num = rand::random::<u8>();

            if num <= 127 {
                sock.send_to(&buf, reciever).await?;
                debug!("sent msg {}", missed_msg);
            } else {

                debug!("has not sent msg {}", missed_msg);
            }
        }
        #[cfg(not(feature = "sim_wan"))]
        sock.send_to(&buf, reciever).await?;

    }

    Ok(())
}
