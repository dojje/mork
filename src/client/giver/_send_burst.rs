use std::{error, fs::File, net::SocketAddr, sync::Arc, thread, time::Duration};

use log::info;
use tokio::net::UdpSocket;

use crate::{punch_hole, read_position};

pub async fn send_file_burst(
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
    info!(
        "will send {} bytes in {} packets",
        file_len,
        file_len / 500 + 1
    );
    loop {
        let mut file_buf = [0u8; 500];
        let amt = read_position(&input_file, &mut file_buf, offset)?;

        sock.send_to(&file_buf[0..amt], reciever).await?;

        offset += 500;
        if offset >= file_len {
            break;
        }
    }
    info!("done sending file");

    Ok(())
}
