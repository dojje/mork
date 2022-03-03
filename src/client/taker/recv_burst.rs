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
