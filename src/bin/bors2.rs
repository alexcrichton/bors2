extern crate env_logger;
extern crate hyper;

#[macro_use]
extern crate log;

use std::env;
use std::net::SocketAddr;
use std::fs::File;

use hyper::server::{Server, Request, Response};

fn main() {
    env_logger::init().unwrap();

    let heroku = env::var("HEROKU").is_ok();
    if heroku {
        File::create("/tmp/app-initialized").unwrap();
    }

    let addr = env::args().nth(1).unwrap_or("127.0.0.1:3000".to_string());
    let addr = addr.parse::<SocketAddr>().unwrap();
    Server::http(addr).unwrap().handle(serve).unwrap();
}

fn serve(req: Request, res: Response) {
    debug!("got a request!");
    debug!("remote addr: {}", req.remote_addr);
    debug!("methods: {}", req.method);
    debug!("headers: {}", req.headers);
    debug!("uri: {}", req.uri);
    debug!("version: {}", req.version);

    drop(res);
}
