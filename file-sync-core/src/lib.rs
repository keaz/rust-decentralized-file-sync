
pub mod client {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize,Debug)]
    pub struct ClientJoinedEvent {
        pub id: String,
        pub port: i32,
    }

}