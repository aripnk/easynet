//! Pure EasyNet state. No OS or socket I/O.

mod config;

pub use config::{Config, default_toml, parse_toml};
