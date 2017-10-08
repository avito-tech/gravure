extern crate image;
extern crate url;
#[macro_use]
extern crate quick_error;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate scoped_pool;
extern crate clap;
extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
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
use futures::sync::mpsc;
use serde_json::from_reader;
use futures::{Future, Sink, Stream};
use tokio_core::net::TcpListener;
use tokio_core::reactor::{Core, Handle};

use std::thread;

use clap::{Arg, App};
use hyper::server::Http;

fn main() {

    let matches = App::new("Avito gravure - The best image service ever")
        .arg(Arg::with_name("config")
                 .short("c")
                 .long("config")
                 .value_name("FILE")
                 .help("Sets a custom config file")
                 .takes_value(true))
        .arg(Arg::with_name("listen")
                 .short("l")
                 .long("listen")
                 .value_name("HOST:[PORT]")
                 .help("Listening parameters")
                 .takes_value(true))
        .arg(Arg::with_name("threads")
                 .short("n")
                 .long("threads")
                 .value_name("NUM")
                 .help("number of thread")
                 .takes_value(true))
        .get_matches();

    let config = matches.value_of("config").unwrap_or("config_test.json");
    let config = File::open(config).unwrap();
    let mut config: Config = from_reader(config).unwrap();
    config.init().unwrap();

    let listen = matches.value_of("listen").unwrap_or("0.0.0.0:4444");
    let threads = matches
        .value_of("threads")
        .unwrap_or("8")
        .parse()
        .unwrap();

    let (job_s, job_r) = mpsc::unbounded();

    let queue = Queue::new(threads, job_r);
    let threads = thread::spawn(move || { queue.run(); });

    // Run event loop in main thread
    let listen = listen.parse().unwrap();
    // Create the event loop that will drive this server
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    // Bind the server's socket
    let listener = TcpListener::bind(&listen, &handle).unwrap();
    let server = listener
        .incoming()
        .for_each(|(sock, addr)| {
                      let server = rest::GravureServer::new(config,
                                                            "upload".to_string(),
                                                            job_s,
                                                            handle.clone());
                      Http::new().bind_connection(&handle, sock, addr, server);
                      Ok(())
                  });
    core.run(server).unwrap();
    threads.join().unwrap();
}
