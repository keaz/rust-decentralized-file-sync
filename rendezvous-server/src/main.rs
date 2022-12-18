use std::{env, sync::Arc, time::Duration};
use std::thread;
use file_sync_core::client;
use rendezvous_server::Server;


fn main() {
    let args: Vec<String> = env::args().collect();
    let server = Server::bind(String::from("127.0.0.1:7878"));
    server.start();

    let clients = Arc::clone(&server.clients);
    println!("Reading clients");
    thread::spawn(move|| loop {

        match clients.lock() {
            Ok(mut clients) => {
                println!("Connected clients {:?}",clients.len());
                clients.iter_mut().for_each(|client|{
                    println!("Sending message to client {:?}",client.address);
                    client.send_message("message");
                })
            },
            Err(_) => {
                eprint!("Failed to get clients");
            },
        }
        // let clients = clients.lock().unwrap();
        // println!("Connected clients {:?}",clients.len());
        thread::sleep(Duration::from_secs(5));
        
    }).join().unwrap();
}

