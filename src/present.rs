use config::Config;
use consul::{self, Catalog};
use handlebars::Handlebars;
use std::collections::HashMap;
use std::io::Write;

#[derive(Serialize)]
pub struct Services<'a> {
    pub project_name: &'a str,
    pub services: Vec<Service<'a>>
}

#[derive(Serialize)]
pub struct Service<'a> {
    pub name: &'a str,
    pub tags: Vec<&'a String>,
    pub nodes: Vec<Node<'a>>,
}

#[derive(Serialize)]
pub struct Node<'a> {
    pub name: &'a str,
    pub address: &'a str,
    pub service_port: u16,
    pub service_tags: &'a Vec<String>,
    pub healthy: bool,
    pub service_urls: Option<HashMap<String, String>>,
    pub default_url: Option<String>,
}

impl<'a> Services<'a> {
    pub fn from_catalog(catalog: &'a Catalog, config: &'a Config) -> Services<'a> {
        let mut services: Vec<_> = catalog.services()
            .iter()
            .map(|name| {
                let nodes = catalog.nodes_by_service(name).unwrap()
                    .into_iter()
                    .map(|node| {
                        let healthy = catalog.is_node_healthy_for_service(node, name);
                        let mut service_urls = generate_service_ulrs(&config, name, node);
                        let default_url = if let Some(ref mut s_urls) = service_urls {
                            s_urls.remove("default")
                        } else {
                            None
                        };
                        Node {
                            name: &node.name,
                            address: &node.address,
                            service_port: node.service_port,
                            service_tags: &node.service_tags,
                            healthy,
                            service_urls,
                            default_url,
                        }
                    })
                    .collect();
                let tags = catalog.service_tags(name).unwrap();
                Service { name, tags, nodes }
            })
            .collect();
        services.sort_by_key(|x| x.name);

        Services { project_name: &config.general.project_name, services }
    }


    pub fn render(&self, template_file: &str, mut w: &mut Write) -> Result<()> {
        let mut handlebars = Handlebars::new();
        handlebars.register_helper("len", Box::new(handlebars_helper::vec_len_formatter));

        let template_name = "service_overview";
        handlebars.register_template_file(template_name, template_file)
            .chain_err(|| ErrorKind::TemplateError(template_name.to_string()))?;
        handlebars.renderw("service_overview",self, &mut w)
            .chain_err(|| ErrorKind::TemplateError(template_name.to_string()))?;

        Ok(())
    }
}

mod handlebars_helper {
    use handlebars::{Handlebars, Helper, RenderContext, RenderError};

    pub fn vec_len_formatter(h: &Helper, _: &Handlebars, rc: &mut RenderContext) -> ::std::result::Result<(), RenderError> {
        let param = h.param(0).unwrap();
        let vec_len = if let Some(v) = param.value().as_array() {
            v.len()
        } else {
            0
        };

        let f = format!("{}", vec_len);
        rc.writer.write_all(f.as_bytes())?;

        Ok(())
    }
}

fn generate_service_ulrs(config: &Config, service_name: &str, node: &consul::Node) -> Option<HashMap<String, String>> {
    if let Some(services) = config.services.get(service_name) {
        let mut m = HashMap::new();
        let mut handlebars = Handlebars::new();

        for service in services {
            let template_name = format!("service_url-{}", service.name);
            handlebars.register_template_string(&template_name, &service.url).unwrap();
            let rendered_url = handlebars.render(&template_name, node).unwrap();
            m.insert(service.name.to_string(), rendered_url);
        }

        Some(m)
    } else {
        None
    }
}

error_chain! {
    errors {
        TemplateError(name: String) {
            description("Failed to render template")
            display("Failed to render template '{}'", name)
        }
    }
}
