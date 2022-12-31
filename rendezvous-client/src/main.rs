use std::time::{SystemTime, UNIX_EPOCH};
use async_std::task;
use rendezvous_client::run;


fn main() {
    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");

    let client_id = format!("client_123_{}",since_the_epoch.subsec_millis());
    let f = run(String::from("127.0.0.1:8080"),client_id);
    task::block_on(f);
}
