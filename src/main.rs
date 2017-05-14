extern crate chrono;
extern crate iron;
extern crate persistent;
extern crate r2d2;
extern crate r2d2_sqlite;
extern crate router;
extern crate rusqlite;
extern crate rustc_serialize;
extern crate sha2;

use chrono::*;

use iron::prelude::*;
use iron::Url;
use iron::modifiers::Redirect;
use iron::status::Status;
use iron::typemap::Key;

use persistent::Read;

use r2d2::Pool;

use r2d2_sqlite::SqliteConnectionManager;

use router::Router;

use rusqlite::Connection;

use rustc_serialize::hex::ToHex;

use sha2::{Digest, Sha256};

use std::env;

const HOST: &'static str = "https://yaus.pw/";

pub type SqlitePool = Pool<SqliteConnectionManager>;

pub struct YausDb;
impl Key for YausDb {
    type Value = SqlitePool;
}

// A simple macro to return early if the URL can't parse.
macro_rules! try_url {
    ($url:expr) => {
        match Url::parse($url) {
            Ok(_) => { },
            Err(_) => {
                return Ok(Response::with((Status::BadRequest, "Malformed URL")));
            }
        }
    }
}

fn create_shortened_url(db: &Connection, long_url: &str) -> IronResult<Response> {
    let mut d = Sha256::default();
    d.input(long_url.as_bytes());
    let locator = d.result().as_slice().to_hex();

    let timestamp = Local::now().to_rfc3339();
    db.execute("INSERT INTO urls VALUES (NULL, ?1, ?2, ?3)",
        &[&timestamp, &long_url, &&locator[0..7]]).unwrap();

    Ok(Response::with((Status::Created, [HOST, &locator[0..7]].join(""))))
}

/// Given a long URL, see if it already exists in the table, else create an entry and return
/// it.
///
/// A 200 means that a shortened URL already exists and has been returned. A 201
/// response means that a new shortened URL has been created.
fn check_or_shorten_url(db: &Connection, long_url: &str) -> IronResult<Response> {
    let mut stmt = db.prepare("SELECT locator FROM urls WHERE url = (?)").unwrap();
    let mut row = stmt.query_map::<String, _>(&[&long_url], |r| r.get(0)).unwrap();

    if let Some(l) = row.next() {
        return Ok(Response::with((Status::Ok, [HOST, &l.unwrap()].join(""))));
    }
    create_shortened_url(db, long_url)
}

/// The handler to shorten a URL.
fn shorten_handler(req: &mut Request) -> IronResult<Response> {
    match req.url.clone().query() {
        None => { Ok(Response::with((Status::BadRequest, "URL missing in query"))) },
        Some(s) => {
            let (k, v) = s.split_at(4);
            if k == "url=" {
                try_url!(v);
                let pool = req.get::<Read<YausDb>>().unwrap().clone();
                let db = pool.get().unwrap();
                check_or_shorten_url(&db, v)
            } else {
                Ok(Response::with((Status::BadRequest, "Malformed query string")))
            }
        }
    }
}

/// The handler that redirects to the long URL.
fn redirect_handler(req: &mut Request) -> IronResult<Response> {
    let pool = req.get::<Read<YausDb>>().unwrap().clone();
    let db = pool.get().unwrap();
    let locator = req.url.path()[0];
    let mut stmt = db.prepare("SELECT url FROM urls WHERE locator = (?)").unwrap();
    let mut row = stmt.query_map::<String, _>(&[&locator], |r| r.get(0)).unwrap();

    if let Some(u) = row.next() {
        let long_url = Url::parse(&u.unwrap()).unwrap();
        Ok(Response::with((Status::MovedPermanently, Redirect(long_url))))
    } else {
        Ok(Response::with((Status::NotFound, "Not found")))
    }
}

fn index_handler(_: &mut Request) -> IronResult<Response> {
    Ok(Response::with((Status::Ok, "See https://github.com/gsquire/yaus for the API")))
}

fn main() {
    let mut router = Router::new();
    router.get("/shorten", shorten_handler, "shorten");
    router.get("/:locator", redirect_handler, "locator");
    router.get("/", index_handler, "index");

    let config = r2d2::Config::default();
    let db = env::var("SHORT_DB").unwrap();
    let manager = SqliteConnectionManager::new(&db);
    let pool = r2d2::Pool::new(config, manager).unwrap();

    let mut chain = Chain::new(router);
    chain.link_before(Read::<YausDb>::one(pool));

    Iron::new(chain).http("localhost:3000").unwrap();
}
