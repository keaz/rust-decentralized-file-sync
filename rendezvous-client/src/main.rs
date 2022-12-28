use async_std::task;
use rendezvous_client::connection_loop;

fn main() {
    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();
    let client_id: &'static str = "client_123";

    let fu = connection_loop("127.0.0.1:8080",client_id);
    task::block_on(fu);

}


