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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawAiSuggestion {
    pub field_name: String,
    pub suggested_value: String,
    pub confidence: f64,
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

    format!(
        r#"You are a metadata governance expert for financial institutions. Given the following {entity_type}, suggest improvements for empty or incomplete fields based on industry standards (DAMA DMBOK, BCBS 239, ISO 8000).

Entity type: {entity_type}
Current field values:
{entity_json}

Fields that already have values: {existing_list}

For each empty or improvable field, provide:
1. field_name: the exact field name from the entity
2. suggested_value: your suggestion
3. confidence: 0.0-1.0 how confident you are
4. rationale: why this suggestion, citing standards where applicable

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

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": config.anthropic_model,
        "max_tokens": 4096,
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ]
    });

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(60))
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

    let client = reqwest::Client::new();
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

    // Clamp confidence values to [0.0, 1.0]
    let suggestions = suggestions
        .into_iter()
        .map(|mut s| {
            s.confidence = s.confidence.clamp(0.0, 1.0);
            s
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
    fn prompt_includes_entity_type() {
        let prompt = build_prompt(
            "glossary_term",
            &serde_json::json!({"term_name": "Test"}),
            &["term_name".to_string()],
        );
        assert!(prompt.contains("glossary_term"));
        assert!(prompt.contains("DAMA DMBOK"));
        assert!(prompt.contains("BCBS 239"));
    }
}
