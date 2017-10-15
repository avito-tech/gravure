use errors::*;
use std::collections::HashMap;
//use futures_pool::Sender;
use tokio_core::reactor::Remote as Sender;

use actions::*;

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
    fn init(&mut self, executor: Sender) -> Result<(), ActionError> {
        for params in &self.actions_raw {
            params.get(0).ok_or(ActionError::Parameter)?;
            let action = Action::from_params(params, executor.clone()).unwrap();
            self.actions.push(action);
        }
        Ok(())
    }
}

// TODO: Enforce init somehow
impl Config {
    pub fn init(&mut self, executor: Sender) -> Result<(), ConfigError> {
        for (_, preset) in &mut self.presets {
            for task in &mut preset.tasks {
                task.init(executor.clone()).map_err(ConfigError::Init)?;
            }
        }
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use config::*;
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
        let config: Config = from_str(test_config).unwrap();
        assert_eq!(config.presets.len(), 1);
        let keys = config.presets.keys().collect::<Vec<_>>();
        assert_eq!(keys, vec!["preset1"]);
        let tasks = &config.presets.get("preset1").unwrap().tasks;
        assert_eq!(tasks.len(), 2);
    }
}
