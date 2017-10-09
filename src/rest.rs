use config::Config;
use std::fs::File;
use std::io::{self, Write, copy};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::time::{SystemTime, UNIX_EPOCH};

use errors::*;
use qs::*;

use regex::Regex;
use futures::{Future, Stream, Sink};
use futures::future::ok;
use futures::sync::mpsc::{UnboundedSender, UnboundedReceiver};
use futures::sync::oneshot;
use tokio_core::reactor::Handle;
use hyper::{self, Uri, StatusCode};
use hyper::header::ContentLength;
use hyper::server::{Http, Request, Response, Service};


pub struct GravureServer {
    pub config: Arc<Config>,
    pub ch: UnboundedSender<Job>,
    upload_dir: String,
    routes: Vec<(Regex, Route)>,
    handle: Handle,
}

enum Route {
    ByPreset,
    UploadTest,
}

impl GravureServer {
    pub fn new(config: Arc<Config>,
               upload_dir: String,
               channel: UnboundedSender<Job>,
               handle: Handle)
               -> Self {
        let mut routes = Vec::new();
        routes.push((Regex::new("^/v1/upload/([a-z0-9_]+)/([0-9]+)$").unwrap(), Route::ByPreset));
        routes.push((Regex::new("^/upload/test$").unwrap(), Route::UploadTest));

        GravureServer {
            config: config,
            ch: channel,
            upload_dir: upload_dir,
            routes: routes,
            handle: handle,
        }
    }

    fn route(&self, req: Request) -> Result<(), HttpError> {
        let uri = req.uri().clone();
        let uri = uri.path();
        for &(ref re, ref route) in &self.routes {
            if let Some(caps) = re.captures(uri) {
                match route {
                    &Route::ByPreset => {
                        let preset = try!(caps.at(1).ok_or(HttpError::UnknownURI)).to_string();
                        let id = try!(caps.at(2).ok_or(HttpError::UnknownURI));
                        let id = try!(id.parse().map_err(|_| HttpError::UnknownURI));
                        return self.by_preset(req, preset, id);
                    }
                    &Route::UploadTest => return self.upload_test(req),
                }
            }
        }
        Err(HttpError::UnknownURI)
    }

    fn by_preset(&self, mut req: Request, preset_name: String, id: u64) -> Result<(), HttpError> {
        let config = self.config.clone();
        let preset = try!(config
                              .presets
                              .get(&preset_name)
                              .ok_or(HttpError::UnknownPreset));


        let (resp, _rx) = oneshot::channel::<Job>();

        let mut hasher = DefaultHasher::default();
        preset_name.hash(&mut hasher);
        id.hash(&mut hasher);
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .hash(&mut hasher);
        let hash_str = hasher.finish().to_string();

        let filename = self.upload_dir.clone() + "/" + &hash_str + ".png";

        let mut file = try!(File::create(filename.clone()).map_err(|e| HttpError::Io(e)));
        let chan = self.ch.clone();
        let read_body = req.body()
            .for_each(move |chunk| {
                          file.write(chunk.as_ref())
                              .map_err(|e| hyper::Error::Io(e))
                              .map(|bytes| {
                                       println!("Received {:?} bytes", bytes);
                                   })
                      })
            .and_then(move |_| {
                for task in &preset.tasks {
                    let job = Job {
                        image_id: id,
                        image_path: filename.to_string(),
                        task: task.clone(),
                        //response: Some(resp),
                        response: None,
                    };

                    chan.send(job);
                    //       .map_err(|e| hyper::Error::Io(io::Error::new(io::ErrorKind::Other, e)))
                }
                Ok(())
            });
        self.handle.spawn(read_body.then(|_| Ok(())));
        Ok(())
    }

    fn upload_test(&self, mut req: Request) -> Result<(), HttpError> {
        let filename = "upload/result/image.png";
        let mut file = try!(File::create(filename).map_err(|e| HttpError::Io(e)));
        let read_body = req.body()
            .for_each(move |chunk| {
                          file.write(chunk.as_ref())
                              .map_err(|e| hyper::Error::Io(e))
                              .map(|bytes| {
                                       println!("Received {:?} bytes", bytes);
                                   })
                      });
        self.handle.spawn(read_body.then(|_| Ok(())));
        Ok(())
    }
}

impl Service for GravureServer {
    // boilerplate hooking up hyper's server types
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let mut resp = Response::new();
        if let Err(_e) = self.route(req) {
            println!("HTTP ERROR => {}", _e);
            resp.set_status(StatusCode::BadRequest);
        }
        // We're currently ignoring the Request
        // And returning an 'ok' Future, which means it's ready
        // immediately, and build a Response with the 'PHRASE' body.
        Box::new(ok(resp))
    }
}
