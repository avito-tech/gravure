use config::Config;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::time::{SystemTime, UNIX_EPOCH};

use errors::*;
use qs::*;

use regex::Regex;
use futures::{Future, Stream};
use futures::future::ok;
use futures::sync::oneshot;
use futures_pool::Sender;
use tokio_core::reactor::Handle;

use hyper::{self, StatusCode};
use hyper::server::{Request, Response, Service};
use slog_scope;

pub struct GravureServer {
    pub config: Arc<Config>,
    pub ch: Sender,
    upload_dir: String,
    routes: Vec<(Regex, Route)>,
    handle: Handle,
}

enum Route {
    ByPreset,
    UploadTest,
}

impl GravureServer {
    pub fn new(config: Arc<Config>, upload_dir: String, channel: Sender, handle: Handle) -> Self {
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

    fn by_preset(&self, req: Request, preset_name: String, id: u64) -> Result<(), HttpError> {
        if !self.config.presets.contains_key(&preset_name) {
            return Err(HttpError::UnknownPreset);
        }

        let config = self.config.clone();
        let (_resp, _rx) = oneshot::channel::<Job>();

        let mut hasher = DefaultHasher::default();
        preset_name.hash(&mut hasher);
        id.hash(&mut hasher);
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| HttpError::SystemTime(e))?
            .hash(&mut hasher);
        let hash_str = hasher.finish().to_string();

        let filename = self.upload_dir.clone() + "/" + &hash_str + ".png";

        let mut file = File::create(filename.clone())
            .map_err(|e| HttpError::Io(e))?;
        let chan = self.ch.clone();
        let client_log = match req.remote_addr() {
            Some(addr) => format!("{}", addr),
            None => "unknown".to_string(),
        };

        let client_log = Arc::new(client_log);
        let client = client_log.clone();
        let read_body = req.body()
            .fold(0, move |bytes, chunk| {
                // we fold to count bytes received
                file.write(chunk.as_ref())
                    .map_err(|e| hyper::Error::Io(e))
                    .map(|add| bytes + add)
            })
            .and_then(move |bytes| {
                          info!("Received {:?} bytes", bytes; "handler"=>"upload", "client"=>client_log.clone());
                          Ok(())
                      })
            .and_then(move |_| {
                let preset = config.presets.get(&preset_name).unwrap();
                // TODO: think when notification needs and does it
                for task in &preset.tasks {
                    let job = Job {
                        image_id: id,
                        image_path: filename.to_string(),
                        task: task.clone(),
                        //response: Some(resp),
                        response: None,
                        client: client.clone(),
                    };

                    job.spawn(chan.clone());
                }

                Ok(())
            });
        self.handle.spawn(read_body.then(|_| Ok(())));
        Ok(())
    }

    fn upload_test(&self, req: Request) -> Result<(), HttpError> {
        let filename = "upload/image.png";
        let mut file = try!(File::create(filename).map_err(|e| HttpError::Io(e)));
        let read_body = req.body()
            .fold(0, move |bytes, chunk| {
                // we fold to count bytes received
                file.write(chunk.as_ref())
                    .map_err(|e| hyper::Error::Io(e))
                    .map(|add| bytes + add)
            })
            .and_then(move |bytes| {
                          info!("Received {:?} bytes", bytes; "handler"=>"upload");
                          Ok(())
                      });
        self.handle.spawn(read_body.then(|_| Ok(())));
        Ok(())
    }
}

impl Service for GravureServer {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let mut resp = Response::new();

        let client_log = match req.remote_addr() {
            Some(addr) => format!("{}", addr),
            None => "unknown".to_string(),
        };
        slog_scope::scope(&slog_scope::logger()
                                   .new(slog_o!("scope" => "request handler", "client"=>client_log)),
                                   || {
        if let Err(_e) = self.route(req) {
            info!("HTTP server error: {}", _e);
            resp.set_status(StatusCode::BadRequest);
        }

        Box::new(ok(resp))
                                   })
    }
}
