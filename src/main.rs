#[macro_use] extern crate lazy_static;

extern crate iron;
extern crate router;
extern crate rusqlite;
extern crate sha2;
extern crate url;

use iron::prelude::*;
use iron::Url;
use iron::modifiers::Redirect;
use iron::status::Status;

use router::Router;

use rusqlite::Connection;

use sha2::digest::Digest;
use sha2::sha2::Sha256;

use url::form_urlencoded;

use std::env;
use std::sync::Mutex;

lazy_static! {
    static ref DB_CONN: Mutex<Connection> = {
        let db = env::var("SHORT_DB").unwrap();
        let c = Connection::open(db).unwrap();
        Mutex::new(c)
    };
}

// TODO: make sure to share the connection. possibly with a mutex?
// TODO: should I cache these prepared statements? make an index as well.
// TODO: add unique constraints.
// TODO: URL parsing validation.
fn create_shortened_url(long_url: &str) -> IronResult<Response> {
    let mut hash = Sha256::new();
    hash.input_str(long_url);
    let locator = hash.result_str();

    let db = DB_CONN.lock().unwrap();
    // TODO: insert a proper timestamp.
    db.execute("INSERT INTO urls VALUES (NULL, $1, $2, $3)", &[&"now", &long_url, &&locator[0..7]]).unwrap();

    // TODO: update this status.
    Ok(Response::with((Status::Ok, &locator[0..7])))
}

/// Given a long URL, see if it already exists in the table, else create an entry and return
/// it.
///
/// A 200 means that a shortened URL already exists and has been returned. A 201
/// response means that a new shortened URL has been created.
fn check_or_shorten_url(long_url: &str) -> IronResult<Response> {
    // The scoping here is needed because the lock will only be released once it drops
    // out of scope.
    {
        let db = DB_CONN.lock().unwrap();
        let mut stmt = db.prepare("SELECT locator FROM urls WHERE url = (?)").unwrap();
        // TODO: why is the generic parameter necessary?
        let mut row = stmt.query_map::<String, _>(&[&long_url], |r| r.get(0)).unwrap();

        if let Some(l) = row.next() {
            return Ok(Response::with((Status::Ok, l.unwrap())));
        }
    }
    create_shortened_url(long_url)
}

/// The handler to shorten a URL.
fn shorten_handler(req: &mut Request) -> IronResult<Response> {
    match req.url.query {
        None => { Ok(Response::with((Status::BadRequest, "URL missing in query"))) },
        Some(ref s) => {
            let mut query_string = form_urlencoded::parse(s.as_bytes());
            if let Some(long_url) = query_string.next() {
                check_or_shorten_url(&*long_url.1)
            } else {
                Ok(Response::with((Status::BadRequest, "Malformed query string")))
            }
        }
    }
}

/// The handler that redirects to the long URL.
fn redirect_handler(req: &mut Request) -> IronResult<Response> {
    let locator = req.extensions.get::<Router>().unwrap().find("locator").unwrap();
    let db = DB_CONN.lock().unwrap();
    let mut stmt = db.prepare("SELECT url FROM urls WHERE locator = (?)").unwrap();
    let mut row = stmt.query_map::<String, _>(&[&locator], |r| r.get(0)).unwrap();
    if let Some(u) = row.next() {
        let long_url = Url::parse(&u.unwrap()).unwrap();
        Ok(Response::with((Status::Found, Redirect(long_url))))
    } else {
        Ok(Response::with((Status::NotFound, "Not found")))
    }
}

fn main() {
    let mut router = Router::new();
    router.get("/shorten", shorten_handler);
    router.get("/:locator", redirect_handler);

    Iron::new(router).http("localhost:3000").unwrap();
}
