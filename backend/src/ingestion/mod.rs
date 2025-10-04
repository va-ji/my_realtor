//! Data ingestion module - functional pipeline for multi-source property data

pub mod enrich;
pub mod fetch;
pub mod parse;
pub mod types;
pub mod utils;
pub mod write;

pub use types::*;
