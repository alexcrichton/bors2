extern crate bors2;
extern crate civet;
extern crate env_logger;
extern crate handlebars;
extern crate oauth2;
extern crate rand;
extern crate rustc_serialize;
extern crate url;

#[macro_use]
extern crate log;

use std::env;
use std::sync::Arc;
use std::net::SocketAddr;
use std::sync::mpsc::channel;
use std::fs::File;

use bors2::env;
use civet::Server;

fn main() {
    env_logger::init().unwrap();
    let addr = env::args().nth(1).unwrap_or("127.0.0.1:3000".to_string());
    let addr = addr.parse::<SocketAddr>().unwrap();

    let heroku = env::var("HEROKU").is_ok();
    let host = if heroku {
        format!("https://bors2-test.herokuapp.com")
    } else {
        format!("http://localhost:3000")
    };

    let bors_env = if heroku {
        bors2::Env::Production
    } else {
        bors2::Env::Development
    };
    let config = bors2::Config {
        session_key: env("SESSION_KEY"),
        gh_client_id: env("GH_CLIENT_ID"),
        gh_client_secret: env("GH_CLIENT_SECRET"),
        db_url: env("DATABASE_URL"),
        env: bors_env,
        host: host,
    };
    let app = bors2::app::App::new(&config);
    let app = bors2::middleware(Arc::new(app));

    let threads = if bors_env == bors2::Env::Development {1} else {5};
    let mut cfg = civet::Config::new();
    cfg.port(addr.port()).threads(threads).keep_alive(true);
    let _a = Server::start(cfg, app);
    println!("listening on port {}", addr.port());
    if heroku {
        File::create("/tmp/app-initialized").unwrap();
    }

    // TODO: handle a graceful shutdown by just waiting for a SIG{INT,TERM}
    let (_tx, rx) = channel::<()>();
    rx.recv().unwrap();
}
