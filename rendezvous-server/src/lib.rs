pub mod server {
    use std::{collections::{hash_map::Entry, HashMap}, sync::Arc};

    use async_std::{io::BufReader, net::{TcpListener, TcpStream, ToSocketAddrs}, prelude::*, task};
    use futures::{channel::mpsc, FutureExt, select, SinkExt};
    use log::{debug, info, warn};
    #[cfg(test)]
    use mockall::{automock, mock, predicate::*};

    use file_sync_core::{Result, spawn_and_log_error};
    use file_sync_core::client::{ClientCommand, ClientEvent, ConnectedPeer, Peer};

    type Sender<T> = mpsc::UnboundedSender<T>;
    type Receiver<T> = mpsc::UnboundedReceiver<T>;

    enum Void {}

    enum Event {
        NewClient {
            id: String,
            client_id: String,
            address: String,
            port: i32,
            stream: Arc<TcpStream>,
            shutdown: Receiver<Void>,
        },
        ClientLeft{
            id: String,
            client_id: String,
        }

    }

    pub async fn accept_loop(addr: impl ToSocketAddrs) -> Result<()> {
        info!("Start accepting incomming connections");
        let listener = TcpListener::bind(addr).await?;
        let (broker_sender, broker_receiver) = mpsc::unbounded();
        let broker_handle = task::spawn(broker_loop(broker_receiver));
        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await {
            let stream = stream?;
            info!("Accepting from: {}", stream.peer_addr()?);
            spawn_and_log_error(connection_loop(broker_sender.clone(), stream));
        }
        drop(broker_sender);
        broker_handle.await;
        Ok(())
    }

    async fn broker_loop(events: Receiver<Event>) {
        let (disconnect_sender, mut disconnect_receiver) = mpsc::unbounded::<(String, Receiver<String>)>();
        let mut peers: HashMap<String, Peer> = HashMap::new();
        let mut events = events.fuse();
        loop {
            let event = select! {
                event = events.next().fuse() => match event {
                    None => break, // 2
                    Some(event) => event,
                },
                disconnect = disconnect_receiver.next().fuse() => {
                    let (id, _pending_messages) = disconnect.unwrap(); // 3
                    match peers.remove(&id) {
                        None => {
                            warn!("Client already left client_id::{}",id)
                        },
                        Some(peer) => {
                            info!("Client is leaving client_id::{}",peer.peer_id);
                            drop(peer);
                        }
                    }
                    continue;
                },
            };
            match event {
                Event::ClientLeft { id,client_id } => {
                    match peers.remove(&client_id) {
                        None => {
                            warn!("Client already left id::{} client_id::{}",id,client_id)
                        },
                        Some(peer) => {
                            info!("Client is leaving client_id::{}",peer.peer_id);
                            let left_event = get_left_event(id,&client_id);
                            //TOD
                            // send_message(Arc::clone(&peer), left_event).await;
                            drop(peer);
                        }
                    }
                }
                Event::NewClient { id, client_id, address, port, stream, shutdown } => {
                    match peers.entry(id.clone()) {
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

    fn get_left_event(id: String, client_id: &str) -> String {
        let client_connected_event = ClientEvent::ClientLeft { id, client_id: String::from(client_id)};
        match serde_json::to_string(&client_connected_event) {
            Err(..) => String::new(),
            Ok(json) => json,
        }
    }

    fn get_connected_event(id: String, client_id: &str, peers_to_be_sent: Vec<ConnectedPeer>) -> (ClientEvent, String) {
        let client_connected_event = ClientEvent::ClientConnected { id, client_id: String::from(client_id), peers: peers_to_be_sent };
        let peers_json = match serde_json::to_string(&client_connected_event) {
            Err(..) => String::new(),
            Ok(json) => json,
        };
        (client_connected_event, peers_json)
    }

    async fn connection_writer_loop(messages: &mut Receiver<String>, stream: Arc<TcpStream>, shutdown: Receiver<Void>) -> Result<()> {
        let mut stream = &*stream;
        let mut messages = messages.fuse();
        let mut shutdown = shutdown.fuse();
        loop {
            select! {
                msg = messages.next().fuse() => match msg {
                    Some(msg) => stream.write_all(msg.as_bytes()).await?,
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

    async fn connection_loop(mut broker: Sender<Event>, stream: TcpStream) -> Result<()> {
        let stream = Arc::new(stream);
        let addr = stream.peer_addr();
        let reader = BufReader::new(&*stream);

        let mut lines = reader.lines();

        let message = match lines.next().await {
            None => Err("peer disconnected immediately")?,
            Some(line) => line?,
        };

        let addr = match addr {
            Err(..) => Err("Cannot get peer address")?,
            Ok(address) => address.ip().to_string(),
        };

        let (id, client_id, port) = extract_connect_client(&message)?;

        debug!("Receive new ConnectClient id :{:?} peer:{:}",id,client_id);
        let (_shutdown_sender, shutdown_receiver) = mpsc::unbounded::<Void>();
        broker.send(Event::NewClient { id, client_id: client_id.clone(), address: addr, port, stream: Arc::clone(&stream), shutdown: shutdown_receiver }).await.unwrap();


        let error_threshold = 10;
        let mut error_count = 0;
        while let Some(line) = lines.next().await {
            let line = match line {
                Err(err) => {
                    warn!("Error {:?} reading line from {:?}",err,client_id);
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

            match serde_json::from_str(&line) {
                Err(err) => Err(err)?,
                Ok(message) => {
                    match message {
                        ClientCommand::LeaveClient { id, client_id } => {
                            info!("Received Client leave command id::{} client::{}",id,client_id);
                            broker.send(Event::ClientLeft {id,client_id}).await.unwrap();
                        }
                        _ => Err("First event wasn't a ClientCommand ")?
                    }
                }
            }

            let (dest, msg) = match line.find(':') {
                None => continue,
                Some(idx) => (&line[..idx], line[idx + 1..].trim()),
            };
            let dest: Vec<String> = dest.split(',').map(|name| name.trim().to_string()).collect();
            let msg: String = msg.to_string();

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
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_loop() {
        // let server = Server::bind(String::from("127.0.0.1:7878"));

        // assert_eq!(server.clients.lock().unwrap().len(),0);
        // assert_eq!(*server.running.lock().unwrap(),true);
    }

    #[test]
    fn server_stop() {
        // let mut server = Server::bind(String::from("127.0.0.1:7878"));
        // server.stop();
        // assert_eq!(server.clients.lock().unwrap().len(),0);
        // assert_eq!(*server.running.lock().unwrap(),false);
    }
}