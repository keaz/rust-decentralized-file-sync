use std::collections::HashMap;

use async_std::task;
use futures::channel::mpsc;
use futures::StreamExt;
use log::{info};
use file_sync_core::{Result};
use file_sync_core::client::{ClientEvent, ConnectedPeer};

type Receiver<T> = mpsc::UnboundedReceiver<T>;

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