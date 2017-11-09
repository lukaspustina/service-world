extern crate ansi_term;
extern crate clap;
extern crate service_world;
extern crate tabwriter;

use ansi_term::Color;
use clap::{App, Arg};
use tabwriter::TabWriter;
use std::io::Write;

fn main() {
    let args = build_cli().get_matches();

    let url: &str = args.value_of("url").unwrap();
    let consul = service_world::Consul::new(url.to_string());
    let catalog = consul.catalog_by(
        args.values_of_lossy("services").into(),
        args.values_of_lossy("tags"),
    ).unwrap();

    let mut tw = TabWriter::new(vec![]).padding(1);
    for service_name in catalog.services() {
        let _ = writeln!(
            &mut tw,
            "Service '{}' tagged with {}",
            Color::Yellow.paint(format!("{}", service_name)),
            Color::Blue.paint(format!("{:?}", catalog.service_tags(service_name))),
        );

        for node in catalog.nodes_by_service(service_name).unwrap() {
            let (node_name, health_indicator) = if catalog.is_node_healthy_for_service(node, service_name) {
                (Color::Green.paint(format!("{}", node.name)), ":-)")
            } else {
                (Color::Red.paint(format!("{}", node.name)), ":-(")
            };

            let _ = writeln!(
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

    let out_str = String::from_utf8(tw.into_inner().unwrap()).unwrap();
    print!("{}", out_str);
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
            Arg::with_name("completions")
                .long("completions")
                .takes_value(true)
                .hidden(true)
                .possible_values(&["bash", "fish", "zsh"])
                .help("The shell to generate the script for"),
        )
}
