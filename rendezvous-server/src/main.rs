use std::{env};
use async_std::task;
use rendezvous_server::server::accept_loop;


fn main() {
    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();
    let _args: Vec<String> = env::args().collect();
    
    let fut = accept_loop("127.0.0.1:8080");
    let _result = task::block_on(fut);
}

