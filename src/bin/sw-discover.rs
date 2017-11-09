extern crate ansi_term;
#[macro_use]
extern crate error_chain;
extern crate clap;
extern crate service_world;
extern crate serde_json;
extern crate tabwriter;

use ansi_term::Color;
use clap::{App, Arg};
use tabwriter::TabWriter;
use service_world::{Consul, Catalog};
use std::io::Write;

fn run() -> Result<()> {
    let args = build_cli().get_matches();

    let output = args.value_of("output module").ok_or_else(|| {
        ErrorKind::CliError("Output module not specified".to_string())
    })?;
    let url = args.value_of("url").ok_or_else(|| {
        ErrorKind::CliError("Url not specified".to_string())
    })?;
    let consul = Consul::new(url.to_string());
    let catalog = consul.catalog_by(
        args.values_of_lossy("services"),
        args.values_of_lossy("tags"),
    )?;

    let mut writer = std::io::stdout();
    match output {
        "json" => json_output(&mut writer, &catalog),
        _ => terminal_output(&mut writer, &catalog)
    }
}

fn build_cli() -> App<'static, 'static> {
    let name = "sw-discover";
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
            Arg::with_name("services")
                .value_name("service name")
                .long("service")
                .short("s")
                .takes_value(true)
                .multiple(true)
                .require_delimiter(true)
                .number_of_values(1)
                .help("Filters service for specified service names"),
        )
        .arg(
            Arg::with_name("tags")
                .value_name("tag name")
                .long("tag")
                .short("t")
                .takes_value(true)
                .multiple(true)
                .require_delimiter(true)
                .number_of_values(1)
                .help("Filters service for specified tags"),
        )
        .arg(
            Arg::with_name("output module")
                .long("output")
                .short("o")
                .takes_value(true)
                .default_value("terminal")
                .possible_values(
                    &[
                        "terminal",
                        "json",
                    ],
                )
                .help("Selects output module")
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

fn terminal_output(w: &mut Write, catalog: &Catalog) -> Result<()> {
    let mut tw = TabWriter::new(vec![]).padding(1);
    for service_name in catalog.services() {
        let _ =
            writeln!(
                &mut tw,
                "Service '{}' tagged with {}",
                Color::Yellow.paint(service_name.as_ref()),
                Color::Blue.paint(format!("{:?}", catalog.service_tags(service_name))),
            );

        for node in catalog.nodes_by_service(service_name).ok_or_else(|| {
            ErrorKind::NoResults(format!("nodes for service {}", service_name))
        })?
            {
                let (node_name, health_indicator) =
                    if catalog.is_node_healthy_for_service(node, service_name) {
                        (Color::Green.paint(node.name.as_ref()), ":-)")
                    } else {
                        (Color::Red.paint(node.name.as_ref()), ":-(")
                    };

                let _ =
                    writeln!(
                        &mut tw,
                        "\t* Node '{}' {} \tip:{},\tport:{},\ttags:{}",
                        node_name,
                        health_indicator,
                        node.address,
                        node.service_port,
                        Color::Blue.paint(format!("{:?}", node.service_tags)),
                    );
            }
        let _ = writeln!(&mut tw, "");
    }

    let out_str = String::from_utf8(tw.into_inner().chain_err(|| ErrorKind::OutputError)?)
        .chain_err(|| ErrorKind::OutputError)?;
    write!(w, "{}", out_str).chain_err(|| ErrorKind::OutputError)
}

fn json_output(mut w: &mut Write, catalog: &Catalog) -> Result<()> {
    serde_json::to_writer_pretty(&mut w, catalog).chain_err(|| ErrorKind::OutputError)
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
