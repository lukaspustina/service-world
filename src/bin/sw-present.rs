#[macro_use]
extern crate error_chain;
extern crate clap;
extern crate handlebars;
extern crate serde;
extern crate service_world;

use clap::{App, Arg};
use service_world::config::Config;
use service_world::consul::Consul;
use service_world::present::Services;
use std::path::Path;

fn run() -> Result<()> {
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

    let services = Services::from_catalog(&catalog, &config);

    let mut writer = std::io::stdout();

    services
        .render(config.present.templates.get("services").unwrap(), &mut writer)
        .map_err(|e| e.into())
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
        Config(service_world::config::Error, service_world::config::ErrorKind);
        Consul(service_world::consul::Error, service_world::consul::ErrorKind);
        Present(service_world::present::Error, service_world::present::ErrorKind);
    }
}

quick_main!(run);
