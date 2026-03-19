// AI integration module - Claude (primary) and OpenAI (fallback)
// Provides metadata enrichment suggestions based on financial services standards

use serde::{Deserialize, Serialize};

use crate::config::AiConfig;
use crate::error::AppError;

// ---------------------------------------------------------------------------
// AI response parsing types (internal)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContentBlock>,
    model: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeContentBlock {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
    model: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    content: Option<String>,
}

// ---------------------------------------------------------------------------
// Suggestion type returned by the AI enrichment service
// ---------------------------------------------------------------------------

fn deserialize_string_or_null<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

fn deserialize_f64_or_null<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<f64> = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or(0.0))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawAiSuggestion {
    #[serde(default, deserialize_with = "deserialize_string_or_null")]
    pub field_name: String,
    #[serde(default, deserialize_with = "deserialize_string_or_null")]
    pub suggested_value: String,
    #[serde(default, deserialize_with = "deserialize_f64_or_null")]
    pub confidence: f64,
    #[serde(default, deserialize_with = "deserialize_string_or_null")]
    pub rationale: String,
}

// ---------------------------------------------------------------------------
// Enrichment result
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct EnrichmentResult {
    pub suggestions: Vec<RawAiSuggestion>,
    pub provider: String,
    pub model: String,
}

// ---------------------------------------------------------------------------
// Prompt builder
// ---------------------------------------------------------------------------

fn build_prompt(
    entity_type: &str,
    entity_data: &serde_json::Value,
    existing_fields: &[String],
) -> String {
    let existing_list = if existing_fields.is_empty() {
        "None specified".to_string()
    } else {
        existing_fields.join(", ")
    };

    match entity_type {
        "glossary_term" => build_glossary_term_prompt(entity_data, &existing_list),
        _ => build_generic_prompt(entity_type, entity_data, &existing_list),
    }
}

/// Specialised prompt for glossary terms covering all 22 AI-suggestible fields
fn build_glossary_term_prompt(
    entity_data: &serde_json::Value,
    existing_list: &str,
) -> String {
    format!(
        r#"You are a metadata governance expert for financial institutions. Given a business glossary term, suggest values for empty metadata fields based on industry standards (DAMA DMBOK, BCBS 239, ISO 8000).

Entity: glossary_term
Current field values:
{entity_json}

Fields that already have values: {existing_list}

Suggest values for ONLY these text fields (never suggest ID fields, FK fields, or ownership fields):
- abbreviation: common abbreviation or acronym
- definition_notes: additional clarifying notes on the definition
- counter_examples: what this term does NOT mean, common confusions
- formula: calculation formula if this is a KPI/metric (null if not applicable)
- unit_of_measure: suggest the unit name (e.g., "Percentage (%)", "Currency", "Count") — user will map to dropdown
- term_type: suggest the type name (e.g., "KPI / Financial Metric", "Business Concept", "Regulatory Term", "Technical Term", "Process Term", "Product Term", "Risk Term", "Compliance Term") — user will map to dropdown
- business_context: business rules, operational rules, and calculation methodology
- examples: concrete examples of this term in use
- source_reference: authoritative source for this definition
- regulatory_reference: relevant regulatory references
- regulatory_tags: suggest regulatory framework names as comma-separated (e.g., "BCBS 239, IFRS 9")
- subject_areas: suggest business areas as comma-separated (e.g., "Retail Banking, Treasury")
- regulatory_reporting_usage: how this term is used in regulatory reports
- external_reference: external standard references (e.g., "BCBS 239 Principle 6")
- tags: suggest keywords for discoverability as comma-separated
- parent_term: suggest a logical parent term name if hierarchical
- related_terms: suggest related terms as comma-separated
- used_in_reports: reports where this term is used
- used_in_policies: policies referencing this term
- golden_source: authoritative source system for this term's data

RESPONSE FORMAT — you MUST respond with a JSON array conforming to this schema:
[
  {{
    "field_name": "string — exact field name from the list above, max 64 chars",
    "suggested_value": "string — max 2000 chars for text fields, max 50 chars for abbreviation, max 200 chars for comma-separated lists (tags, subject_areas, regulatory_tags)",
    "confidence": 0.85,
    "rationale": "string — max 500 chars, cite standards (DAMA DMBOK, BCBS 239, ISO 8000) where applicable"
  }}
]

ALLOWED VALUES for dropdown fields (suggest the DISPLAY NAME, not an ID):
- term_type: "KPI / Financial Metric", "Business Concept", "Regulatory Term", "Technical Term", "Process Term", "Product Term", "Risk Term", "Compliance Term"
- unit_of_measure: "Percentage", "Currency", "Count", "Ratio", "Days", "Months", "Years", "Basis Points", "Boolean", "Text", "Date", "Volume", "Weight", "Rate"

RULES:
- Never suggest values for fields ending in _id, _at, or _by
- Never suggest owner, steward, approver, organisational_unit, or domain_owner
- Only suggest for fields that are empty or missing — skip fields in the "already have values" list
- Every suggested_value MUST be a non-null, non-empty string
- Return [] if no suggestions are needed
- Return ONLY the JSON array — no markdown, no explanation text"#,
        entity_json = serde_json::to_string_pretty(entity_data).unwrap_or_default(),
        existing_list = existing_list,
    )
}

/// Generic prompt for non-glossary entity types (data_element, etc.)
fn build_generic_prompt(
    entity_type: &str,
    entity_data: &serde_json::Value,
    existing_list: &str,
) -> String {
    format!(
        r#"You are a metadata governance expert for financial institutions. Given the following {entity_type}, suggest improvements for empty or incomplete fields based on industry standards (DAMA DMBOK, BCBS 239, ISO 8000).

Entity type: {entity_type}
Current field values:
{entity_json}

Fields that already have values: {existing_list}

IMPORTANT: Only suggest values for TEXT fields shown above. NEVER suggest values for:
- Any field ending in _id (domain_id, category_id, owner_user_id, steward_user_id, classification_id, glossary_term_id, etc.)
- Any field ending in _at (timestamps)
- System fields like status_id, created_by, updated_by, version_number, is_current_version, is_cde, is_nullable

For each empty or improvable text field, provide:
1. field_name: the exact field name from the entity
2. suggested_value: your suggestion (must be a non-null string)
3. confidence: 0.0-1.0 how confident you are
4. rationale: why this suggestion, citing standards where applicable (must be a non-null string)

Return ONLY a JSON array of suggestions. Only suggest for fields that are empty, missing, or could be significantly improved. Do not suggest for fields that are already well-populated.

Example response format:
[
  {{
    "field_name": "business_context",
    "suggested_value": "Used in regulatory reporting...",
    "confidence": 0.85,
    "rationale": "Per BCBS 239 Principle 1..."
  }}
]

Return an empty array [] if no suggestions are needed."#,
        entity_type = entity_type,
        entity_json = serde_json::to_string_pretty(entity_data).unwrap_or_default(),
        existing_list = existing_list,
    )
}

// ---------------------------------------------------------------------------
// Claude client
// ---------------------------------------------------------------------------

async fn call_claude(
    config: &AiConfig,
    prompt: &str,
) -> Result<(String, String), AppError> {
    let api_key = config
        .anthropic_api_key
        .as_ref()
        .ok_or_else(|| AppError::AiService("Anthropic API key not configured".into()))?;

    // Force IPv4 resolution and use OS native TLS (CODING_STANDARDS Section 15.5)
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(90))
        .local_address(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED))
        .resolve(
            "api.anthropic.com",
            std::net::SocketAddr::new(
                std::net::IpAddr::V4(std::net::Ipv4Addr::new(160, 79, 104, 10)),
                443,
            ),
        )
        .build()
        .map_err(|e| AppError::AiService(format!("failed to build HTTP client: {e}")))?;

    let body = serde_json::json!({
        "model": config.anthropic_model,
        "max_tokens": 8192,
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ]
    });

    tracing::debug!("Calling Claude API at https://api.anthropic.com/v1/messages");

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::AiService(format!("Claude API request failed: {e}")))?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(AppError::AiService(format!(
            "Claude API returned {status}: {error_text}"
        )));
    }

    let claude_resp: ClaudeResponse = response
        .json()
        .await
        .map_err(|e| AppError::AiService(format!("Failed to parse Claude response: {e}")))?;

    let text = claude_resp
        .content
        .into_iter()
        .filter_map(|block| block.text)
        .collect::<Vec<_>>()
        .join("");

    Ok((text, claude_resp.model))
}

// ---------------------------------------------------------------------------
// OpenAI client
// ---------------------------------------------------------------------------

async fn call_openai(
    config: &AiConfig,
    prompt: &str,
) -> Result<(String, String), AppError> {
    let api_key = config
        .openai_api_key
        .as_ref()
        .ok_or_else(|| AppError::AiService("OpenAI API key not configured".into()))?;

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(90))
        .local_address(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED))
        .build()
        .map_err(|e| AppError::AiService(format!("failed to build HTTP client: {e}")))?;

    let body = serde_json::json!({
        "model": config.openai_model,
        "messages": [
            {
                "role": "system",
                "content": "You are a metadata governance expert for financial institutions. Always respond with valid JSON only."
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": 0.3,
        "max_tokens": 4096
    });

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| AppError::AiService(format!("OpenAI API request failed: {e}")))?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(AppError::AiService(format!(
            "OpenAI API returned {status}: {error_text}"
        )));
    }

    let openai_resp: OpenAiResponse = response
        .json()
        .await
        .map_err(|e| AppError::AiService(format!("Failed to parse OpenAI response: {e}")))?;

    let text = openai_resp
        .choices
        .first()
        .and_then(|c| c.message.content.clone())
        .unwrap_or_default();

    Ok((text, openai_resp.model))
}

// ---------------------------------------------------------------------------
// Response parser — extracts JSON array from AI text
// ---------------------------------------------------------------------------

fn parse_suggestions(text: &str) -> Result<Vec<RawAiSuggestion>, AppError> {
    // The AI may wrap JSON in markdown code fences; strip them
    let trimmed = text.trim();
    let json_str = if trimmed.starts_with("```") {
        // Strip opening ```json or ``` and closing ```
        let without_opening = trimmed
            .strip_prefix("```json")
            .or_else(|| trimmed.strip_prefix("```"))
            .unwrap_or(trimmed);
        without_opening
            .strip_suffix("```")
            .unwrap_or(without_opening)
            .trim()
    } else {
        trimmed
    };

    // Try to find a JSON array in the text
    let json_to_parse = if json_str.starts_with('[') {
        json_str.to_string()
    } else if let Some(start) = json_str.find('[') {
        if let Some(end) = json_str.rfind(']') {
            json_str[start..=end].to_string()
        } else {
            return Err(AppError::AiService(
                "AI response does not contain a valid JSON array".into(),
            ));
        }
    } else {
        return Err(AppError::AiService(
            "AI response does not contain a JSON array".into(),
        ));
    };

    let suggestions: Vec<RawAiSuggestion> = serde_json::from_str(&json_to_parse).map_err(|e| {
        AppError::AiService(format!("Failed to parse AI suggestions as JSON: {e}"))
    })?;

    // Validate and clean each suggestion per CODING_STANDARDS Section 15.2
    let suggestions = suggestions
        .into_iter()
        .filter_map(|mut s| {
            // 1. Field allow-list: drop suggestions for disallowed fields
            if s.field_name.is_empty()
                || s.field_name.ends_with("_id")
                || s.field_name.ends_with("_at")
                || s.field_name.ends_with("_by")
            {
                return None;
            }

            // 2. Drop empty suggestions
            if s.suggested_value.trim().is_empty() {
                return None;
            }

            // 3. Content cleaning: strip control chars and excessive whitespace
            s.suggested_value = s.suggested_value
                .chars()
                .filter(|c| !c.is_control() || *c == '\n')
                .collect::<String>()
                .trim()
                .to_string();
            s.rationale = s.rationale
                .chars()
                .filter(|c| !c.is_control() || *c == '\n')
                .collect::<String>()
                .trim()
                .to_string();

            // 4. Length enforcement per CODING_STANDARDS Section 15.2
            // Bounded fields (VARCHAR): reject if over limit — don't silently truncate
            // Unbounded fields (TEXT): no limit enforced
            let max_len: Option<usize> = match s.field_name.as_str() {
                "abbreviation" => Some(50),
                _ => None, // TEXT columns have no practical limit
            };
            if let Some(limit) = max_len {
                if s.suggested_value.len() > limit {
                    tracing::warn!(
                        field = %s.field_name,
                        length = s.suggested_value.len(),
                        max = limit,
                        "AI suggestion exceeds field length limit — dropping"
                    );
                    return None;
                }
            }

            // 5. Confidence clamping
            s.confidence = s.confidence.clamp(0.0, 1.0);

            Some(s)
        })
        .collect();

    Ok(suggestions)
}

// ---------------------------------------------------------------------------
// Public enrichment service
// ---------------------------------------------------------------------------

/// Calls the AI provider (Claude primary, OpenAI fallback) to generate
/// metadata enrichment suggestions for the given entity.
///
/// Returns structured suggestions with provider/model metadata.
pub async fn enrich_entity(
    config: &AiConfig,
    entity_type: &str,
    entity_data: serde_json::Value,
    existing_fields: Vec<String>,
) -> Result<EnrichmentResult, AppError> {
    // Verify at least one provider is configured
    if config.anthropic_api_key.is_none() && config.openai_api_key.is_none() {
        return Err(AppError::AiService(
            "No AI provider configured. Set ANTHROPIC_API_KEY or OPENAI_API_KEY in your environment.".into(),
        ));
    }

    let prompt = build_prompt(entity_type, &entity_data, &existing_fields);

    // Try Claude first (primary)
    if config.anthropic_api_key.is_some() {
        match call_claude(config, &prompt).await {
            Ok((text, model)) => {
                tracing::info!(provider = "claude", model = %model, "AI enrichment call succeeded");
                let suggestions = parse_suggestions(&text)?;
                return Ok(EnrichmentResult {
                    suggestions,
                    provider: "CLAUDE".to_string(),
                    model,
                });
            }
            Err(e) => {
                tracing::warn!(error = %e, "Claude API failed, attempting OpenAI fallback");
                // Fall through to OpenAI
            }
        }
    }

    // Fallback to OpenAI
    if config.openai_api_key.is_some() {
        match call_openai(config, &prompt).await {
            Ok((text, model)) => {
                tracing::info!(provider = "openai", model = %model, "AI enrichment call succeeded (fallback)");
                let suggestions = parse_suggestions(&text)?;
                return Ok(EnrichmentResult {
                    suggestions,
                    provider: "OPENAI".to_string(),
                    model,
                });
            }
            Err(e) => {
                return Err(AppError::AiService(format!(
                    "Both AI providers failed. Last error: {e}"
                )));
            }
        }
    }

    Err(AppError::AiService(
        "No AI provider available for enrichment".into(),
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_clean_json_array() {
        let text = r#"[
            {
                "field_name": "business_context",
                "suggested_value": "Used for regulatory reporting",
                "confidence": 0.85,
                "rationale": "Per BCBS 239"
            }
        ]"#;

        let suggestions = parse_suggestions(text).unwrap();
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].field_name, "business_context");
        assert!((suggestions[0].confidence - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_code_fenced_json() {
        let text = r#"```json
[
    {
        "field_name": "examples",
        "suggested_value": "Customer ID: C12345",
        "confidence": 0.7,
        "rationale": "Standard example format"
    }
]
```"#;

        let suggestions = parse_suggestions(text).unwrap();
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].field_name, "examples");
    }

    #[test]
    fn parse_empty_array() {
        let text = "[]";
        let suggestions = parse_suggestions(text).unwrap();
        assert!(suggestions.is_empty());
    }

    #[test]
    fn parse_clamps_confidence() {
        let text = r#"[{"field_name":"f","suggested_value":"v","confidence":1.5,"rationale":"r"}]"#;
        let suggestions = parse_suggestions(text).unwrap();
        assert!((suggestions[0].confidence - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_json_with_surrounding_text() {
        let text = r#"Here are my suggestions:
[{"field_name":"f","suggested_value":"v","confidence":0.5,"rationale":"r"}]
Hope this helps!"#;
        let suggestions = parse_suggestions(text).unwrap();
        assert_eq!(suggestions.len(), 1);
    }

    #[test]
    fn parse_invalid_json_returns_error() {
        let text = "This is not JSON at all.";
        assert!(parse_suggestions(text).is_err());
    }

    #[test]
    fn prompt_includes_entity_type_for_generic() {
        let prompt = build_prompt(
            "data_element",
            &serde_json::json!({"element_name": "Test"}),
            &["element_name".to_string()],
        );
        assert!(prompt.contains("data_element"));
        assert!(prompt.contains("DAMA DMBOK"));
        assert!(prompt.contains("BCBS 239"));
    }

    #[test]
    fn glossary_prompt_includes_all_enrichable_fields() {
        let prompt = build_prompt(
            "glossary_term",
            &serde_json::json!({"term_name": "NPL Ratio", "definition": "Non-performing loan ratio"}),
            &["term_name".to_string(), "definition".to_string()],
        );
        assert!(prompt.contains("glossary_term"));
        assert!(prompt.contains("definition_notes"));
        assert!(prompt.contains("counter_examples"));
        assert!(prompt.contains("formula"));
        assert!(prompt.contains("unit_of_measure"));
        assert!(prompt.contains("term_type"));
        assert!(prompt.contains("regulatory_tags"));
        assert!(prompt.contains("subject_areas"));
        assert!(prompt.contains("regulatory_reporting_usage"));
        assert!(prompt.contains("external_reference"));
        assert!(prompt.contains("golden_source"));
        assert!(prompt.contains("used_in_reports"));
        assert!(prompt.contains("used_in_policies"));
        assert!(prompt.contains("parent_term"));
    }
}
