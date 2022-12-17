use file_sync_core::client::Client;
pub struct Server {
    pub clients : Vec<Client>,
}


impl Server {
    pub fn bind(address: &str) -> Server {

        let clients = vec![];
        Server { clients }
    }

    pub fn stop(&mut self) {
        self.clients.clear();// TODO
    }
}

mod tests {
    use super::*;

    #[test]
    fn server_bind(){
        let server = Server::bind("127.0.0.1");
        assert_eq!(server.clients.len(),0)
    }

    #[test]
    fn server_stop(){
        let mut server = Server::bind("127.0.0.1");
        server.stop();
        assert_eq!(server.clients.len(),0)
    }
}