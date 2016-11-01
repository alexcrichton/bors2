
use pg::GenericConnection;
use pg::rows::Row;

use errors::*;

pub struct Event {
    pub id: i32,
    pub provider_id: String,
    pub provider_event_id: String,
    pub provider_event: String,
    pub event: String,
}

pub enum Provider {
    GitHub,
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
                                      RETURNING *").chain_err(|| "wut"));
        let rows = try!(stmt.query(&[&(provider as i32),
                                     &provider_event_id,
                                     &provider_event,
                                     &event]).chain_err(|| "wut2"));
        Ok(Event::from_row(&rows.iter().next().unwrap()))
    }

    pub fn from_row(row: &Row) -> Event {
        Event {
            id: row.get("id"),
            provider_id: row.get("provider_id"),
            provider_event_id: row.get("provider_event_id"),
            provider_event: row.get("provider_event"),
            event: row.get("event"),
        }
    }
}
