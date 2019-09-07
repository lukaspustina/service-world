use futures::{future, Future, Stream};
use hyper::{Client as HyperClient, Uri};
use serde::de::DeserializeOwned;
use serde_json;
use std::collections::HashMap;
use std::str;
use tokio_core::reactor::Core;

trait Client {
    fn new(urls: Vec<String>) -> Result<Self>
    where
        Self: ::std::marker::Sized;
    fn services(&mut self) -> Result<HashMap<String, Vec<String>>>;
    fn nodes(&mut self, services: &[&str]) -> Result<HashMap<String, Vec<Node>>>;
    fn healthy_nodes(&mut self, services: &[&str]) -> Result<HashMap<String, Vec<Health>>>;
}

#[derive(Debug)]
pub struct SyncClient {
    urls: Vec<String>,
    core: Core,
}

impl Client for SyncClient {
    fn new(urls: Vec<String>) -> Result<SyncClient> {
        let core = Core::new().chain_err(|| ErrorKind::TokioError)?;

        Ok(SyncClient { urls, core })
    }

    fn services(&mut self) -> Result<HashMap<String, Vec<String>>> {
        let uri_str = format!("{}/v1/catalog/services", self.urls[0]);
        let uri: Uri = uri_str.parse().chain_err(|| {
            ErrorKind::ConsulError("could not parse url".to_string())
        })?;
        let hyper = HyperClient::new(&self.core.handle());
        let call = hyper.get(uri).and_then(|res| res.body().concat2()).map(
            |body| {
                let json = str::from_utf8(&body).chain_err(|| {
                    ErrorKind::ConsulError("Failed to read JSON".to_string())
                })?;
                let ss: HashMap<String, Vec<String>> = serde_json::from_str(json).chain_err(|| {
                    ErrorKind::ConsulError("Failed to deserialize JSON".to_string())
                })?;
                Ok(ss)
            },
        );

        self.core.run(call).chain_err(|| {
            ErrorKind::ConsulError("failed to get services".to_string())
        })?
    }

    fn nodes(&mut self, services: &[&str]) -> Result<HashMap<String, Vec<Node>>> {
        let base_uri = format!("{}/v1/catalog/service/@@", self.urls[0]);
        consul_calls_by_services(&mut self.core, &base_uri, services)
    }

    fn healthy_nodes(&mut self, services: &[&str]) -> Result<HashMap<String, Vec<Health>>> {
        // @@ is a place holder used in `consul_calls_by_services` to insert the service name into
        // this url
        let base_uri = format!("{}/v1/health/service/@@?passing", self.urls[0]);
        consul_calls_by_services(&mut self.core, &base_uri, services)
    }
}

fn consul_calls_by_services<T: DeserializeOwned>(
    core: &mut Core,
    uri_base: &str,
    services: &[&str],
) -> Result<HashMap<String, Vec<T>>> {
    let service_calls: Result<Vec<_>> = services
        .iter()
        .map(|service| {
            let uri_str = uri_base.replace("@@", service);
            let uri: Uri = uri_str.parse().chain_err(|| {
                ErrorKind::ConsulError("could not parse url".to_string())
            })?;
            let hyper = HyperClient::new(&core.handle());
            let service_name = service.to_string();
            let call = hyper.get(uri).and_then(|res| res.body().concat2()).map(
                |body| {
                    let json = str::from_utf8(&body).chain_err(|| {
                        ErrorKind::ConsulError(format!(
                            "Failed to read JSON for service '{}'",
                            service_name
                        ))
                    })?;
                    let ss: Vec<T> = serde_json::from_str(json).chain_err(|| {
                        ErrorKind::ConsulError(format!(
                            "Failed to deserialize JSON for service '{}'",
                            service_name
                        ))
                    })?;
                    Ok((service_name, ss))
                },
            );
            Ok(call)
        })
        .collect();

    service_calls
        .map(|calls| {
            let joined = future::join_all(calls)
                .and_then(move |answers: Vec<Result<(String, Vec<_>)>>| {
                    // Results are Ok((String, Vec<Node>);
                    // Errs are not arrive here due to the semantics of join_all.
                    let result: HashMap<String, Vec<T>> = answers
                    .into_iter()
                    .map(|x| x.unwrap()) // Safe
                    .collect();

                    future::ok(result)
                })
                .map_err(move |e| Error::with_chain(e, ErrorKind::TokioError));

            core.run(joined)
        })
        .chain_err(|| {
            ErrorKind::ConsulError("failed to get nodes for services".to_string())
        })?
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Node {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Node")]
    pub name: String,
    #[serde(rename = "NodeMeta")]
    pub meta_data: HashMap<String, String>,
    #[serde(rename = "Address")]
    pub address: String,
    #[serde(rename = "ServicePort")]
    pub service_port: u16,
    #[serde(rename = "ServiceTags")]
    pub service_tags: Vec<String>,
    #[serde(rename = "ServiceID")]
    pub service_id: String,
    #[serde(rename = "ServiceName")]
    pub service_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Health {
    #[serde(rename = "Node")]
    pub node: HealthyNode,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HealthyNode {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Node")]
    pub name: String,
    #[serde(rename = "Address")]
    pub address: String,
}

#[derive(Debug, Serialize)]
pub struct Catalog {
    pub services: HashMap<String, Vec<String>>,
    nodes_by_service: HashMap<String, Vec<Node>>,
    healthy_nodes_by_service: HashMap<String, Vec<Health>>,
}

impl Catalog {
    pub fn services(&self) -> Vec<&String> {
        self.services.keys().collect()
    }

    pub fn service_tags(&self, service_name: &str) -> Option<Vec<&String>> {
        self.services.get(service_name).map(|x| x.iter().collect())
    }

    pub fn nodes_by_service(&self, service_name: &str) -> Option<Vec<&Node>> {
        self.nodes_by_service.get(service_name).map(|x| {
            x.iter().collect()
        })
    }

    pub fn is_node_healthy_for_service(&self, node: &Node, service_name: &str) -> bool {
        self.healthy_nodes_by_service.get(service_name).map_or(
            false,
            |xs| {
                // TODO: Fix me -- that is uncessary
                let ids: Vec<_> = xs.iter().map(|x| x.node.id.clone()).collect();
                ids.contains(&node.id)
            },
        )
    }
}

pub struct Consul {
    url: String,
}

impl Consul {
    pub fn new(url: String) -> Self {
        Consul { url }
    }

    pub fn catalog(&self) -> Result<Catalog> {
        self.catalog_by(None, None)
    }

    pub fn catalog_by(
        &self,
        services: Option<Vec<String>>,
        tags: Option<Vec<String>>,
    ) -> Result<Catalog> {
        let mut client = SyncClient::new(vec![self.url.clone()])?;

        let service_filter: Box<dyn Fn(&String) -> bool> = if let Some(services) = services {
            Box::new(move |x| services.contains(x))
        } else {
            Box::new(|_x| true)
        };

        let tag_filter: Box<dyn Fn(&String) -> bool> = if let Some(tags) = tags {
            Box::new(move |x| tags.contains(x))
        } else {
            Box::new(|_x| true)
        };

        let services: HashMap<String, Vec<String>> = client.services().map(|h| {
            h.into_iter()
                .filter(|&(ref key, _)| service_filter(key))
                .filter(|&(_, ref values)| values.iter().any(|x| tag_filter(x)))
                .collect()
        })?;

        let nodes_by_service: HashMap<String, Vec<_>> = {
            let service_names: Vec<_> = services.keys().map(|s| &**s).collect();
            client.nodes(&service_names).map(|h| {
                h.into_iter()
                    .map(|(service, values)| {
                        let v = values
                            .into_iter()
                            .filter(|node| node.service_tags.iter().any(|x| tag_filter(x)))
                            .collect::<Vec<_>>();
                        (service, v)
                    })
                    .collect()
            })?
        };

        let healthy_nodes_by_service: HashMap<String, Vec<_>> = {
            let service_names: Vec<_> = services.keys().map(|s| &**s).collect();
            client.healthy_nodes(&service_names)?
        };

        Ok(Catalog {
            services,
            nodes_by_service,
            healthy_nodes_by_service,
        })
    }
}

error_chain! {
    errors {
        TokioError {
            description("Failed to use Tokio")
            display("Failed to use Tokio")
        }

        ConsulError(cause: String) {
            description("Failed get data from Consul")
            display("Failed get data from Consul because {}", cause)
        }
    }
}
