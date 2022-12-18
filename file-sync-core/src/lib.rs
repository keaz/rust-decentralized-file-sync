pub fn copy_files(){}

pub mod client {
    use std::{net::{TcpStream, SocketAddr}, io::Write};



    pub struct Client {
        pub id: String,
        pub stream: TcpStream,
        pub addr: SocketAddr,
    }

    pub struct ClientJoined{
        pub id:  String,
    }
    
    pub struct ClientLeft{
        pub id: String,
    }
    

    impl Client {
        pub fn close(&self){
            
        }

        pub fn send_message(&self, message: &str){
            // self.stream.write_all(message.as_bytes()).unwrap();
        }
    }
    

    impl ClientJoined {
        pub fn build(id: String ) -> ClientJoined {
            ClientJoined { id }
        }
    }

    impl ClientLeft {
        pub fn build(id: String ) -> ClientLeft {
            ClientLeft { id }
        }
    }

}