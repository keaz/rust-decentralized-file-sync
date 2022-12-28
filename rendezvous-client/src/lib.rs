extern crate async_std;
extern crate futures;

use async_std::{
    io::{BufReader, stdin},
    net::{TcpStream, ToSocketAddrs},
    prelude::*,
};
use futures::{FutureExt, select};
use log::{debug,info,warn,error};


type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

use file_sync_core::client::ClientJoinedEvent;


pub async fn connection_loop(addr: impl ToSocketAddrs,client_id: &str) -> Result<()> {

    let stream = TcpStream::connect(addr).await?;
    let client_joined_event = ClientJoinedEvent{id: String::from(client_id),port:7890};
    let event_json =  serde_json::to_string(&client_joined_event)?;

    let (reader, mut writer) = (&stream, &stream); // 1
    send_event(event_json, &mut writer).await?;

    let mut lines_from_server = BufReader::new(reader).lines().fuse();
    let mut lines_from_stdin = BufReader::new(stdin()).lines().fuse();
    loop {
        select! { // 3
            line = lines_from_server.next().fuse() => match line {
                Some(line) => {
                    let line = line?;
                    let clients = serde_json::to_vec(&line);
                    debug!("{}", line);
                },
                None => break,
            }
            // line = lines_from_stdin.next().fuse() => match line {
            //     Some(line) => {
            //         let line = line?;
            //         writer.write_all(line.as_bytes()).await?;
            //         writer.write_all(b"\n").await?;
            //     }
            //     None => break,
            // }
        }
    }
    Ok(())
}

async fn send_event(event_json: String, writer: &mut &TcpStream) -> Result<()> {
    writer.write_all(event_json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    Ok(())
}
