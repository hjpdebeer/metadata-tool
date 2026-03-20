//! API route handlers organised by metadata domain.
//!
//! Each submodule contains Axum handler functions registered in `main.rs`.
//! All handlers use utoipa annotations for OpenAPI documentation and
//! follow ADR-0006 data access patterns for consistent read/write behaviour.

pub mod admin;
pub mod ai;
pub mod app_bulk_upload;
pub mod applications;
pub mod auth;
pub mod bulk_upload;
pub mod data_dictionary;
pub mod data_quality;
pub mod de_bulk_upload;
pub mod glossary;
pub mod health;
pub mod ingestion;
pub mod lineage;
pub mod notifications;
pub mod processes;
pub mod users;
pub mod workflow;
