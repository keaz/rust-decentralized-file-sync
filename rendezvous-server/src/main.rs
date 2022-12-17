use std::env;

use file_sync_core::client::Client;
use rendezvous_server::Server;
fn main() {
    let args: Vec<String> = env::args().collect();
    let server = Server::bind("127.0.0.1:7878");
    
}

