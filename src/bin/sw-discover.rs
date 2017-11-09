extern crate ansi_term;
extern crate clap;
extern crate consul;
extern crate tabwriter;

use ansi_term::Color;
use clap::{App, Arg};
use consul::Client;
use tabwriter::TabWriter;
use std::collections::HashMap;
use std::io::Write;
use std::iter::FromIterator;

fn main() {
    let args = build_cli().get_matches();

    let url: &str = args.value_of("url").unwrap();
    let client = Client::new(url);

    let services: Vec<String> = args.values_of_lossy("services").unwrap_or_else(|| Vec::new());
    let service_filter: Box<Fn(&String) -> bool> = if services.is_empty() {
        Box::new(|_x| true)
    } else {
        Box::new(|x| services.contains(&x))
    };

    let tags: Vec<String> = args.values_of_lossy("tags").unwrap_or_else(|| Vec::new());
    let tag_filter: Box<Fn(&String) -> bool> = if tags.is_empty() {
        Box::new(|_x| true)
    } else {
        Box::new(|x| tags.contains(&x))
    };

    let services: HashMap<String, Vec<String>> = client.catalog.services()
        .unwrap()
        .into_iter()
        .filter(|&(ref key, _)| service_filter(&key))
        .filter(|&(_, ref values)| values.iter().any(|x| tag_filter(x)))
        .collect();

    let nodes_by_service: HashMap<&String, Vec<_>> = HashMap::from_iter(services
        .keys()
        .map(|service| {
            (service, client.catalog.get_nodes(service.clone()).unwrap()
                .into_iter()
                .filter(|node| node.ServiceTags.iter().any(|x| tag_filter(x)))
                .collect::<Vec<_>>()
            )
        }));

    let healthy_nodes_by_service: HashMap<&String, Vec<_>> = HashMap::from_iter(services
        .keys()
        .map(|service| {
            (service, client.health.healthy_nodes_by_service(service).unwrap())
        }));

    let mut tw = TabWriter::new(vec![]).padding(1);
    for (name, tags) in &services {
        let _ = writeln!(
            &mut tw,
            "Service '{}' tagged with {}",
            Color::Yellow.paint(format!("{}", name)),
            Color::Blue.paint(format!("{:?}", tags)),
        );

        for node in &nodes_by_service[name] {
            let (node_name, health_indicator) = if healthy_nodes_by_service[name].contains(&node.Address) {
                (Color::Green.paint(format!("{}", node.Node)), ":-)")
            } else {
                (Color::Red.paint(format!("{}", node.Node)), ":-(")
            };

            let _ = writeln!(
                &mut tw,
                "\t* Node '{}' {} \tip:{},\tport:{},\ttags:{}",
                node_name,
                health_indicator,
                node.Address,
                node.ServicePort,
                Color::Blue.paint(format!("{:?}", node.ServiceTags)),
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
