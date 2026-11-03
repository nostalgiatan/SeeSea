//! Library half of the `seesea` binary, split out so integration tests can reach the server and the adapters.

pub mod config;
pub mod embedding;
pub mod server;
pub mod sources;
pub mod models;
pub mod userconfig;
pub mod keyless;
pub mod access;
pub mod cluster;
pub mod wire;
pub mod tls;
pub mod account;
pub mod seeds;
pub mod desk;
pub mod ui;
