extern crate image;
extern crate url;
#[macro_use]
extern crate quick_error;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate clap;
extern crate futures;
extern crate futures_pool;
extern crate tokio_core;
extern crate tokio_io;
extern crate hyper;
extern crate regex;
extern crate liquid;
extern crate multipart;
extern crate num_cpus;
#[macro_use]
extern crate slog;
extern crate slog_term;
extern crate slog_json;
extern crate slog_async;
#[macro_use]
extern crate slog_scope;

pub mod config;
pub mod errors;
pub mod rest;
pub mod actions;
pub mod qs;
pub mod template;

use config::*;
use std::fs::File;

use serde_json::from_reader;
use futures::{Future, Stream};

use futures_pool::Pool;
use tokio_core::net::TcpListener;
use tokio_core::reactor::Core;

use std::sync::Arc;

use clap::{Arg, App};
use slog::Drain;
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
                 .help("number of threads for resizing pictures (use 0 for auto)")
                 .takes_value(true))
        .arg(Arg::with_name("loglevel")
                 .short("v")
                 .long("verbose")
                 .value_name("NUM")
                 .help("logging level")
                 .takes_value(true))
        .arg(Arg::with_name("jsonlog")
                 .short("j")
                 .long("json")
                 .help("use json logger"))
        .get_matches();

    // Create the event loop that will drive this server
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let listen = matches.value_of("listen").unwrap_or("0.0.0.0:4444");
    let mut threads = matches
        .value_of("threads")
        .unwrap_or("0")
        .parse()
        .unwrap();

    if threads == 0 {
        threads = num_cpus::get();
    }

    let loglevel = matches
        .value_of("loglevel")
        .unwrap_or("6")
        .parse()
        .unwrap();

    let json = matches.is_present("jsonlog");

    let loglevel = slog::Level::from_usize(loglevel).expect("log level should be 0-6");

    let json_drain = slog_json::Json::default(std::io::stderr())
        .filter(move |_| json.clone())
        .fuse();

    let decorator = slog_term::TermDecorator::new().build();
    let term_drain = slog_term::CompactFormat::new(decorator)
        .build()
        .filter(move |_| !json.clone())
        .fuse();

    let drain = slog::Duplicate::new(json_drain, term_drain).fuse();
    let drain = slog::LevelFilter::new(drain, loglevel).fuse();
    let drain = slog_async::Async::new(drain)
        .chan_size(65536)
        .build()
        .fuse();

    let log = slog::Logger::root(drain, o!("version" => env!("CARGO_PKG_VERSION")));
    let _guard = slog_scope::set_global_logger(log);

    let pool = || Pool::builder().pool_size(threads).build();
    // but sets logging scope
    let (sender, mut pool) = slog_scope::scope(&slog_scope::logger()
                                                    .new(slog_o!("scope" => "threadpool")),
                                               pool);

    info!("Server starting");

    let config = matches.value_of("config").unwrap_or("config_test.json");
    let config = File::open(config).unwrap();
    let mut config: Config = from_reader(config).unwrap();
    config.init(core.remote()).unwrap();
    let config = Arc::new(config);

    // Run event loop in main thread
    let listen = listen.parse().unwrap();

    // Bind the server's socket
    let listener = TcpListener::bind(&listen, &handle).unwrap();
    let server = listener
        .incoming()
        .for_each(|(sock, addr)| {
                      let server = rest::GravureServer::new(config.clone(),
                                                            "upload".to_string(),
                                                            sender.clone(),
                                                            handle.clone());
                      Http::new().bind_connection(&handle, sock, addr, server);
                      Ok(())
                  })
        .then(|_| Ok::<(), ()>(()));
    core.run(server).unwrap();
    pool.shutdown();
}
