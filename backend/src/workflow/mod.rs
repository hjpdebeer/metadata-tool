// Workflow engine module - manages entity lifecycle states

pub mod service;

/// Supported workflow entity types
pub const ENTITY_GLOSSARY_TERM: &str = "GLOSSARY_TERM";
pub const ENTITY_DATA_ELEMENT: &str = "DATA_ELEMENT";
pub const ENTITY_QUALITY_RULE: &str = "QUALITY_RULE";
pub const ENTITY_APPLICATION: &str = "APPLICATION";
pub const ENTITY_BUSINESS_PROCESS: &str = "BUSINESS_PROCESS";

/// Workflow state codes
pub const STATE_DRAFT: &str = "DRAFT";
pub const STATE_PROPOSED: &str = "PROPOSED";
pub const STATE_UNDER_REVIEW: &str = "UNDER_REVIEW";
pub const STATE_REVISED: &str = "REVISED";
pub const STATE_ACCEPTED: &str = "ACCEPTED";
pub const STATE_REJECTED: &str = "REJECTED";
pub const STATE_DEPRECATED: &str = "DEPRECATED";

/// Workflow action codes
pub const ACTION_SUBMIT: &str = "SUBMIT";
pub const ACTION_APPROVE: &str = "APPROVE";
pub const ACTION_REJECT: &str = "REJECT";
pub const ACTION_REVISE: &str = "REVISE";
pub const ACTION_WITHDRAW: &str = "WITHDRAW";
pub const ACTION_DEPRECATE: &str = "DEPRECATE";
