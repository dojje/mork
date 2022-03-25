use std::{
    error::{Error},
    fs::{File},
    net::SocketAddr,
    sync::Arc, path::Path, env,
};

use dovepipe::{send_file, Source};
use log::{info, debug};
use shared::messages::{
    have_file::HaveFile, recieving_ip::RecievingIp, you_have_file::YouHaveFile, Message,
};
use tokio::net::UdpSocket;

use flate2::write::GzEncoder;
use flate2::Compression;

use crate::{ensure_global_ip, send_unil_recv, SendMethod, TRANSFER_FILENAME};

// mod send_burst;
// mod send_index;

pub async fn sender<'a>(
    filepath: &Path,
    sock: Arc<UdpSocket>,
    server_addr: SocketAddr,
    send_method: SendMethod,
) -> Result<(), Box<dyn Error>> {
    debug!("filepath: {}", filepath.display());
    // Compress it into a gzip
    // The gzip should contain one item

    // Create encoder
    // Create the compressed file
    let tar_filepath = env::temp_dir().join("mork_tmp.tar.gz");
    let tar_gz = File::create(&tar_filepath)?;
    
    // Create the encoder
    let mut enc = GzEncoder::new(tar_gz, Compression::default());

    {
        let mut tar = tar::Builder::new(&mut enc);
        debug!("made gzip encoder");

        // Add data to tarball
        debug!("adding data from data folder to tarball");
        tar.append_path(filepath)?;
    }
    
    // Finish the gzip file
    enc.finish()?;

    // Send the sending msg to the server
    // Get only the file name of the thing to send
    let only_file_name = TRANSFER_FILENAME;
    // Send `HaveFile` msg
    let have_file = HaveFile::new(only_file_name.to_owned(), 200);
    let mut buf = [0u8; 508];
    // Recieve a `YouHaveFile` message
    let amt = send_unil_recv(&sock, &have_file.to_raw(), &server_addr, &mut buf, 500).await?;
    let buf = &buf[0..amt];
    let you_have_file = YouHaveFile::from_raw(&buf)?;
    // Extract code from the message
    let code = you_have_file.code;
    println!("Code for recv: {}", &code);

    // Wait for recieving ip from server
    loop {
        let mut buf = [0; 508];
        // Sends holepunch msgs until it gets any recievers ip
        let amt = send_unil_recv(&sock, &[255u8], &server_addr, &mut buf, 1000).await?;
        let buf = &buf[0..amt];

        let recieving_ip = RecievingIp::from_raw(buf)?;

        // Copy stuff
        let tar_filepath = tar_filepath.clone();
        let correct_ip = ensure_global_ip(recieving_ip.ip, &server_addr);
        let sock_send = sock.clone();
        let send_method = send_method.clone();
        tokio::spawn(async move {
            let tar_filepath = tar_filepath.clone();
            match &send_method {
                SendMethod::Burst => {
                    // send_file_burst(sock_send, file_name, correct_ip)
                    //    .await
                    //    .expect("could not send file");
                }
                SendMethod::Confirm => todo!(),
                SendMethod::Index => {
                    info!("got reciever");
                    send_file(Source::SocketArc(sock_send), tar_filepath.as_path(), correct_ip)
                        .await
                        .expect("could not send file");
                }
            }
        });
    }
}

// ** THE PROGRESS TRACKER **
//
// It works by having an array of bits of all messages
// It's the lengh of all messages that should be recieved
// When a message is recieved, the bit for that message will flip
