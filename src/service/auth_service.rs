use crate::{
    error::AppError, models::dto::auth_dto::TokenResponse, models::dto::user_dto::UserLoginRequest,
    service::token_service::TokenService,
};
use std::sync::Arc;

use bcrypt::verify;

use crate::repository::user_repository::UserRepository;

pub struct AuthService<R: UserRepository> {
    repo: Arc<R>,
    token_service: Arc<TokenService>,
}

const TOKEN_TIME: i64 = 60 * 60 * 24;

impl<R: UserRepository> AuthService<R> {
    pub fn new(repo: Arc<R>, token_service: Arc<TokenService>) -> Self {
        Self {
            repo,
            token_service,
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

        let access_token = self.token_service.generate_token(user.id, &user.username)?;

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: TOKEN_TIME,
        })
    }

    //pub async fn logout(&self, token: &str) -> Result<(), AppError>
    //pub async fn refresh(&self, token: &str) -> Result<TokenPair, AppError> {}
}
