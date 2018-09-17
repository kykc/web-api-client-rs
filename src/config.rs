#[derive(Serialize, Deserialize)]
pub struct Config {
    pub width: u32,
    pub height: u32,
    pub url: String,
    pub headers: String,
    pub request: Option<String>,
}

use serde_json;
use std::fs::{File};
use std::error::Error;
use std::io::{Write, Read};
use std::path::{Path, PathBuf};
use dirs;

pub fn write_config(config: &Config) {
    let config_path = get_config_path();
    let j = serde_json::to_string_pretty(&config).unwrap();
    let display = config_path.display();

    let mut file = match File::create(&config_path) {
        Err(why) => panic!("couldn't create {}: {}",
                           display,
                           why.description()),
        Ok(file) => file,
    };

    match file.write_all(j.as_bytes()) {
        Err(why) => {
            panic!("couldn't write to {}: {}", display,
                                               why.description())
        },
        Ok(_) => (),
    }
}

pub fn get_current_config() -> Config {
    let mut default_config = Config{
        height: (600u32), 
        width: (1366u32), 
        url: "https://api.github.com/users/kykc/repos".to_string(), 
        headers: "".to_string(),
        request: Some("".to_string()),
    };

    let config_path = get_config_path();

    if Path::new(config_path.to_str().unwrap()).exists() {
        let mut f = File::open(config_path.clone()).expect("Config file not found");
        let mut contents = String::new();
        f.read_to_string(&mut contents).expect("something went wrong reading config the file");
        default_config = serde_json::from_str(&contents).unwrap();
    }

    write_config(&default_config);

    default_config
}

fn get_config_path() -> PathBuf {
    let mut path: PathBuf = dirs::home_dir().expect("Cannot get use home directory location");
    path.push(".auweb.json");
    
    path
}