use futures::channel::mpsc;
use file_sync_core::spawn_and_log_error;

mod server;
mod client;


pub async fn run(address: String, client_id: String)  {

    let (peer_sender, peer_receiver) = mpsc::unbounded();
    let client_async = spawn_and_log_error(client::server_connection_loop(address,client_id,peer_sender));
    let server_async = spawn_and_log_error(server::peer_loop(peer_receiver));
    futures::future::join(server_async,client_async).await;
}

