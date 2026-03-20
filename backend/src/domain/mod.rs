//! Domain model types for all metadata entities.
//!
//! Each submodule defines Rust structs that map to PostgreSQL tables via
//! SQLx `FromRow`, generate OpenAPI schemas via utoipa `ToSchema`, and
//! serialize/deserialize via serde. Types are organised by metadata domain.

pub mod ai;
pub mod applications;
pub mod data_dictionary;
pub mod data_quality;
pub mod glossary;
pub mod lineage;
pub mod notifications;
pub mod processes;
pub mod users;
pub mod workflow;
