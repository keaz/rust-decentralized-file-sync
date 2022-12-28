use std::{sync::{Arc}, collections::{HashMap, hash_map::Entry}, borrow::BorrowMut, };
use async_std::{prelude::*,net::{TcpStream, ToSocketAddrs, TcpListener}, task, io::BufReader,};
use futures::{channel::mpsc, select, SinkExt, FutureExt};
use serde::{Deserialize, Serialize};
use file_sync_core::client::ClientJoinedEvent;


use log::{info,debug,warn};

type JSONResult<T> = serde_json::Result<T>;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
type Sender<T> = mpsc::UnboundedSender<T>; // 2
type Receiver<T> = mpsc::UnboundedReceiver<T>;


enum Void {} 

enum Event { 
    NewPeer {
        id: String,
        address: String,
        port: i32,
        stream: Arc<TcpStream>,
        shutdown: Receiver<Void>,
    },

}

#[derive(Serialize, Deserialize)]
struct Peer {
    id: String,
    address: String,
    port: i32,
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
                assert!(peers.remove(&id).is_some());
                continue;
            },
        };
        match event {
            // Event::Message { from, to, msg } => {
            //     for addr in to {
            //         if let Some(peer) = peers.get_mut(&addr) {
            //             let msg = format!("from {}: {}\n", from, msg);
            //             peer.send(msg).await
            //                 .unwrap() // 6
            //         }
            //     }
            // }
            Event::NewPeer { id,address,port, stream, shutdown } => {
                match peers.entry(id.clone()) {
                    Entry::Occupied(..) => (),
                    Entry::Vacant(entry) => {
                        let (_client_sender, mut client_receiver) = mpsc::unbounded();
                        entry.insert(Peer { id: id.clone(), address, port});
                        let peers_to_be_sent:Vec<&Peer> =  peers.values().map(|peer| peer).collect();
                        let peers_json = match serde_json::to_string(&peers_to_be_sent) {
                            Err(..) => String::new(),
                            Ok(json) => json,
                        };
                        let _rs = (&*stream).write_all(peers_json.as_bytes()).await;
                        let _rs = (&*stream).write_all(b"\n").await;
                        let mut disconnect_sender = disconnect_sender.clone();

                        spawn_and_log_error(async move {

                            let res = connection_writer_loop(&mut client_receiver, stream, shutdown).await;
                            disconnect_sender.send((id, client_receiver)).await.unwrap();
                            res
                        });
                    }
                }
            }
        }
    }
    drop(peers); // 5
    drop(disconnect_sender); // 6
    while let Some((_name, _pending_messages)) = disconnect_receiver.next().await {
    }
}

async fn connection_writer_loop(messages: &mut Receiver<String>,stream: Arc<TcpStream>,shutdown: Receiver<Void>,) -> Result<()> {
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

    let new_peer: ClientJoinedEvent = match serde_json::from_str(&message) {
        Err(err) => Err(err)?,
        Ok(message) => message,
    };
    
    let peer_id = new_peer.id.clone();
    debug!("New Peer join message {:?}",new_peer);
    let (_shutdown_sender, shutdown_receiver) = mpsc::unbounded::<Void>();
    broker.send(Event::NewPeer { id: new_peer.id, address: addr,port: new_peer.port , stream: Arc::clone(&stream), shutdown: shutdown_receiver }).await .unwrap();

    let error_threshold = 10;
    let mut error_count = 0;
    while let Some(line) = lines.next().await {
        let message = match line {
            Err(err) => {
                warn!("Error {:?} reading line from {:?}",err,&peer_id);
                error_count += 1;
                if error_count == error_threshold {
                    Err("Client error count reached the threshold")?
                }
                continue
            },
            Ok(message) => {
                error_count = 0;
                message
            },
        };
    }
    // while let Some(line) = lines.next().await {
    //     let line = line?;
    //     let (dest, msg) = match line.find(':') {
    //         None => continue,
    //         Some(idx) => (&line[..idx], line[idx + 1 ..].trim()),
    //     };
    //     let dest: Vec<String> = dest.split(',').map(|name| name.trim().to_string()).collect();
    //     let msg: String = msg.to_string();

    //     broker.send(Event::Message { // 4
    //         from: name.clone(),
    //         to: dest,
    //         msg,
    //     }).await.unwrap();
    // }
    Ok(())
}


fn spawn_and_log_error<F>(fut: F) -> task::JoinHandle<()> where F: Future<Output = Result<()>> + Send + 'static,{
    task::spawn(async move {
        if let Err(e) = fut.await {
            warn!("{}", e)
        }
    })
}


mod tests {
    use super::*;

    #[test]
    fn server_bind(){
        // let server = Server::bind(String::from("127.0.0.1:7878"));
        // assert_eq!(server.clients.lock().unwrap().len(),0);
        // assert_eq!(*server.running.lock().unwrap(),true);
    }

    #[test]
    fn server_stop(){
        // let mut server = Server::bind(String::from("127.0.0.1:7878"));
        // server.stop();
        // assert_eq!(server.clients.lock().unwrap().len(),0);
        // assert_eq!(*server.running.lock().unwrap(),false);
    }
}