use config::Config;
use consul::{self, Consul, Catalog};
use handlebars::Handlebars;
use std::collections::HashMap;
use std::io::Write;

#[derive(Serialize)]
pub struct Services<'a> {
    pub project_name: &'a str,
    pub services: Vec<Service<'a>>,
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
    pub fn from_catalog(catalog: &'a Catalog, config: &'a Config) -> Result<Services<'a>> {
        let mut services: Vec<_> = catalog
            .services()
            .iter()
            .map(|name| {
                let nodes = if let Some(nodes) = catalog.nodes_by_service(name) {
                    nodes
                        .into_iter()
                        .map(|node| {
                            let healthy = catalog.is_node_healthy_for_service(node, name);
                            let mut service_urls = generate_service_ulrs(config, name, node).ok();
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
                        .collect()
                } else {
                    Vec::new()
                };
                let tags = catalog.service_tags(name).unwrap_or_else(Vec::new);
                Service { name, tags, nodes }
            })
            .collect();
        services.sort_by_key(|x| x.name);

        Ok(Services {
            project_name: &config.general.project_name,
            services,
        })
    }

    pub fn render(&self, template_file: &str, mut w: &mut dyn Write) -> Result<()> {
        let mut handlebars = Handlebars::new();
        handlebars.register_helper("len", Box::new(handlebars_helper::vec_len_formatter));

        let template_name = "service_overview";
        handlebars
            .register_template_file(template_name, template_file)
            .chain_err(|| ErrorKind::TemplateError(template_name.to_string()))?;
        handlebars
            .render_template_to_write("service_overview", self, &mut w)
            .chain_err(|| ErrorKind::TemplateError(template_name.to_string()))?;

        Ok(())
    }
}

mod handlebars_helper {
    use handlebars::{Context, Handlebars, Helper, HelperResult, RenderContext, Output};

    pub fn vec_len_formatter(h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
        let vec_len = if let Some(param) = h.param(0) {
            if let Some(v) = param.value().as_array() {
                v.len()
            } else {
                0
            }
        } else {
            0
        };

        let f = format!("{}", vec_len);
        out.write(&f)?;

        Ok(())
    }
}

pub fn gen_index_html(config: &Config, w: &mut dyn Write) -> Result<()> {
    let template_name = "index";

    let template_filename = config.present.templates.get(template_name).ok_or_else(|| {
        ErrorKind::TemplateNotSet(template_name.to_string())
    })?;
    // TODO: Let me be a path
    let template_file = format!("{}/{}", &config.present.template_dir, template_filename);

    let mut handlebars = Handlebars::new();
    handlebars
        .register_template_file(template_name, template_file)
        .chain_err(|| ErrorKind::TemplateError(template_name.to_string()))?;
    handlebars.render_template_to_write("index", config, w).chain_err(|| {
        ErrorKind::TemplateError(template_name.to_string())
    })?;

    Ok(())
}

pub fn gen_services_html(config: &Config, consul: &Consul, w: &mut dyn Write) -> Result<()> {
    let template_name = "services";

    let template_filename = config.present.templates.get(template_name).ok_or_else(|| {
        ErrorKind::TemplateNotSet(template_name.to_string())
    })?;
    // TODO: Let me be a path
    let template_file = format!("{}/{}", &config.present.template_dir, template_filename);

    let catalog = consul.catalog().chain_err(|| {
        ErrorKind::TemplateError(template_name.to_string())
    })?;
    let services = Services::from_catalog(&catalog, config)?;

    services.render(&template_file, w)
}

fn generate_service_ulrs(
    config: &Config,
    service_name: &str,
    node: &consul::Node,
) -> Result<HashMap<String, String>> {
    let mut m = HashMap::new();
    if let Some(services) = config.services.get(service_name) {
        let mut handlebars = Handlebars::new();

        for service in services {
            let template_name = format!("service_url-{}", service.name);
            handlebars
                .register_template_string(&template_name, &service.url)
                .chain_err(|| ErrorKind::TemplateError(template_name.to_string()))?;
            let rendered_url = handlebars.render(&template_name, node).chain_err(|| {
                ErrorKind::TemplateError(template_name.to_string())
            })?;
            m.insert(service.name.to_string(), rendered_url);
        }
    }
    Ok(m)
}

error_chain! {
    errors {
        TemplateNotSet(name: String) {
            description("Template not set")
            display("Template not set '{}'", name)
        }

        TemplateError(name: String) {
            description("Failed to render template")
            display("Failed to render template '{}'", name)
        }
    }
}
