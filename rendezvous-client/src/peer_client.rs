use std::collections::HashMap;

use async_std::channel::Sender;
use async_std::net::TcpStream;
use async_std::task;
use futures::channel::mpsc;
use futures::StreamExt;
use log::{info};
use file_sync_core::{Result};
use file_sync_core::client::{ClientEvent, ConnectedPeer};
use serde::{Serialize, Deserialize};

type Receiver<T> = mpsc::UnboundedReceiver<T>;


#[derive(Serialize, Deserialize,Debug)]
pub enum PeerMessage {
    JoinPeerCommand{
        id: String,
        peer_id: String,
    },
    LeavePeerCommand {
        id: String,
        peer_id: String
    },
    ReadyToUploadFileCommand{
        id: String,
        peer_id: String,
        file: String,
    },
    PeerJoinedEvent{
        id: String,
        peer_id: String,
    },
    FileModifiedEvent {
        id: String,
        peer_id: String,
        file: String,
    }
}


pub struct Peer{
    pub id: String,
    pub stream: TcpStream,
    pub sender: Sender<InternalMessages>,
}

pub async fn peer_loop(peer_receiver: Receiver<ClientEvent>) -> Result<()>{

    let peers_holder: HashMap<String, ConnectedPeer> = HashMap::new();

    info!("Peer loop started");
    let peer_receiver_handler = task::spawn(peer_receiver_loop(peer_receiver,peers_holder));

    peer_receiver_handler.await;
    Ok(())
}

async fn peer_receiver_loop(mut peer_receiver: Receiver<ClientEvent>,mut peers_holder: HashMap<String, ConnectedPeer>) {
    loop {
        let client_event  = match peer_receiver.next().await {
            None => break,
            Some(new_peer) => new_peer,
        };
        match client_event {
            ClientEvent::ClientConnected{ id: _,client_id:_,peers} => {
                for peer in peers {
                    //TODO Should connect with peers
                    peers_holder.insert(peer.peer_id.clone(), peer);
                }
            },
            ClientEvent::ClientLeft{id,client_id} => {
                peers_holder.remove(&client_id);
                info!("Client removed id::{} client_id::{}",id,client_id);
            },
        }
    }
}
