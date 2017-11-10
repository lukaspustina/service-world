extern crate consul as consul_api;
#[macro_use]
extern crate error_chain;
extern crate handlebars;
#[macro_use]
extern crate serde_derive;
extern crate toml;

pub mod config;
pub mod consul;
pub mod present;
