#![feature(proc_macro)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate diesel_codegen;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate error_chain;
extern crate dotenv;
extern crate rustc_serialize;
extern crate curl;
extern crate oauth2;

use diesel::prelude::*;
use diesel::pg::PgConnection;
use dotenv::dotenv;
use std::env;

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

pub mod schema;
pub mod models;
pub mod http;
pub mod github;
pub mod errors;
pub mod travis;
