//! API route handlers organised by metadata domain.
//!
//! Each submodule contains Axum handler functions registered in `main.rs`.
//! All handlers use utoipa annotations for OpenAPI documentation and
//! follow ADR-0006 data access patterns for consistent read/write behaviour.

pub mod admin;
pub mod glossary;
pub mod bulk_upload;
pub mod data_dictionary;
pub mod data_quality;
pub mod lineage;
pub mod applications;
pub mod notifications;
pub mod processes;
pub mod workflow;
pub mod users;
pub mod auth;
pub mod ai;
pub mod health;
