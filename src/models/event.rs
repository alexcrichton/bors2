
use pg::GenericConnection;
use pg::rows::Row;

use errors::*;

pub struct Event {
    pub id: i32,
    pub provider_id: Provider,
    pub provider_event_id: String,
    pub provider_event: String,
    pub event: String,
}

pub enum Provider {
    GitHub,
    Travis,
    AppVeyor,
}

impl Event {
    pub fn insert(conn: &GenericConnection,
                  provider: Provider,
                  provider_event_id: &str,
                  provider_event: &str,
                  event: &str) -> BorsResult<Event> {
        let stmt = try!(conn.prepare("INSERT INTO events
                                      (provider_id,
                                       provider_event_id,
                                       provider_event,
                                       event,
                                       state)
                                      VALUES ($1, $2, $3, $4, 0)
                                      RETURNING *"));
        let rows = try!(stmt.query(&[&(provider as i32),
                                     &provider_event_id,
                                     &provider_event,
                                     &event]));
        Ok(Event::from_row(&rows.iter().next().unwrap()))
    }

    pub fn from_row(row: &Row) -> Event {
        Event {
            id: row.get("id"),
            provider_id: match row.get("provider_id") {
                0 => Provider::GitHub,
                1 => Provider::Travis,
                2 => Provider::AppVeyor,
                n => panic!("invalid id: {}", n),
            },
            provider_event_id: row.get("provider_event_id"),
            provider_event: row.get("provider_event"),
            event: row.get("event"),
        }
    }
}
