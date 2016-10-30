use std::str;

use curl;
use rustc_serialize::json;

error_chain! {
    foreign_links {
        curl::Error, Curl;
        json::DecoderError, Json;
        str::Utf8Error, NotUtf8;
    }
}
