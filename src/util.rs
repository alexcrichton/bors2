use std::error::Error;
use conduit::{Request, Response, Handler};

use errors::*;
use db::RequestTransaction;

pub struct C(pub fn(&mut Request) -> BorsResult<Response>);

impl Handler for C {
    fn call(&self, req: &mut Request) -> Result<Response, Box<Error+Send>> {
        let C(f) = *self;
        match f(req) {
            Ok(resp) => {
                req.commit();
                Ok(resp)
            }
            Err(e) => {
                {
                    error!("top-level error: {}", e);
                    let mut cur = e.cause();
                    while let Some(e) = cur {
                        error!("error: {}", e);
                        cur = e.cause();
                    }
                }
                Err(Box::new(e))
            }
        }
    }
}

