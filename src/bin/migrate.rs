#![deny(warnings)]

extern crate bors2;
extern crate migrate;
extern crate postgres;

use std::env;
use std::collections::HashSet;

use migrate::Migration;
use postgres::transaction::Transaction;

use bors2::env;

#[allow(dead_code)]
fn main() {
    let conn = postgres::Connection::connect(&env("DATABASE_URL")[..],
                                             postgres::TlsMode::None).unwrap();
    let migrations = migrations();

    let arg = env::args().nth(1);
    if arg.as_ref().map(|s| &s[..]) == Some("rollback") {
        rollback(conn.transaction().unwrap(), migrations).unwrap();
    } else {
        apply(conn.transaction().unwrap(), migrations).unwrap();
    }
}

fn apply(tx: Transaction,
         migrations: Vec<Migration>) -> postgres::Result<()> {
    let mut mgr = try!(migrate::Manager::new(tx));
    for m in migrations.into_iter() {
        try!(mgr.apply(m));
    }
    mgr.set_commit();
    mgr.finish()
}

fn rollback(tx: Transaction,
            migrations: Vec<Migration>) -> postgres::Result<()> {
    let mut mgr = try!(migrate::Manager::new(tx));
    for m in migrations.into_iter().rev() {
        if mgr.contains(m.version()) {
            try!(mgr.rollback(m));
            break
        }
    }
    mgr.set_commit();
    mgr.finish()
}

fn migrations() -> Vec<Migration> {
    let migrations = vec![
        Migration::add_table(20161030140653, "projects", "
            id                      SERIAL PRIMARY KEY,
            repo_user               VARCHAR NOT NULL,
            repo_name               VARCHAR NOT NULL,
            github_webhook_secret   VARCHAR NOT NULL,
            github_access_token     VARCHAR NOT NULL,
            travis_access_token     VARCHAR,
            appveyor_token          VARCHAR
        "),
        Migration::add_table(20161030140654, "events", "
            id                      SERIAL PRIMARY KEY,
            provider_id             INTEGER NOT NULL,
            provider_event_id       VARCHAR NOT NULL,
            provider_event          VARCHAR NOT NULL,
            event                   VARCHAR NOT NULL,
            created_at              TIMESTAMP NOT NULL default now(),
            state                   INTEGER NOT NULL,
            processed_at            TIMESTAMP NOT NULL default now()
        "),
        // Migration::add_table(20161030140653, "pull_requests", "
        //     id          SERIAL PRIMARY KEY,
        //     number      INTEGER NOT NULL,
        //     github_id   INTEGER NOT NULL,
        //     status      INTEGER NOT NULL,
        //     head_ref    VARCHAR NOT NULL,
        //     head_commit VARCHAR NOT NULL,
        //     title       VARCHAR NOT NULL,
        //     approved_by VARCHAR,
        //     mergeable   BOOLEAN NOT NULL,
        //     assignee    VARCHAR,
        //     priority    INTEGER NOT NULL,
        //     rollup      BOOLEAN NOT NULL,
        //     created_at  TIMESTAMP NOT NULL DEFAULT now()
        // "),
    ];
    // NOTE: Generate a new id via `date +"%Y%m%d%H%M%S"`

    let mut seen = HashSet::new();
    for m in migrations.iter() {
        if !seen.insert(m.version()) {
            panic!("duplicate id: {}", m.version());
        }
    }
    return migrations;

    // fn foreign_key(id: i64, table: &str, column: &str,
    //                references: &str) -> Migration {
    //     let add = format!("ALTER TABLE {table} ADD CONSTRAINT fk_{table}_{col}
    //                              FOREIGN KEY ({col}) REFERENCES {reference}",
    //                       table = table, col = column, reference = references);
    //     let rm = format!("ALTER TABLE {table} DROP CONSTRAINT fk_{table}_{col}",
    //                       table = table, col = column);
    //     Migration::run(id, &add, &rm)
    // }
    //
    // fn undo_foreign_key(id: i64, table: &str,
    //                     column: &str,
    //                     real_column: &str,
    //                     references: &str) -> Migration {
    //     let add = format!("ALTER TABLE {table} ADD CONSTRAINT fk_{table}_{col}
    //                        FOREIGN KEY ({real_col}) REFERENCES {reference}",
    //                       table = table, col = column, reference = references,
    //                       real_col = real_column);
    //     let rm = format!("ALTER TABLE {table} DROP CONSTRAINT fk_{table}_{col}",
    //                      table = table, col = column);
    //     Migration::run(id, &rm, &add)
    // }
    //
    // fn index(id: i64, table: &str, column: &str) -> Migration {
    //     let add = format!("CREATE INDEX index_{table}_{column}
    //                        ON {table} ({column})",
    //                       table = table, column = column);
    //     let rm = format!("DROP INDEX index_{table}_{column}",
    //                      table = table, column = column);
    //     Migration::run(id, &add, &rm)
    // }
}
