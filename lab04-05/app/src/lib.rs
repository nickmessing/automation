//! Exchange rates from the Frankfurter API: a CLI port of lab02 plus a small web UI.

pub mod api;
pub mod chart;
pub mod cli;
pub mod error;
pub mod stats;
pub mod store;
pub mod svg;
pub mod validate;
pub mod web;

pub use api::{Client, Rate};
pub use error::Error;
