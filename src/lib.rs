extern crate consul;

use consul::Client;
use std::collections::HashMap;
use std::iter::FromIterator;

pub struct Node {
    pub name: String,
    pub address: String,
    pub service_port: u16,
    pub service_tags: Vec<String>,
}

pub struct Catalog {
    services: HashMap<String, Vec<String>>,
    nodes_by_service: HashMap<String, Vec<Node>>,
    healthy_nodes_by_service: HashMap<String, Vec<String>>,
}

impl Catalog {
    pub fn services(&self) -> Vec<&String> {
        self.services.keys().collect()
    }

    pub fn service_tags(&self, service_name: &str) -> Option<Vec<&String>> {
        self.services.get(service_name)
            .map(|x| x.iter().collect() )
    }

    pub fn nodes_by_service(&self, service_name: &str) -> Option<Vec<&Node>> {
        self.nodes_by_service.get(service_name)
            .map(|x| x.iter().collect() )
    }

    pub fn is_node_healthy_for_service(&self, node: &Node, service_name: &str) -> bool {
        self.healthy_nodes_by_service.get(service_name)
            .map_or(false, |x| x.contains(&node.address))
    }
}

pub struct Consul {
    url: String,
}

impl Consul {
    pub fn new(url: String) -> Self {
        Consul { url }
    }

    pub fn catalog(&self) -> Option<Catalog> {
        self.catalog_by(None, None)
    }

    pub fn catalog_by(&self, services: Option<Vec<String>>, tags: Option<Vec<String>>) -> Option<Catalog> {
        let client = Client::new(&self.url);

        let service_filter: Box<Fn(&String) -> bool> = if let Some(services) = services {
            Box::new(move |x| services.contains(&x))
        } else {
            Box::new(|_x| true)
        };

        let tag_filter: Box<Fn(&String) -> bool> = if let Some(tags) = tags {
            Box::new(move |x| tags.contains(&x))
        } else {
            Box::new(|_x| true)
        };

        let services: HashMap<String, Vec<String>> = client.catalog.services()
            .unwrap()
            .into_iter()
            .filter(|&(ref key, _)| service_filter(&key))
            .filter(|&(_, ref values)| values.iter().any(|x| tag_filter(x)))
            .collect();

        let nodes_by_service: HashMap<String, Vec<_>> = HashMap::from_iter(services
            .keys()
            .map(|service| {
                (
                    service.to_string(),
                    client.catalog.get_nodes(service.clone()).unwrap()
                        .into_iter()
                        .filter(|node| node.ServiceTags.iter().any(|x| tag_filter(x)))
                        .map(|node| Node {
                            name: node.Node,
                            address: node.Address,
                            service_port: node.ServicePort,
                            service_tags: node.ServiceTags,
                        })
                        .collect::<Vec<_>>()
                )
            }));

        let healthy_nodes_by_service: HashMap<String, Vec<_>> = HashMap::from_iter(services
            .keys()
            .map(|service| {
                (
                    service.to_string(),
                    client.health.healthy_nodes_by_service(service).unwrap()
                )
            }));

        Some(Catalog {
            services,
            nodes_by_service,
            healthy_nodes_by_service
        })
    }
}
