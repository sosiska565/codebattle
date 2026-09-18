use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

pub struct TokenService {
    jwt_secret: String,
    ttl_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub iat: u64,
    pub exp: u64,
}

impl TokenService {
    pub fn new(jwt_secret: &str, ttl_seconds: i64) -> Self {
        Self {
            jwt_secret: jwt_secret.to_string(),
            ttl_seconds,
        }
    }
    pub fn generate_token(&self, user_id: Uuid, username: &str) -> Result<String, AppError> {
        let now = Utc::now();
        let claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            iat: now.timestamp() as u64,
            exp: (now + Duration::seconds(self.ttl_seconds)).timestamp() as u64,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(AppError::Jwt)
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, AppError> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(AppError::Jwt)
    }
}
