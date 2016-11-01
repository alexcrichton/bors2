use std::io;
use std::str;

use curl;
use rustc_serialize::json;
use pg;

error_chain! {
    types {
        BorsError, BorsErrorKind, BorsChainErr, BorsResult;
    }

    foreign_links {
        curl::Error, Curl;
        json::DecoderError, Json;
        str::Utf8Error, NotUtf8;
        pg::error::Error, PostgresError;
        io::Error, Io;
    }
}
