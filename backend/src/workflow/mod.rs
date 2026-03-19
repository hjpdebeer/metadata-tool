//! Workflow engine for metadata entity lifecycle management (Principle 5).
//!
//! Implements a generic state machine: Draft -> Proposed -> Under Review ->
//! Accepted/Revised/Rejected -> Deprecated. Supports all entity types via
//! configurable workflow definitions, transitions, tasks, and approvers.
//! State transitions are recorded in `workflow_history` for audit (Principle 9).

pub mod service;

/// Entity type code for glossary terms in workflow operations.
pub const ENTITY_GLOSSARY_TERM: &str = "GLOSSARY_TERM";

/// Entity type code for data elements in workflow operations.
pub const ENTITY_DATA_ELEMENT: &str = "DATA_ELEMENT";

/// Entity type code for quality rules in workflow operations.
pub const ENTITY_QUALITY_RULE: &str = "QUALITY_RULE";

/// Entity type code for applications in workflow operations.
pub const ENTITY_APPLICATION: &str = "APPLICATION";

/// Entity type code for business processes in workflow operations.
pub const ENTITY_BUSINESS_PROCESS: &str = "BUSINESS_PROCESS";

/// Initial state for newly created entities before submission.
pub const STATE_DRAFT: &str = "DRAFT";

/// State indicating an entity has been submitted for review.
pub const STATE_PROPOSED: &str = "PROPOSED";

/// State indicating an entity is actively being reviewed by approvers.
pub const STATE_UNDER_REVIEW: &str = "UNDER_REVIEW";

/// State indicating an entity has been sent back for revision.
pub const STATE_REVISED: &str = "REVISED";

/// Terminal state indicating an entity has been approved and accepted.
pub const STATE_ACCEPTED: &str = "ACCEPTED";

/// Terminal state indicating an entity has been rejected.
pub const STATE_REJECTED: &str = "REJECTED";

/// Terminal state indicating an entity has been retired from active use.
pub const STATE_DEPRECATED: &str = "DEPRECATED";

/// Action to submit an entity for review (DRAFT -> PROPOSED).
pub const ACTION_SUBMIT: &str = "SUBMIT";

/// Action to approve an entity (UNDER_REVIEW -> ACCEPTED).
pub const ACTION_APPROVE: &str = "APPROVE";

/// Action to reject an entity (UNDER_REVIEW -> REJECTED).
pub const ACTION_REJECT: &str = "REJECT";

/// Action to request revision (UNDER_REVIEW -> REVISED).
pub const ACTION_REVISE: &str = "REVISE";

/// Action to withdraw a submission (PROPOSED -> DRAFT).
pub const ACTION_WITHDRAW: &str = "WITHDRAW";

/// Action to deprecate an accepted entity (ACCEPTED -> DEPRECATED).
pub const ACTION_DEPRECATE: &str = "DEPRECATE";
