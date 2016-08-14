extern crate futures;
extern crate futures_cpupool;
extern crate futures_curl;
extern crate futures_io;
extern crate futures_minihttp;
extern crate futures_mio;
extern crate postgres;
extern crate r2d2;
extern crate r2d2_postgres;

use std::env;
use std::io;
use std::net::SocketAddr;

use futures_minihttp::{Server, Request, Response};

fn main() {
    let port = env::var("PORT").unwrap_or("8080".to_string());
    let default_addr = format!("127.0.0.1:{}", port);
    let addr = env::args().nth(1).unwrap_or(default_addr);
    let addr = addr.parse::<SocketAddr>().unwrap();

    println!("Listening on {}", addr);
    Server::new(&addr).workers(1).serve(|_: Request| {
        let mut resp = Response::new();
        resp.body("hello");
        futures::finished::<_, io::Error>(resp)
    }).unwrap();
}
