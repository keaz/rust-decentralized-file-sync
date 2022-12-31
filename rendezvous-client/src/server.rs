extern crate async_std;
extern crate futures;

use async_std::{
    io::{BufReader, stdin},
    net::{TcpStream, ToSocketAddrs},
    prelude::*,
};
use futures::{FutureExt, select, SinkExt};
use futures::channel::mpsc;
use log::{debug, error, info, warn};
use uuid::Uuid;

use file_sync_core::{client::ClientEvent, Result};
use file_sync_core::client::{ClientCommand, ConnectedPeer, Peer};

type Sender<T> = mpsc::UnboundedSender<T>;

pub async fn server_connection_loop(addr: impl ToSocketAddrs, client_id: String, mut peer_sender: Sender<ConnectedPeer>) -> Result<()> {
    let stream = TcpStream::connect(addr).await?;
    let join_client_command = ClientCommand::ConnectClient { id: Uuid::new_v4().to_string(), client_id: client_id.clone(), port: 7890 };
    let event_json = serde_json::to_string(&join_client_command)?;

    let (reader, mut writer) = (&stream, &stream); // 1
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
                        Err(err) => Err(err)?,
                        Ok(message) => {
                            match message {
                                ClientEvent::ClientConnected{ id: _,client_id: _,peers} => {
                                    for peer in peers {
                                        if peer.peer_id.eq(&client_id) {
                                            continue;
                                        }
                                        peer_sender.send(peer).await.unwrap();
                                    }
                                },
                                ClientEvent::ClientLeft{id,client_id} => {
                                    info!("Client left id::{} client_id::{}",id,client_id);
                                }
                                _ => Err("Wrong ClientEvent event")?,
                            }
                        },
                    };


                },
                None => break,
            },
            line = lines_from_stdin.next().fuse() => match line {
                Some(line) => {
                    let line = line?;
                    if line.eq("EXIT") {
                        let leave_command = ClientCommand::LeaveClient{id: Uuid::new_v4().to_string(), client_id: client_id.clone()};
                        let event_json = serde_json::to_string(&leave_command)?;

                        send_event(event_json, &mut &stream).await?;

                        writer.write_all(line.as_bytes()).await?;
                        writer.write_all(b"\n").await?;
                    }

                }
                None => break,
            }
        }
    }
    Ok(())
}

async fn send_event(event_json: String, writer: &mut &TcpStream) -> Result<()> {
    writer.write_all(event_json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    Ok(())
}