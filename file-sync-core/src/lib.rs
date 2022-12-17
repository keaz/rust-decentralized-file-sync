pub fn copy_files(){}

pub mod client {

    pub struct ClientJoined{
        pub id:  String,
    }
    
    pub struct ClientLeft{
        pub id: String,
    }
    

    pub struct Client {
        pub id: String,
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