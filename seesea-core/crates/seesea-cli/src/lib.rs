//! Library half of the `seesea` binary, split out so integration tests can reach the server and the adapters.

pub mod access;
pub mod account;
pub mod cluster;
pub mod config;
pub mod desk;
pub mod deskx;
pub mod embedding;
pub mod keyless;
pub mod models;
pub mod reqlog;
pub mod seeds;
pub mod server;
pub mod sources;
pub mod tls;
pub mod ui;
pub mod userconfig;
pub mod wire;
