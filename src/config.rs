use errors::*;
use std::collections::HashMap;

use actions::*;

//#include!(concat!(env!("OUT_DIR"), "/config_types.rs"));
#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub presets: HashMap<String, Preset>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Preset {
    pub name: String,
    pub tasks: Vec<Task>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Task {
    pub name: String,

    #[serde(default = "Vec::new")]
    #[serde(skip_deserializing)]
    #[serde(skip_serializing)]
    #[serde(rename = "actions_box")]
    pub actions: Vec<Action>,
    #[serde(rename = "actions")]
    pub actions_raw: Vec<Vec<String>>,
    pub url_template: String,
}

impl Task {
    fn init(&mut self) -> Result<(), ActionError> {
        for params in &self.actions_raw {
            try!(params.get(0).ok_or(ActionError::Parameter));
            // let cmd = params.get(0).unwrap();
            let action = Action::from_params(params).unwrap();
            self.actions.push(action);
        }
        Ok(())
    }
}

// TODO: Enforce init somehow
impl Config {
    ///    MUST be applied to config before any actions on it
    pub fn init(&mut self) -> Result<(), ConfigError> {
        for (_, preset) in &mut self.presets {
            for task in &mut preset.tasks {
                try!(task.init().map_err(ConfigError::Init));
            }
        }
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use config::*;
    use errors::*;
    use image::DynamicImage;
    use actions::*;
    use url;
    use std::collections::HashMap;

    use serde_json::from_str;

    #[test]
    fn parse_config() {
        let test_config = r#"
{
"presets": {
    "preset1": {
        "name": "preset1",
        "tasks": [
            {
            "name": "task1",
            "actions": [
                    [ "atest1", "30", "30" ],
                    [ "atest2", "times", "text" ]
                ],
            "url_template": "http://{{node_id}}/protools/{{task_name}}/{{img_id}}"
            },
            {
            "name": "task2",
            "actions": [
                    [ "atest1", "60", "60" ],
                    [ "atest2", "image", "/usr/share/watermark.jpg", "10%", "br" ]
                ],
            "url_template": "http://{{node_id}}/protools/{{task_name}}/{{img_id}}"
            }
            ]
        }
    }
}"#;
        println!("{}", test_config);
        let mut config: Config = from_str(test_config).unwrap();
        //        println!("{:?}", config);
    }
}
