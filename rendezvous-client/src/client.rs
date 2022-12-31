use async_std::task;
use futures::channel::mpsc;
use futures::StreamExt;
use log::info;
use file_sync_core::{Result};
use file_sync_core::client::{ClientEvent, ConnectedPeer, Peer};

type Receiver<T> = mpsc::UnboundedReceiver<T>;

pub async fn peer_loop(peer_receiver: Receiver<ConnectedPeer>) -> Result<()>{

    let peers: Vec<ConnectedPeer> = vec![];

    info!("Peer loop started");
    let peer_receiver_handler = task::spawn(peer_receiver_loop(peer_receiver,peers));

    peer_receiver_handler.await;
    Ok(())
}

async fn peer_receiver_loop(mut peer_receiver: Receiver<ConnectedPeer>,mut peers: Vec<ConnectedPeer>) {
    loop {
        let new_peer  = match peer_receiver.next().await {
            None => break,
            Some(new_peer) => new_peer,
        };
        info!("Adding new peer {}",new_peer.peer_id);
        peers.push(new_peer);
    }
}