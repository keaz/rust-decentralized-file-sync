use std::{env, sync::Arc, time::Duration};
use std::thread;


use rendezvous_server::Server;
fn main() {
    let args: Vec<String> = env::args().collect();
    let server = Server::bind(String::from("127.0.0.1:7878"));
    server.start();

    let clients = Arc::clone(&server.clients);
    println!("Reading clients");
    thread::spawn(move|| loop {
        println!("Connected clients {:?}",clients.lock().unwrap().len());
        thread::sleep(Duration::from_secs(5));
    }).join().unwrap();
}

