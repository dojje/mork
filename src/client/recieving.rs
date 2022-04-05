use dovepipe::File as AsyncFile;
use std::fs::File as StdFile;
use tar::Archive;

use flate2::read::GzDecoder;
use log::info;
use shared::{
    messages::{i_have_code::IHaveCode, ip_for_code::IpForCode, Message},
    send_msg,
};

#[cfg(target = "windows")]
use std::os::windows::prelude::FileExt;

use std::{error::Error, net::SocketAddr, sync::Arc};
use tokio::{fs::remove_file, net::UdpSocket};

use dovepipe::{reciever::ProgressTracking, recv_file, Source};

use crate::{ensure_global_ip, punch_hole, recv, SendMethod, TRANSFER_FILENAME};

// mod recv_burst;
// mod recv_index;

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

    let mut buf = [0u8; 508];
    let amt = recv(&sock, &server_addr, &mut buf).await?;
    let buf = &buf[0..amt];

    let ip_for_code = IpForCode::from_raw(buf)?;
    let ip = ensure_global_ip(ip_for_code.ip, &server_addr);
    info!("file name: {}", &ip_for_code.file_name);
    info!("other ip: {}", &ip);

    // TODO Check if file fits on disk

    // Punch hole
    punch_hole(&sock, ip).await?;

    // Use custom output if specified
    // Else use the filename provided by the server
    // TODO: Remove filename from server
    let _filename = match output {
        Some(filename) => filename,
        None => ip_for_code.file_name,
    };

    let file = AsyncFile::create(TRANSFER_FILENAME).await?;

    info!("Recieving file...");

    match send_method {
        SendMethod::Burst => {
            // recv_file_burst(&mut file, sock, ip).await?;
        }
        SendMethod::Confirm => todo!(),
        SendMethod::Index => {
            recv_file(
                Source::SocketArc(sock),
                file,
                ip,
                ProgressTracking::File("mork_paging_file".to_string()),
            )
            .await
            .expect("not able to send file");
            // Apparently the `?` operator doesn't work on the line above
            // This should be fixed i think

            let tar_gz = StdFile::open(TRANSFER_FILENAME)?;
            let tar = GzDecoder::new(tar_gz);
            let mut archive = Archive::new(tar);
            archive.unpack(".")?;

            remove_file(TRANSFER_FILENAME).await?;
        }
    }

    Ok(())
}
