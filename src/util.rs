use std::error::Error;
use std::collections::HashMap;
use std::io::{self, Cursor};

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

pub fn html(text: &str) -> Response {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(),
                   vec!["text/html; charset=utf-8".to_string()]);
    headers.insert("Content-Length".to_string(), vec![text.len().to_string()]);
    Response {
        status: (200, "OK"),
        headers: headers,
        body: Box::new(Cursor::new(text.to_string().into_bytes())),
    }
}

pub fn redirect(url: &str) -> Response {
    let mut headers = HashMap::new();
    headers.insert("Location".to_string(), vec![url.to_string()]);
    headers.insert("Content-Length".to_string(), vec!["0".to_string()]);
    Response {
        status: (302, "Found"),
        headers: headers,
        body: Box::new(io::empty()),
    }
}

pub trait RequestFlash {
    fn set_flash_error(&mut self, err: &str);
    fn flash_error(&self) -> Option<&str>;
}

struct FlashError(String);

impl<'a> RequestFlash for Request + 'a {
    fn set_flash_error(&mut self, err: &str) {
        self.mut_extensions().insert(FlashError(err.to_string()));
    }

    fn flash_error(&self) -> Option<&str> {
        self.extensions().find::<FlashError>().map(|s| &*s.0)
    }
}
