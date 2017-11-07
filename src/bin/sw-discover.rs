extern crate clap;
extern crate consul;
extern crate tabwriter;

use clap::{App, Arg};
use consul::Client;
use tabwriter::TabWriter;
use std::collections::HashSet;
use std::io::Write;

fn main() {
    let args = build_cli().get_matches();

    let url: &str = args.value_of("url").unwrap();
    let service_filter: Option<Vec<_>> = args.values_of_lossy("services");
    let tag_filter: Option<Vec<_>> = args.values_of_lossy("tags");

    let client = Client::new(url);
    let mut tw = TabWriter::new(vec![]).padding(1);

    let services = client.catalog.services().unwrap();
    let mut service_names: Vec<_> = services.keys().collect();
    service_names.sort();
    for service in service_names {
        if let Some(ref s_filter) = service_filter {
            if !s_filter.contains(service) {
                continue;
            }
        }
        let nodes = client.catalog.get_nodes(service.clone()).unwrap();
        let healthy_nodes = client.health.healthy_nodes_by_service(&service).unwrap();
        let _ = writeln!(
            &mut tw,
            "Service '{}' tagged with {:?}",
            service,
            &services[service]
        );
        for node in nodes {
            if let Some(ref s_tags) = tag_filter {
                let set: HashSet<_> = s_tags.iter().chain(node.ServiceTags.iter()).collect();
                if set.len() == s_tags.len() + node.ServiceTags.len() {
                    continue;
                }
            }
            let health_indicator = if healthy_nodes.contains(&node.Address) {
                ":-)"
            } else {
                ":-("
            };
            let _ = writeln!(
                &mut tw,
                "\t* Node '{}' {} \tip:{},\tservice: id:{},\tname:{},\tport:{},\ttags:{:?}",
                node.Node,
                health_indicator,
                node.Address,
                node.ServiceID,
                node.ServiceName,
                node.ServicePort,
                node.ServiceTags
            );
        }
        let _ = writeln!(&mut tw, "");
    }

    let out_str = String::from_utf8(tw.into_inner().unwrap()).unwrap();
    print!("{}", out_str);

    /*
    let services = client.catalog.services().unwrap();
    let service_names: Vec<_> = services.keys().collect();
    for s in service_names {
        let nodes = client.health.healthy_nodes_by_service(&s);
        for n in nodes {
            println!("{}: node={:?}", s, n);
        }
    }
    */
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
