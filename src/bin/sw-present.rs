#[macro_use]
extern crate error_chain;
extern crate clap;
extern crate handlebars;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate service_world;
extern crate toml;

use clap::{App, Arg};
use handlebars::{Handlebars, Helper, RenderContext, RenderError};
use service_world::Consul;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

mod config {
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
}

fn run() -> Result<()> {
    use config::Config;

    let args = build_cli().get_matches();

    let config = if let Some(config_file) = args.value_of("config") {
        Config::from_file(Path::new(config_file))
    } else {
        Ok(Default::default())
    }.unwrap();

    // Consul Client should take all URLs and decides which to use by itself.
    let url: &str = args.value_of("url")
        .or(
            // This is Rust at its not so finest: There's no coercing from Option<&String> to Option<&str>,
            // so we have to reborrow.
            config.consul.urls.get(0).map(|x| &**x)
        )
        .ok_or_else(||
            ErrorKind::CliError("Url is neither specified as CLI parameter nor in configuration file".to_string())
        )?;

    let consul = Consul::new(url.to_string());
    let catalog = consul.catalog()?;

    let mut writer = std::io::stdout();

    let mut services: Vec<_> = catalog.services()
        .iter()
        .map(|name| {
            let nodes = catalog.nodes_by_service(name).unwrap()
                .into_iter()
                .map(|node| {
                    let healthy = catalog.is_node_healthy_for_service(node, name);
                    let mut service_urls = generate_service_ulrs(&config, name, node);
                    let default_url = if let Some(ref mut s_urls) = service_urls {
                        s_urls.remove("default")
                    } else {
                        None
                    };
                    Node {
                        name: &node.name,
                        address: &node.address,
                        service_port: node.service_port,
                        service_tags: &node.service_tags,
                        healthy,
                        service_urls: service_urls,
                        default_url: default_url
                    }
                })
                .collect();
            let tags = catalog.service_tags(name).unwrap();
            Service { name, tags, nodes }
        })
        .collect();
    services.sort_by_key(|x| x.name);
    let context = Context { project_name: &config.general.project_name, services };

    render_template(config.present.templates.get("services").unwrap(), &mut writer, &context)
}

fn handlebars_vec_len_formatter(h: &Helper, _: &Handlebars, rc: &mut RenderContext) -> ::std::result::Result<(), RenderError> {
    let param = h.param(0).unwrap();
    let vec_len = if let Some(v) = param.value().as_array() {
        v.len()
    } else {
        0
    };

    let f = format!("{}", vec_len);
    rc.writer.write_all(f.as_bytes())?;

    Ok(())
}

#[derive(Serialize)]
struct Context<'a> {
    project_name: &'a str,
    services: Vec<Service<'a>>
}

#[derive(Serialize)]
struct Service<'a> {
    name: &'a str,
    tags: Vec<&'a String>,
    nodes: Vec<Node<'a>>,
}

#[derive(Serialize)]
struct Node<'a> {
    name: &'a str,
    address: &'a str,
    service_port: u16,
    service_tags: &'a Vec<String>,
    healthy: bool,
    service_urls: Option<HashMap<String, String>>,
    default_url: Option<String>,
}

fn generate_service_ulrs(config: &config::Config, service_name: &str, node: &service_world::Node) -> Option<HashMap<String, String>> {
    if let Some(services) = config.services.get(service_name) {
        let mut m = HashMap::new();
        let mut handlebars = Handlebars::new();

        for service in services {
            let template_name = format!("service_url-{}", service.name);
            handlebars.register_template_string(&template_name, &service.url).unwrap();
            let rendered_url = handlebars.render(&template_name, node).unwrap();
            m.insert(service.name.to_string(), rendered_url);
        }

        Some(m)
    } else {
        None
    }
}

fn render_template(template_file: &str, mut w: &mut Write, context: &Context) -> Result<()> {
    let mut handlebars = Handlebars::new();
    handlebars.register_helper("len", Box::new(handlebars_vec_len_formatter));

    handlebars.register_template_file("service_overview", template_file).unwrap();
    handlebars.renderw("service_overview", context, &mut w).unwrap();

    Ok(())
}

fn build_cli() -> App<'static, 'static> {
    let name = "sw-present";
    let version = env!("CARGO_PKG_VERSION");

    App::new(name)
        .version(version)
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .required(true)
                .takes_value(true)
                .help("Sets config file"),
        )
        .arg(
            Arg::with_name("completions")
                .long("completions")
                .takes_value(true)
                .hidden(true)
                .possible_values(&["bash", "fish", "zsh"])
                .help("The shell to generate the script for"),
        )
}

error_chain! {
    errors {
        CliError(cause: String) {
            description("Failed to run")
            display("Failed to run because {}", cause)
        }

        NoResults(for_what: String) {
            description("No results found")
            display("No results found for {}", for_what)
        }

        OutputError {
            description("Output failed")
            display("Output failed")
        }
    }

    links {
        ServiceWorld(service_world::Error, service_world::ErrorKind);
    }
}

quick_main!(run);
