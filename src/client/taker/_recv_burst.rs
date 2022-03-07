use std::{error, fs::File, io::Write, net::SocketAddr, sync::Arc, time::Duration};

use log::info;
use tokio::{net::UdpSocket, time};

use crate::recv;

pub async fn recv_file_burst(
    file: &mut File,
    sock: Arc<UdpSocket>,
    ip: SocketAddr,
) -> Result<(), Box<dyn error::Error>> {
    loop {
        let wait_time = time::sleep(Duration::from_millis(2000));
        let mut buf = [0u8; 508];
        tokio::select! {
            _ = wait_time => {
                info!("No message has been recieved for 2000ms, exiting!");
                break;
            }
            amt = recv(&sock, &ip, &mut buf) => {
                let amt = amt?;
                let buf = &buf[0..amt];

                if buf.len() == 1 && buf[0] == 255 {
                    // Skip if the first iteration is a hole punch msg
                    continue;
                }

                file.write(&buf).unwrap();
            }
        }
    }

    Ok(())
}
