extern crate async_std;
extern crate futures;

use async_std::{
    io::{BufReader, stdin},
    net::{TcpStream, ToSocketAddrs},
    prelude::*,
};
use futures::{FutureExt, select, SinkExt};
use futures::channel::mpsc;
use log::{debug, info, warn};
use uuid::Uuid;

use file_sync_core::{client::ClientEvent, Result};
use file_sync_core::client::ClientCommand;

type Sender<T> = mpsc::UnboundedSender<T>;

pub async fn server_connection_loop(addr: impl ToSocketAddrs, client_id: String, mut peer_sender: Sender<ClientEvent>) -> Result<()> {
    let stream = TcpStream::connect(addr).await?;
    let join_client_command = ClientCommand::ConnectClient { id: Uuid::new_v4().to_string(), client_id: client_id.clone(), port: 7890 };
    let event_json = serde_json::to_string(&join_client_command)?;

    let (reader, mut writer) = (&stream, &stream); 
    send_event(event_json, &mut writer).await?;

    let mut lines_from_server = BufReader::new(reader).lines().fuse();
    let mut lines_from_stdin = BufReader::new(stdin()).lines().fuse();
    loop {
        select! {
            line = lines_from_server.next().fuse() => match line {
                Some(line) => {
                    let line = line?;
                    debug!("{}", line);
                    match serde_json::from_str(&line) {
                        Err(..) => {
                            warn!("Error converting JSON to ClientEvent");
                        }
                        Ok(message) => {
                            match peer_sender.send(message).await {
                                Err(err) => {
                                    warn!("Failed to send message {:?}",err);
                                },
                                Ok(..) => {
                                    debug!("Successfully sent event to client");
                                }
                            };
                        },
                    };
                },
                None => break,
            },
            line = lines_from_stdin.next().fuse() => match line {
                Some(line) => {
                    let line = line?;
                    if line.eq("EXIT") {
                        send_exit_event(client_id.clone(), stream.clone()).await?;
                        break;
                    }
                }
                None => break,
            }
        }
    }
    Ok(())
}


async fn send_exit_event(client_id: String,stream:TcpStream ) -> Result<()> {
    let id = Uuid::new_v4();
    let leave_command = ClientCommand::LeaveClient{id: id.to_string(), client_id: client_id.clone()};
    let event_json = serde_json::to_string(&leave_command)?;

    send_event(event_json, &mut &stream).await?;
    info!("Sent exit event id::{}",id);
    Ok(())
}

async fn send_event(event_json: String, writer: &mut &TcpStream) -> Result<()> {
    writer.write_all(event_json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    Ok(())
}