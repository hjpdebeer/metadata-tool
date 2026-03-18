// Naming standards enforcement module
// Validates technical metadata names against configurable patterns

use regex::Regex;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize)]
pub struct NamingValidationResult {
    pub is_compliant: bool,
    pub violations: Vec<NamingViolation>,
}

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
        if let Ok(re) = Regex::new(&standard.pattern) {
            if !re.is_match(name) {
                violations.push(NamingViolation {
                    standard_name: standard.name.clone(),
                    message: format!(
                        "'{}' does not match standard '{}': {}",
                        name, standard.name, standard.description
                    ),
                    suggestion: None,
                });
            }
        }
    }

    NamingValidationResult {
        is_compliant: violations.is_empty(),
        violations,
    }
}
