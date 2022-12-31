use std::future::Future;
use async_std::task;
use log::warn;

pub mod client {
    use futures::channel::mpsc;
    use serde::{Deserialize, Serialize};
    type Sender<T> = mpsc::UnboundedSender<T>;

    pub struct Peer {
        pub peer_id: String,
        pub address: String,
        pub port: i32,
        pub sender: Sender<String>,
    }

    #[derive(Serialize, Deserialize,Debug)]
    pub struct ConnectedPeer{
        pub peer_id: String,
        pub address: String,
        pub port: i32,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub enum ClientEvent {
        ClientConnected {
            id: String,
            client_id: String,
            peers: Vec<ConnectedPeer>,
        },
        ClientLeft{
            id: String,
            client_id: String,
        }
    }

    #[derive(Serialize, Deserialize)]
    pub enum ClientCommand {
        ConnectClient {
            id: String,
            client_id: String,
            port: i32,
        },
        LeaveClient {
            id: String,
            client_id: String,
        }
    }

}

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub fn spawn_and_log_error<F>(fut: F) -> task::JoinHandle<()> where F: Future<Output = Result<()>> + Send + 'static,{
    task::spawn(async move {
        if let Err(e) = fut.await {
            warn!("{}", e)
        }
    })
}