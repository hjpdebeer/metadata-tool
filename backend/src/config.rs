use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub jwt_secret: String,
    pub jwt_expiry_hours: u64,
    pub entra: EntraConfig,
    pub graph: GraphConfig,
    pub ai: AiConfig,
    pub frontend_url: String,
}

#[derive(Debug, Clone)]
pub struct EntraConfig {
    pub tenant_id: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone)]
pub struct GraphConfig {
    pub tenant_id: String,
    pub client_id: String,
    pub client_secret: String,
    pub sender_email: String,
}

#[derive(Debug, Clone)]
pub struct AiConfig {
    pub primary_provider: String,
    pub anthropic_api_key: Option<String>,
    pub anthropic_model: String,
    pub openai_api_key: Option<String>,
    pub openai_model: String,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let port_str = env::var("PORT").unwrap_or_else(|_| "8080".into());
        let port: u16 = port_str
            .parse()
            .map_err(|_| anyhow::anyhow!("PORT must be a valid u16, got '{port_str}'"))?;

        let expiry_str = env::var("JWT_EXPIRY_HOURS").unwrap_or_else(|_| "8".into());
        let jwt_expiry_hours: u64 = expiry_str.parse().map_err(|_| {
            anyhow::anyhow!("JWT_EXPIRY_HOURS must be a valid u64, got '{expiry_str}'")
        })?;

        // SEC-002: Validate JWT secret at startup
        let jwt_secret = env::var("JWT_SECRET")?;
        if jwt_secret.len() < 32 {
            return Err(anyhow::anyhow!(
                "JWT_SECRET must be at least 32 characters (got {}). Generate with: openssl rand -base64 48",
                jwt_secret.len()
            ));
        }
        if jwt_secret == "change-this-to-a-secure-random-string" {
            return Err(anyhow::anyhow!(
                "JWT_SECRET must be changed from the default. Generate with: openssl rand -base64 48"
            ));
        }

        Ok(Self {
            database_url: env::var("DATABASE_URL")?,
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port,
            jwt_secret,
            jwt_expiry_hours,
            entra: EntraConfig {
                tenant_id: env::var("ENTRA_TENANT_ID")?,
                client_id: env::var("ENTRA_CLIENT_ID")?,
                client_secret: env::var("ENTRA_CLIENT_SECRET")?,
                redirect_uri: env::var("ENTRA_REDIRECT_URI")?,
            },
            graph: GraphConfig {
                tenant_id: env::var("GRAPH_TENANT_ID")?,
                client_id: env::var("GRAPH_CLIENT_ID")?,
                client_secret: env::var("GRAPH_CLIENT_SECRET")?,
                sender_email: env::var("GRAPH_SENDER_EMAIL")?,
            },
            ai: AiConfig {
                primary_provider: env::var("AI_PRIMARY_PROVIDER")
                    .unwrap_or_else(|_| "claude".into()),
                anthropic_api_key: env::var("ANTHROPIC_API_KEY").ok(),
                anthropic_model: env::var("ANTHROPIC_MODEL")
                    .unwrap_or_else(|_| "claude-sonnet-4-6".into()),
                openai_api_key: env::var("OPENAI_API_KEY").ok(),
                openai_model: env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".into()),
            },
            frontend_url: env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "http://localhost:5173".into()),
        })
    }
}
