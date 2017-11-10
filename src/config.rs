use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use toml;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub general: General,
    pub consul: Consul,
    pub present: Present,
    pub services: HashMap<String, Vec<Service>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct General {
    pub project_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Consul {
    pub urls: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Present {
    pub templates: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Service {
    pub name: String,
    pub url: String,
}

impl Default for Config {
    fn default() -> Config {
        let general = General { project_name: "Service World".to_string() };
        let consul = Consul { urls: vec!["http://localhost:8500".to_string()] };
        let present = Present { templates: HashMap::new() };
        let services = HashMap::new();

        Config { general, consul, present, services }
    }
}

impl Config {
    pub fn from_file(file_path: &Path) -> Result<Config> {
        let mut file = File::open(file_path)?;
        let content = Config::read_to_string(&mut file)?;

        Config::parse_toml(&content)
    }

    fn read_to_string(file: &mut File) -> Result<String> {
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        Ok(content)
    }

    fn parse_toml(content: &str) -> Result<Config> {
        let config: Config = toml::from_str(content)?;

        Ok(config)
    }
}

error_chain! {
    foreign_links {
        CouldNotRead(::std::io::Error);
        CouldNotParse(::toml::de::Error);
    }
}
