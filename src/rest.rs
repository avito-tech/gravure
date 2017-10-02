use config::Config;
use std::fs::File;
use std::io::copy;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::hash::{Hash, SipHasher, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

use errors::*;

use std::sync::mpsc::{Sender, Receiver};
use qs::*;

use hyper::server::{Handler, Request, Response};
use hyper::uri::RequestUri;
use hyper::status::StatusCode;

use regex::Regex;

pub struct GravureServer {
    pub config: Arc<Config>,
    pub ch: Mutex<Sender<Arc<Job>>>,
    upload_dir: String,
    routes: Vec<(Regex, Route)>,
}

enum Route {
    ByPreset,
    UploadTest,
}

impl GravureServer {
    pub fn new(config: Config, upload_dir: String, channel: Sender<Arc<Job>>) -> Self {
        let mut routes = Vec::new();
        routes.push((Regex::new("^/v1/upload/([a-z0-9_]+)/([0-9]+)$").unwrap(), Route::ByPreset));
        routes.push((Regex::new("^/upload/test$").unwrap(), Route::UploadTest));

        GravureServer {
            config: Arc::new(config),
            ch: Mutex::new(channel),
            upload_dir: upload_dir,
            routes: routes,
        }
    }

    fn route(&self, req: Request) -> Result<(), HttpError> {
        if let RequestUri::AbsolutePath(s) = req.uri.clone() {
            // TODO avoid clone
            for &(ref re, ref route) in &self.routes {
                if let Some(caps) = re.captures(&s) {
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
        } else {
            Err(HttpError::UnknownURI)
        }

    }

    fn by_preset(&self, mut req: Request, preset_name: String, id: u64) -> Result<(), HttpError> {
        let preset = try!(self.config.presets.get(&preset_name).ok_or(HttpError::UnknownPreset));
        for task in &preset.tasks {
            let (resp, _rx): (Option<Mutex<Sender<()>>>, Option<Receiver<()>>) = (Option::None,
                                                                                  Option::None);

            let mut hasher = SipHasher::default();

            preset_name.hash(&mut hasher);
            id.hash(&mut hasher);
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().hash(&mut hasher);

            let filename = self.upload_dir.clone() + "/" + &hasher.finish().to_string() + ".png";

            let mut file = try!(File::create(filename.clone()).map_err(|e| HttpError::Io(e)));
            let bytes = try!(copy(&mut req, &mut file).map_err(|e| HttpError::Io(e)));
            println!("Received {:?} bytes", bytes);

            let job = Arc::new(Job {
                image_id: id,
                image_path: filename.to_string(),
                task: task.clone(),
                response: resp,
            });

            match self.ch.lock() {
                Ok(chan) => {
                    match chan.send(job) {
                        Err(e) => {
                            return Err(HttpError::Send(e.description().to_string()));
                        }
                        Ok(()) => println!("Match OK"),
                    }
                }
                Err(e) => {
                    return Err(HttpError::Lock(e.description().to_string()));
                }
            };
        }
        Ok(())
    }

    fn upload_test(&self, mut req: Request) -> Result<(), HttpError> {
        let filename = "upload/result/image.png";
        let mut file = try!(File::create(filename).map_err(|e| HttpError::Io(e)));
        let bytes = try!(copy(&mut req, &mut file).map_err(|e| HttpError::Io(e)));
        println!("Received {:?} bytes", bytes);
        Ok(())
    }
}

impl Handler for GravureServer {
    fn handle(&self, req: Request, mut res: Response) {
        if let Err(_e) = self.route(req) {
            println!("HTTP ERROR => {}", _e);
            *res.status_mut() = StatusCode::BadRequest;
        } else {
            return;
        }
    }
}