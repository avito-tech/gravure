use errors::*;
use template::PathTemplate;

use std::ascii::AsciiExt;
use std::path::Path;
use std::fs::File;
use std::string::String;

use image;
use image::DynamicImage;
use image::ImageFormat;
use image::FilterType;
use image::ImageError;

use hyper::Uri;
use hyper::client::Request;
use multipart::client::Multipart;

#[derive(Clone)]
pub struct ImageData {
    pub image: DynamicImage,
    pub image_format: ImageFormat,
    pub id: u64,
}

impl ImageData {
    pub fn new(image_path: String, image_id: u64) -> Result<ImageData, ImageError> {
        let img = try!(image::open(image_path.clone()));
        println!("ImageData: IMAGE OK");

        let image_format = try!(ImageData::get_format(image_path));
        println!("ImageData: IMAGE FORMAT OK");

        Ok(ImageData {
               image: img,
               image_format: image_format,
               id: image_id,
           })
    }

    fn get_format(image_path: String) -> Result<ImageFormat, ImageError> {
        let path = Path::new(&image_path);
        let ext = path.extension()
            .and_then(|s| s.to_str())
            .map_or("".to_string(), |s| s.to_ascii_lowercase());

        Ok(match &ext[..] {
               "jpg" | "jpeg" => image::ImageFormat::JPEG,
               "png" => image::ImageFormat::PNG,
               format => {
                   return Err(image::ImageError::UnsupportedError(format!("Image format image/{:?} \
                                                                        is not supported.",
                                                                          format)))
               }
           })
    }
}


#[derive(Clone)]
pub enum Action {
    Resize(Resizer),
    Save(Saver),
    Upload(Uploader),
}

impl Action {
    pub fn from_params(params: &Vec<String>) -> Result<Self, ActionError> {
        let cmd = try!(params.get(0).ok_or(ActionError::Parameter));
        match cmd.as_str() {
            "resize" => Ok(Action::Resize(try!(build_resizer(params)))),
            "save" => Ok(Action::Save(try!(build_saver(params)))),
            "upload" => Ok(Action::Upload(try!(build_uploader(params)))),
            _ => Err(ActionError::Wrong),
        }
    }

    pub fn run(&self, image_data: &mut ImageData) -> Result<ImageData, ActionError> {
        match self {
            &Action::Resize(ref r) => r.run(image_data),
            &Action::Save(ref s) => s.run(image_data),
            &Action::Upload(ref u) => u.run(image_data),
        }
    }
}

#[derive(Clone)]
pub struct Resizer {
    width: u32,
    height: u32,
    filter: FilterType,
}

pub fn build_resizer(params: &Vec<String>) -> Result<Resizer, ActionError> {
    let mut iter = params.iter();
    try!(iter.next().ok_or(ActionError::Parameter));
    let width = try!(iter.next().ok_or(ActionError::Parameter));
    let width = try!(width.parse().map_err(|_| ActionError::Parameter));

    let height = try!(iter.next().ok_or(ActionError::Parameter));
    let height = try!(height.parse().map_err(|_| ActionError::Parameter));

    Ok(Resizer {
           width: width,
           height: height,
           filter: FilterType::Gaussian,
       })
}

impl Resizer {
    pub fn run(&self, image_data: &mut ImageData) -> Result<ImageData, ActionError> {
        Ok(ImageData {
               image: image_data
                   .image
                   .resize(self.width, self.height, self.filter),
               image_format: image_data.image_format,
               id: image_data.id,
           })
    }
}

#[derive(Clone)]
pub struct Saver {
    path_template: String,
}

pub fn build_saver(params: &Vec<String>) -> Result<Saver, ActionError> {
    let mut iter = params.iter();
    try!(iter.next().ok_or(ActionError::Parameter));

    // liquid::Renderable(which is Box<Vec<Renderable>>) cannot be passed between threads
    // So we can only check it for corectness, but cannot save it inside Sender
    let path_template = try!(iter.next().ok_or(ActionError::Parameter));
    try!(PathTemplate::new(path_template.clone()).map_err(|_| ActionError::Parameter));
    // Ok(Saver { path: "./".to_owned() })
    Ok(Saver { path_template: path_template.clone() })
}

impl Saver {
    pub fn run(&self, image_data: &mut ImageData) -> Result<ImageData, ActionError> {
        let template = try!(PathTemplate::new(self.path_template.clone())
            .map_err(|_| ActionError::Parameter));

        let extension = try!(match image_data.image_format {
                                 ImageFormat::JPEG => Ok("jpg"),
                                 ImageFormat::PNG => Ok("png"),
                                 _ => {
                Err(ActionError::Image(ImageError::UnsupportedError("Image format is not \
                                                                     supported."
                    .to_string())))
            }
                             });

        let path = try!(template
                            .render(image_data.id, extension.to_owned())
                            .map_err(|e| ActionError::BadTemplate(e)));
        println!("SAVING to {:?}", path);
        let mut file = try!(File::create(path).map_err(|e| ActionError::Io(e)));

        try!(image_data
                 .image
                 .save(&mut file, image_data.image_format)
                 .map_err(|e| ActionError::Image(e)));
        Ok((*image_data).clone())
    }
}


#[derive(Clone)]
pub struct Uploader {
    path_template: String,
}

pub fn build_uploader(params: &Vec<String>) -> Result<Uploader, ActionError> {
    let mut iter = params.iter();
    try!(iter.next().ok_or(ActionError::Parameter));

    let path_template = try!(iter.next().ok_or(ActionError::Parameter));
    try!(PathTemplate::new(path_template.clone()).map_err(|_| ActionError::Parameter));

    Ok(Uploader { path_template: path_template.clone() })
}

impl Uploader {
    pub fn run(&self, image_data: &mut ImageData) -> Result<ImageData, ActionError> {
        let template = try!(PathTemplate::new(self.path_template.clone())
            .map_err(|_| ActionError::Parameter));

        let path = try!(template
                            .render(image_data.id, "jpg".to_owned())
                            .map_err(|e| ActionError::BadTemplate(e)));

        // let path = &self.path_template;

        println!("FAKE UPLOAD to {:?}", path);

        Err(ActionError::Run)
        //let url = try!(Uri::parse(&path).map_err(|e| ActionError::UrlParse(e)));
        //let request = try!(Request::new(Method::Post, url)
        //.map_err(|e| ActionError::HyperRequestError(e)));

        //let mut multipart = try!(Multipart::from_request(request)
        //.map_err(|e| ActionError::HyperRequestError(e)));

        ////        let mut buffer = BufStream::new(&mut bf);
        ////        let dir = try!(TempDir::new("gravure").map_err(|e| ActionError::Io(e)));
        ////        let file_path = dir.path().join(image_data.id.to_string());

        ////  let id = image_data.id.to_string();
        //// let mut file_path = "/tmp/gravure_".to_string();
        //// file_path.push_str(&id);

        //// println!("Create temp file {:?}", file_path);

        //// let mut file = try!(File::create(&file_path).map_err(|e| ActionError::Io(e)));

        //// image_data.image.save(&mut file, ImageFormat::JPEG);
        //// /        image_data.image.save(&mut buffer, ImageFormat::JPEG);

        //// let mut file = try!(File::open(&file_path).map_err(|e| ActionError::Io(e)));
        //let buf = image_data.image.raw_pixels();
        //multipart
        //.write_stream("file", &mut buf.as_slice(), None, None)
        //.unwrap();
        ////        multipart.write_stream("file", &mut buffer, None, None).unwrap();

        //println!("Send file {:?}", image_data.id);

        //try!(multipart
        //.send()
        //.map_err(|e| ActionError::HyperRequestError(e)));

        //Ok((*image_data).clone())
    }
}
