use crate::{
    error::AppError, models::dto::auth_dto::TokenResponse, models::dto::user_dto::UserLoginRequest,
};
use std::sync::Arc;

use axum::handler::Handler;
use bcrypt::verify;
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::repository::user_repository::UserRepository;

pub struct AuthService<R: UserRepository> {
    repo: Arc<R>,
    jwt_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub uid: uuid::Uuid,
    pub username: String,
    pub iat: u64,
    pub exp: u64,
}

const TOKEN_TIME: i64 = 60 * 60 * 24;

impl<R: UserRepository> AuthService<R> {
    pub fn new(repo: Arc<R>, jwt_secret: &str) -> Self {
        Self {
            repo,
            jwt_secret: jwt_secret.to_string(),
        }
    }

    pub async fn login(&self, dto: UserLoginRequest) -> Result<TokenResponse, AppError> {
        let user = self
            .repo
            .find_by_email(&dto.email)
            .await?
            .ok_or(AppError::Unauthorized("User not found".to_string()))?;

        let valid = tokio::task::spawn_blocking(move || verify(&dto.password, &user.pass_hash))
            .await
            .map_err(|e| AppError::Internal(e.into()))?
            .map_err(AppError::Hash)?;

        if !valid {
            return Err(AppError::Unauthorized("Wrong password".to_string()));
        }

        let access_token = self.generate_token(user.id, &user.username)?;

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: TOKEN_TIME,
        })
    }

    pub fn generate_token(&self, user_id: Uuid, username: &str) -> Result<String, AppError> {
        let now = Utc::now();
        let claims = Claims {
            uid: user_id,
            username: username.to_string(),
            iat: now.timestamp() as u64,
            exp: (now + Duration::seconds(TOKEN_TIME)).timestamp() as u64,
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
    //pub async fn logout(&self, token: &str) -> Result<(), AppError> {}
    //pub async fn refresh(&self, token: &str) -> Result<TokenPair, AppError> {}
}
