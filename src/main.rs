extern crate image;
extern crate url;
#[macro_use] extern crate quick_error;
extern crate serde;
extern crate serde_json;
extern crate scoped_pool;
extern crate clap;
extern crate hyper;
extern crate regex;
extern crate liquid;
extern crate multipart;

pub mod config;
pub mod errors;
pub mod rest;
pub mod actions;
pub mod qs;
pub mod template;

use config::*;
use std::fs::File;

use qs::*;
use std::sync::mpsc;
use serde_json::from_reader;

use std::thread;

use clap::{Arg, App};
use hyper::server::Server;

fn main() {

    let matches = App::new("Avito gravure - The best image service ever")
        .arg(Arg::with_name("config")
             .short("c")
             .long("config")
             .value_name("FILE")
             .help("Sets a custom config file")
             .takes_value(true)
            )
        .arg(Arg::with_name("listen")
             .short("l")
             .long("listen")
             .value_name("HOST:[PORT]")
             .help("Listening parameters")
             .takes_value(true)
            )
        .arg(Arg::with_name("threads")
             .short("n")
             .long("threads")
             .value_name("NUM")
             .help("number of thread")
             .takes_value(true)
            )
        .get_matches();

    let config = matches.value_of("config").unwrap_or("config_test.json");
    let config = File::open(config).unwrap();
    let mut config: Config = from_reader(config).unwrap();
    config.init().unwrap();

    let listen = matches.value_of("listen").unwrap_or("0.0.0.0:4444");
    let threads = matches.value_of("threads").unwrap_or("8").parse().unwrap();

    let (job_s, job_r) = mpsc::channel();
    let server = rest::GravureServer::new(config, "upload".to_owned(), job_s);

    let queue = Queue::new(threads, job_r);
    let handle = thread::spawn(move || {queue.run();});
    Server::http(listen).unwrap().handle(server).unwrap();
    handle.join().unwrap();
}
