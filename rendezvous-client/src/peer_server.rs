use std::{collections::{hash_map::Entry, HashMap}, sync::Arc};

use async_std::{io::BufReader, net::{TcpListener, TcpStream, ToSocketAddrs}, prelude::*, task};
use futures::{channel::mpsc, FutureExt, select, SinkExt};
use log::{debug, info, warn};

use file_sync_core::{Result, spawn_and_log_error};
use file_sync_core::client::{ClientCommand, ClientEvent, ConnectedPeer};

use crate::peer_client::{PeerMessage, Peer};

type Sender<T> = mpsc::UnboundedSender<T>;
type Receiver<T> = mpsc::UnboundedReceiver<T>;
enum Void {}

pub async fn accept_loop(addr: impl ToSocketAddrs) -> Result<()> {
    info!("Start accepting incoming connections from peers...");
    let listener = TcpListener::bind(addr).await?;

    let (broker_message_sender, broker_message_receiver) = mpsc::unbounded();
    let broker_handle = task::spawn(broker_loop(broker_message_receiver));
    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        info!("Accepting from: {}", stream.peer_addr()?);
        spawn_and_log_error(connection_loop(broker_message_sender.clone(), stream));
    }

    drop(broker_message_sender);
    broker_handle.await;
    Ok(())
}

async fn connection_loop(mut messages: Sender<PeerMessage>, stream: TcpStream) -> Result<()> {
    let stream = Arc::new(stream);
    let addr = stream.peer_addr();
    let reader = BufReader::new(&*stream);

    let mut lines = reader.lines();

    // let (_shutdown_sender, shutdown_receiver) = mpsc::unbounded::<Void>();
    // events.send(Event::NewClient { id, client_id: client_id.clone(), address: addr, port, stream: Arc::clone(&stream), shutdown: shutdown_receiver }).await.unwrap();

    let error_threshold = 10;
    let mut error_count = 0;
    while let Some(line) = lines.next().await {
        let line = match line {
            Err(err) => {
                warn!("Error reading line from {:?}",err);
                error_count += 1;
                if error_count == error_threshold {
                    Err("Client error count reached the threshold")?
                }
                continue;
            }
            Ok(message) => {
                error_count = 0;
                message
            }
        };

        let message = serde_json::from_str(&line);
        match message {
            Err(err) => Err(err)?,
            Ok(message) => {
                match messages.send(message).await {
                    Ok(_) => debug!("Successfully sent the message to broker"),
                    Err(_) => todo!("Failed sending message to broker"),
                }
            }
        }
    }
    Ok(())
}

async fn broker_loop(peer_messages: Receiver<PeerMessage>) {
    let (disconnect_sender, mut disconnect_receiver) = mpsc::unbounded::<(String, Receiver<ClientEvent>)>();
    let mut peers: HashMap<String, Peer> = HashMap::new();
    let mut peer_messages = peer_messages.fuse();
    loop {
        let message = select! {
            event = peer_messages.next().fuse() => match event {
                None => break, // 2
                Some(event) => event,
            },
            disconnect = disconnect_receiver.next().fuse() => {
                let (id, _pending_messages) = disconnect.unwrap();
                match peers.remove(&id) {
                    None => {
                        warn!("Client already left client_id::{}",id)
                    },
                    Some(peer) => {
                        info!("Client is leaving client_id::{}",&peer.id);
                        for peer in peers.values() {
                            let left_event = ClientEvent::ClientLeft { id: id.clone(), client_id: String::from(peer.id.clone())};
                            peer.sender.clone().send(left_event).await.unwrap();
                        }
                        drop(peer);
                    }
                }
                continue;
            },
        };

        match message {
            PeerMessage::JoinPeerCommand { id, peer_id } => {
                match peers.entry(peer_id.clone()) {
                    Entry::Occupied(_) => { warn!("Peer ::{} already exists ::{}",peer_id,id)},
                    Entry::Vacant(entry) => {
                       debug!("Start adding new peer {}",peer_id);
                       let (client_sender, mut client_receiver) = mpsc::unbounded();

                        entry.insert(Peer { peer_id: client_id.clone(), address, port, sender: client_sender });

                        let peers_to_be_sent = peers.values()
                            .filter(|peer| !peer.peer_id.eq(&client_id))
                            .map(|peer| ConnectedPeer { peer_id: peer.peer_id.clone(), address: peer.address.clone(), port: peer.port })
                            .collect();

                        let (client_connected_event, peers_json) = get_connected_event(id, &client_id, peers_to_be_sent);

                        debug!("Sending client connected event {:?}",client_connected_event);
                        send_message(Arc::clone(&stream), peers_json).await;

                        let mut disconnect_sender = disconnect_sender.clone();

                        spawn_and_log_error(async move {
                            let res = connection_writer_loop(&mut client_receiver, stream, shutdown).await;
                            disconnect_sender.send((client_id, client_receiver)).await.unwrap();
                            res
                        });

                    },
                }
            },
            PeerMessage::LeavePeerCommand { id, peer_id } => todo!(),
            PeerMessage::ReadyToUploadFileCommand { id, peer_id, file } => todo!(),
            PeerMessage::PeerJoinedEvent { id, peer_id } => todo!(),
            PeerMessage::FileModifiedEvent { id, peer_id, file } => todo!(),
        }


        match event {
            Event::ClientLeft { id,client_id } => {
                match peers.remove(&client_id) {
                    None => {
                        warn!("Client already left id::{} client_id::{}",id,client_id)
                    },
                    Some(peer) => {
                        info!("Client is leaving client_id::{}",peer.peer_id);
                        for peer in peers.values() {
                            let left_event = ClientEvent::ClientLeft { id: id.clone(), client_id: String::from(client_id.clone())};
                            peer.sender.clone().send(left_event).await.unwrap();
                        }

                        drop(peer);
                    }
                }
            }
            Event::NewClient { id, client_id, address, port, stream, shutdown } => {
                match peers.entry(client_id.clone()) {
                    Entry::Occupied(..) => (),
                    Entry::Vacant(entry) => {
                        let (client_sender, mut client_receiver) = mpsc::unbounded();
                        entry.insert(Peer { peer_id: client_id.clone(), address, port, sender: client_sender });

                        let peers_to_be_sent = peers.values()
                            .filter(|peer| !peer.peer_id.eq(&client_id))
                            .map(|peer| ConnectedPeer { peer_id: peer.peer_id.clone(), address: peer.address.clone(), port: peer.port })
                            .collect();

                        let (client_connected_event, peers_json) = get_connected_event(id, &client_id, peers_to_be_sent);

                        debug!("Sending client connected event {:?}",client_connected_event);
                        send_message(Arc::clone(&stream), peers_json).await;

                        let mut disconnect_sender = disconnect_sender.clone();

                        spawn_and_log_error(async move {
                            let res = connection_writer_loop(&mut client_receiver, stream, shutdown).await;
                            disconnect_sender.send((client_id, client_receiver)).await.unwrap();
                            res
                        });
                    }
                }
            }
        }
    }
    drop(peers); // 5
    drop(disconnect_sender); // 6
    while let Some((_name, _pending_messages)) = disconnect_receiver.next().await {}
}

async fn send_message(stream: Arc<TcpStream>, peers_json: String) {
    let _rs = (&*stream).write_all(peers_json.as_bytes()).await;
    let _rs = (&*stream).write_all(b"\n").await;
}

fn get_connected_event(id: String, client_id: &str, peers_to_be_sent: Vec<ConnectedPeer>) -> (ClientEvent, String) {
    let client_connected_event = ClientEvent::ClientConnected { id, client_id: String::from(client_id), peers: peers_to_be_sent };
    let peers_json = match serde_json::to_string(&client_connected_event) {
        Err(..) => String::new(),
        Ok(json) => json,
    };
    (client_connected_event, peers_json)
}

async fn connection_writer_loop(messages: &mut Receiver<ClientEvent>, stream: Arc<TcpStream>, shutdown: Receiver<Void>) -> Result<()> {
    let mut stream = &*stream;
    let mut messages = messages.fuse();
    let mut shutdown = shutdown.fuse();
    loop {
        select! {
            msg = messages.next().fuse() => match msg {
                Some(msg) => {
                    match serde_json::to_string(&msg) {
                        Err(..) => {
                            warn!("Failed to convert {:?} to JSON",msg);
                        },
                        Ok(json_event) => {
                            stream.write_all(json_event.as_bytes()).await?;
                            stream.write_all(b"\n").await?;
                            debug!("Succesfilly sent event to peer");
                        }
                    }
                },
                None => break,
            },
            void = shutdown.next().fuse() => match void {
                Some(void) => match void {},
                None => break,
            }
        }
    }
    Ok(())
}




fn extract_connect_client(message: &String) -> Result<(String, String, i32)> {
    let (id, client_id, port) = match serde_json::from_str(&message) {
        Err(err) => Err(err)?,
        Ok(message) => {
            match message {
                ClientCommand::ConnectClient { id, client_id, port } => {
                    (id, client_id, port)
                }
                _ => Err("First event wasn't a ClientCommand ")?
            }
        }
    };
    Ok((id, client_id, port))
}
