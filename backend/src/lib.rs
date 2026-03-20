//! # Metadata Tool Backend
//!
//! Enterprise metadata lifecycle management for data governance in financial institutions.
//! Provides REST API, domain models, workflow engine, AI enrichment, and notification
//! services for managing business glossary terms, data dictionaries, data quality rules,
//! data lineage, business applications, and business processes.
//!
//! ## Architecture (ADR-0001: Modular Monolith)
//!
//! - [`api`] — Axum HTTP route handlers with OpenAPI annotations
//! - [`domain`] — Rust structs for database entities, request/response DTOs
//! - [`auth`] — JWT authentication and RBAC middleware
//! - [`workflow`] — Generic state machine for entity lifecycle (Principle 5)
//! - [`ai`] — Claude/OpenAI integration for metadata enrichment (Principle 6)
//! - [`notifications`] — Email queue and in-app notifications
//! - [`naming`] — Technical metadata naming standards validation (Principle 8)
//! - [`db`] — PostgreSQL connection pool and shared application state
//!
//! ## Key Principles
//!
//! All code must comply with the 14 foundational principles in
//! `METADATA_TOOL_PRINCIPLES.md` and coding standards in `CODING_STANDARDS.md`.

pub mod ai;
pub mod api;
pub mod auth;
pub mod config;
pub mod db;
pub mod domain;
pub mod error;
pub mod naming;
pub mod notifications;
pub mod settings;
pub mod workflow;
