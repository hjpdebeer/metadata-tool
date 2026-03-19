//! Technical metadata naming standards validation (Principle 8).
//!
//! Validates column, table, and schema names against configurable regex patterns
//! loaded from the `naming_standards` database table. Returns compliance status
//! and violation details for each name checked.

use regex::Regex;
use serde::{Deserialize, Serialize};

/// A naming standard rule loaded from the `naming_standards` table, with a regex pattern to match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamingStandard {
    pub name: String,
    pub applies_to: String,
    pub pattern: String,
    pub description: String,
    pub example_valid: String,
    pub example_invalid: String,
    pub is_mandatory: bool,
}

/// Result of validating a name against all applicable naming standards.
#[derive(Debug, Serialize)]
pub struct NamingValidationResult {
    pub is_compliant: bool,
    pub violations: Vec<NamingViolation>,
}

/// A single naming standard violation with the standard name and a descriptive message.
#[derive(Debug, Serialize)]
pub struct NamingViolation {
    pub standard_name: String,
    pub message: String,
    pub suggestion: Option<String>,
}

/// Validate a name against the appropriate naming standards
pub fn validate_name(name: &str, entity_type: &str, standards: &[NamingStandard]) -> NamingValidationResult {
    let mut violations = Vec::new();

    for standard in standards {
        if standard.applies_to != entity_type || !standard.is_mandatory {
            continue;
        }
        if let Ok(re) = Regex::new(&standard.pattern)
            && !re.is_match(name)
        {
            violations.push(NamingViolation {
                standard_name: standard.name.clone(),
                message: format!(
                    "'{name}' does not match standard '{}': {}",
                    standard.name, standard.description
                ),
                suggestion: None,
            });
        }
    }

    NamingValidationResult {
        is_compliant: violations.is_empty(),
        violations,
    }
}
