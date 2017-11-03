extern crate consul;
extern crate tabwriter;

use consul::Client;
use tabwriter::TabWriter;
use std::io::Write;

fn main() {
    let url = ::std::env::args().nth(1).unwrap_or_else(|| "http:://localhost:8500".to_string());

    let client = Client::new(&url);
    let mut tw = TabWriter::new(vec![]).padding(1);

    let services = client.catalog.services().unwrap();
    let mut service_names: Vec<_> = services.keys().collect();
    service_names.sort();
    for service in service_names {
        let nodes = client.catalog.get_nodes(service.clone()).unwrap();
        let _ = writeln!(&mut tw, "Service '{}' tagged with {:?}", service, services.get(service).unwrap());
        for node in nodes {
            let _ = writeln!(&mut tw, "\t* Node '{}'\tip:{},\tservice: id:{},\tname:{},\tport:{},\ttags:{:?}", node.Node, node.Address, node.ServiceID, node.ServiceName, node.ServicePort, node.ServiceTags );
        }
        let _ = writeln!(&mut tw, "");
    }

    let out_str = String::from_utf8(tw.into_inner().unwrap()).unwrap();
    print!("{}", out_str);
}

