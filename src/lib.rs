#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate handlebars;
extern crate hyper;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate tokio_core;
extern crate toml;

pub mod config;
pub mod consul;
pub mod present;
