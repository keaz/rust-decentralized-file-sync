use async_std::task;
use futures::channel::mpsc;
use file_sync_core::{client::{ClientEvent}, Result, spawn_and_log_error};

mod server;
mod client;


pub async fn run(address: String, client_id: String)  {
    let (peer_sender, peer_receiver) = mpsc::unbounded();

    let server_async = spawn_and_log_error(server::server_connection_loop(address,client_id,peer_sender));
    let peer_async = spawn_and_log_error(client::peer_loop(peer_receiver));
    futures::future::join(server_async,peer_async).await;
}

