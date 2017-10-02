
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
