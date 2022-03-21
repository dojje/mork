use std::{
    error::{self, Error},
    fs::File,
    net::SocketAddr,
    sync::Arc, path::Path,
};

use dovepipe::{send_file, Source};
use log::info;
use shared::messages::{
    have_file::HaveFile, recieving_ip::RecievingIp, you_have_file::YouHaveFile, Message,
};
use tokio::net::UdpSocket;

use crate::{ensure_global_ip, send_unil_recv, SendMethod};

// mod send_burst;
// mod send_index;

fn get_file_len(filepath: &Path) -> Result<u64, Box<dyn error::Error>> {
    let file = File::open(filepath)?;

    Ok(file.metadata().unwrap().len())
}

pub async fn sender<'a>(
    filepath: &Path,
    sock: Arc<UdpSocket>,
    server_addr: SocketAddr,
    send_method: SendMethod,
) -> Result<(), Box<dyn Error>> {
    let file_len = match get_file_len(&filepath) {
        Ok(f) => f,
        Err(_) => panic!("file {} doesn't exist", filepath.to_str().unwrap()),
    };

    let only_file_name = Path::new(&filepath).file_name().unwrap();
    let have_file = HaveFile::new(only_file_name.to_str().unwrap().to_owned(), file_len);

    let mut buf = [0u8; 508];
    let amt = send_unil_recv(&sock, &have_file.to_raw(), &server_addr, &mut buf, 500).await?;
    let buf = &buf[0..amt];
    let you_have_file = YouHaveFile::from_raw(&buf)?;

    let code = you_have_file.code;
    println!("Code for recv: {}", &code);

    // Wait for recieving ip from server
    loop {
        let mut buf = [0; 508];
        // Sends holepunch msgs until it gets any recievers ip
        let amt = send_unil_recv(&sock, &[255u8], &server_addr, &mut buf, 1000).await?;
        let buf = &buf[0..amt];

        let recieving_ip = RecievingIp::from_raw(buf)?;

        let correct_ip = ensure_global_ip(recieving_ip.ip, &server_addr);
        let filename = filepath.to_str().unwrap().to_owned();
        let sock_send = sock.clone();
        let send_method = send_method.clone();
        tokio::spawn(async move {
            match &send_method {
                SendMethod::Burst => {
                    // send_file_burst(sock_send, file_name, correct_ip)
                    //    .await
                    //    .expect("could not send file");
                }
                SendMethod::Confirm => todo!(),
                SendMethod::Index => {
                    info!("got reciever");
                    send_file(Source::SocketArc(sock_send), filename.as_str(), correct_ip)
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
