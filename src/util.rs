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
            Err(e) => Err(Box::new(e)),
        }
    }
}

