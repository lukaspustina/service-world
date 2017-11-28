// For rocket_codegen
#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate consul as consul_api;
#[macro_use]
extern crate error_chain;
extern crate handlebars;
extern crate rocket;
#[macro_use]
extern crate serde_derive;
extern crate toml;

pub mod config;
pub mod consul;
pub mod present;
