use std::str;

use curl::easy::{Easy, List};
use oauth2::Token;
use rustc_serialize::{json, Decodable, Encodable};

use errors::*;

pub fn github_get<T>(url: &str, auth: &Token) -> Result<T>
    where T: Decodable,
{
    let headers = vec![
        format!("Authorization: token {}", auth.access_token),
        format!("Accept: application/vnd.github.v3+json"),
    ];

    get(&format!("https://api.github.com{}", url), &headers)
}

pub fn github_post<T, U>(url: &str, auth: &Token, u: &U) -> Result<T>
    where T: Decodable,
          U: Encodable,
{
    let headers = vec![
        format!("Authorization: token {}", auth.access_token),
        format!("Accept: application/vnd.github.v3+json"),
    ];

    post(&format!("https://api.github.com{}", url), &headers, u)
}

pub fn github_delete(url: &str, auth: &Token) -> Result<()> {
    let headers = vec![
        format!("Authorization: token {}", auth.access_token),
        format!("Accept: application/vnd.github.v3+json"),
    ];

    delete(&format!("https://api.github.com{}", url), &headers)
}

pub fn get<T>(url: &str, headers: &[String]) -> Result<T>
    where T: Decodable,
{
    let mut handle = Easy::new();
    let mut list = List::new();
    try!(list.append("User-Agent: hello!"));
    for header in headers {
        try!(list.append(header));
    }

    try!(handle.http_headers(list));
    try!(handle.get(true));
    try!(handle.url(url));
    perform(&mut handle, url)
}

pub fn post<T, U>(url: &str, headers: &[String], u: &U) -> Result<T>
    where U: Encodable,
          T: Decodable,
{
    let mut handle = Easy::new();
    let mut list = List::new();
    try!(list.append("User-Agent: hello!"));
    for header in headers {
        try!(list.append(header));
    }

    try!(handle.http_headers(list));
    try!(handle.post(true));
    try!(handle.post_fields_copy(json::encode(u).unwrap().as_bytes()));
    try!(handle.url(url));
    perform(&mut handle, url)
}

pub fn delete(url: &str, headers: &[String]) -> Result<()> {
    let mut handle = Easy::new();
    let mut list = List::new();
    try!(list.append("User-Agent: hello!"));
    for header in headers {
        try!(list.append(header));
    }

    try!(handle.http_headers(list));
    try!(handle.custom_request("DELETE"));
    try!(handle.url(url));
    perform(&mut handle, url)
}

fn perform<T: Decodable>(handle: &mut Easy, url: &str) -> Result<T> {
    let mut headers = Vec::new();
    let mut data = Vec::new();

    {
        let mut t = handle.transfer();
        try!(t.header_function(|data| {
            headers.push(data.to_owned());
            true
        }));
        try!(t.write_function(|buf| {
            data.extend_from_slice(&buf);
            Ok(buf.len())
        }));

        debug!("sending a request to {}", url);
        try!(t.perform().chain_err(|| {
            format!("failed to send http requests to {}", url)
        }));
    }

    match try!(handle.response_code()) {
        200 |
        201 |
        204 => {} // Ok!
        code => {
            return Err(format!("not a 200 code: {}\n\n{}\n", code,
                               String::from_utf8_lossy(&data)).into())
        }
    }

    let json = try!(str::from_utf8(&data).chain_err(|| {
        "github didn't send utf-8"
    }));
    json::decode(json).chain_err(|| {
        "failed to parse json"
    })
}
