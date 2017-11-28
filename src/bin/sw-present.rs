#![feature(plugin)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate error_chain;
extern crate clap;
extern crate handlebars;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate service_world;

use clap::{App, Arg};
use handlebars::Handlebars;
use rocket::{Request, State};
use rocket::response::content;
use rocket_contrib::Template;
use service_world::config::Config;
use service_world::consul::Consul;
use service_world::present::Services;
use std::io::{BufWriter, Write};
use std::path::Path;

fn run() -> Result<()> {
    let args = build_cli().get_matches();

    let config = if let Some(config_file) = args.value_of("config") {
        Config::from_file(Path::new(config_file))
    } else {
        Ok(Default::default())
    }.unwrap();

    // TODO: Consul Client should take all URLs and decides which to use by itself.
    let url: String = {
        let s = args.value_of("url")
            .or_else(||
                // This is Rust at its not so finest: There's no coercing from Option<&String> to Option<&str>,
                // so we have to reborrow.
                config.consul.urls.get(0).map(|x| &**x)
            )
            .ok_or_else(||
                ErrorKind::CliError("Url is neither specified as CLI parameter nor in configuration file".to_string())
            )?;
        s.to_string()
    };
    let consul = Consul::new(url);

    if args.is_present("rocket") {
        launch_rocket(config, consul)
    } else {
        let mut writer = std::io::stdout();
        gen_services_html(&config, &consul, &mut writer)
    }
}

#[get("/")]
fn index(config: State<Config>) -> Result<content::Html<String>> {
    let mut buffer = vec![];
    gen_index_html(&config,&mut buffer);

    String::from_utf8(buffer)
        .map(content::Html)
        .map_err(|_| Error::from(ErrorKind::OutputError))
}

#[get("/services")]
fn services(config: State<Config>, consul: State<Consul>) -> Result<content::Html<String>> {
    let catalog = consul.catalog().unwrap();
    let services = Services::from_catalog(&catalog, &config).unwrap();

    let mut buffer = vec![];
    gen_services_html(&config, &consul, &mut buffer);

    String::from_utf8(buffer)
        .map(content::Html)
        .map_err(|_| Error::from(ErrorKind::OutputError))
}

fn launch_rocket(config: Config, consul: Consul) -> Result<()> {

    let mut rocket = rocket::ignite()
        .catch(errors![not_found])
        .mount("/", routes![index, services])
        .attach(Template::fairing())
        .manage(config)
        .manage(consul);

    rocket.launch();

    Ok(())
}

#[error(404)]
fn not_found(_: &Request) -> String {
    "not implemented".to_string()
}

fn gen_index_html(config: &Config, w: &mut Write) -> Result<()> {
    let template_filename = config.present.templates.get("index").ok_or_else(|| {
        ErrorKind::CliError("Index template file not specified".to_string())
    })?;
    // TODO: Let me be a path
    let template_file = format!("{}/{}", &config.present.template_dir, template_filename);

    let mut handlebars = Handlebars::new();
    let template_name = "index";
    handlebars
        .register_template_file(template_name, template_file)
        .chain_err(|| ErrorKind::OutputError)?;
    handlebars
        .renderw("index", config, w)
        .chain_err(|| ErrorKind::OutputError)?;

    Ok(())
}

fn gen_services_html(config: &Config, consul: &Consul, w: &mut Write) -> Result<()> {
    let template_filename = config.present.templates.get("services").ok_or_else(|| {
        ErrorKind::CliError("Services template file not specified".to_string())
    })?;
    // TODO: Let me be a path
    let template_file = format!("{}/{}", &config.present.template_dir, template_filename);

    let catalog = consul.catalog()?;
    let services = Services::from_catalog(&catalog, &config)?;

    services.render(&template_file, w).map_err(
        |e| e.into(),
    )
}

fn build_cli() -> App<'static, 'static> {
    let name = "sw-present";
    let version = env!("CARGO_PKG_VERSION");

    App::new(name)
        .version(version)
        .arg(
            Arg::with_name("url")
                .index(1)
                .conflicts_with("completions")
                .help("URL of consul agent to retrieve catalog from"),
        )
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .required(true)
                .takes_value(true)
                .conflicts_with("completions")
                .help("Sets config file"),
        )
        .arg(
            Arg::with_name("rocket")
                .short("r")
                .long("rocket")
                .conflicts_with("completions")
                .help("Sets Rocket mode -- activates internal web server"),
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
