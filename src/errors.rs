use std::io::Error as IoError;
use liquid::Error as LiquidError;
use image::ImageError;
use hyper::Error as HyperError;
use hyper::error::UriError as UriParseError;

quick_error! {
    #[derive(Debug)]
    pub enum ActionError {
        Parameter {
            description("bad parameters")
        }
        Run {
            description("error running action")
        }
        Wrong {
            description("Wrong action")
        }

        BadTemplate(e: TemplateError) {
            cause(e)
        }

        Io(e: IoError) {
            cause(e)
        }
        Image(e: ImageError){
           cause(e)
        }
        UrlParse(e: UriParseError) {
            cause(e)
        }
        HyperRequestError(e: HyperError) {
            cause(e)
        }
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum JobError {
        Receive {
            description("job receive")
        }

        Image(e: ImageError) {
            cause(e)
            description("image loading")
        }
        Action(e: ActionError) {
            cause(e)
            description("action failed")
        }
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum ConfigError {
        Parse {
            description("config parse error")
        }

        Init(e: ActionError) {
            cause(e)
                description("action init error")
        }
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum TemplateError {
        Convert {
            description("bad template values")
        }

        Engine(e: LiquidError) {
            cause(e)
                description("template engine error")
        }
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum HttpError {
        UnknownURI {
            description("BAD URL")
        }

        Io (e: IoError) {
            cause(e)
            description(e.description())
        }

        UnknownPreset {
            description("Preset is unknown")
        }

        Send(desc: String) {
            description(desc)
        }

        Lock(desc: String) {
            description(desc)
        }
        Hyper(e: HyperError) {
            cause(e)
        }
        SystemTime(e: ::std::time::SystemTimeError) {
            cause(e)
        }
    }
}
