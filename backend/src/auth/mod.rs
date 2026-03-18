pub mod middleware;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Claims extracted from a validated JWT token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub email: String,
    pub display_name: String,
    pub roles: Vec<String>,
    pub exp: usize,
    pub iat: usize,
}
