#[macro_use]
extern crate error_chain;
extern crate clap;
extern crate handlebars;
#[macro_use]
extern crate serde_derive;
extern crate service_world;

use clap::{App, Arg};
use handlebars::Handlebars;
use service_world::{Consul, Catalog};
use std::io::Write;

fn run() -> Result<()> {
    let args = build_cli().get_matches();

    let url = args.value_of("url").ok_or_else(|| {
        ErrorKind::CliError("Url not specified".to_string())
    })?;
    let template_file = args.value_of("template").ok_or_else(|| {
        ErrorKind::CliError("Template not specified".to_string())
    })?;

    let consul = Consul::new(url.to_string());
    let catalog = consul.catalog()?;

    let mut writer = std::io::stdout();
    let context = Context { project_name: "Service World", catalog: &catalog };
    render_template(template_file,&mut writer, &context)
}


#[derive(Serialize)]
struct Context<'a> {
    project_name: &'a str,
    catalog: &'a Catalog,
}

fn render_template(template_file: &str, mut w: &mut Write, context: &Context) -> Result<()> {
    let mut handlebars = Handlebars::new();

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
            Arg::with_name("url")
                .index(1)
                .required(true)
                .conflicts_with("completions")
                .help("URL of consul agent to retrieve catalog from"),
        )
        .arg(
            Arg::with_name("template")
                .long("template")
                .takes_value(true)
                .default_value("templates/default.hbs")
                .help("Sets template file for output"),
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
