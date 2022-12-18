use std::{net::TcpListener, thread, sync::{Arc, Mutex}};
use uuid::Uuid;

use file_sync_core::client::Client;
pub struct Server {
    address: String,
    pub clients : Arc<Mutex<Vec<Client>>>,
    running: Arc<Mutex<bool>>,
}


impl Server {
    pub fn bind(address: String) -> Server {
        let running = Arc::new(Mutex::new(true));
        let clients: Vec<Client> = vec![];
        let clients = Arc::new(Mutex::new(clients));
        Server { address,clients, running }
    }

    pub fn start(&self){
        let listener = TcpListener::bind(&self.address).unwrap();
        let running = Arc::clone(&self.running);
        let clients = Arc::clone(&self.clients);
        
        let _handler = thread::spawn(move|| loop {
            if *running.lock().unwrap() {
                match listener.accept()   {
                    Ok((socket, addr)) => {
                        println!("new client: {addr:?}");
                        let id = Uuid::new_v4().to_string();
                        let clients = &mut *clients.lock().unwrap();
                        clients.push(Client { id, stream: socket, addr });
                    },
                    Err(e) => {
                        println!("couldn't get client: {e:?}")
                    },
                } 
                continue;
            }
            break;

        });
        
        
    }

    pub fn stop(&mut self) {
        // let clients = self.clients.lock().unwrap();// TODO
        let arc_running = Arc::clone(&self.running);
        let mut running = arc_running.lock().unwrap();
        *running = false;
    }
}

mod tests {
    use super::*;

    #[test]
    fn server_bind(){
        let server = Server::bind(String::from("127.0.0.1:7878"));
        assert_eq!(server.clients.lock().unwrap().len(),0);
        assert_eq!(*server.running.lock().unwrap(),true);
    }

    #[test]
    fn server_stop(){
        let mut server = Server::bind(String::from("127.0.0.1:7878"));
        server.stop();
        assert_eq!(server.clients.lock().unwrap().len(),0);
        assert_eq!(*server.running.lock().unwrap(),false);
    }
}