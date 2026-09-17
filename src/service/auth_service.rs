use std::sync::Arc;

use crate::repository::user_repository::UserRepository;

pub struct AuthService<R: UserRepository> {
    repo: Arc<R>,
    jwt_secret: String,
}

impl<R: UserRepository> AuthService<R> {
    pub fn new(repo: Arc<R>, jwt_secret: &str) -> Self {
        Self { repo, jwt_secret }
    }

    pub async fn login(&self, dto: UserLoginRequest) -> Result<TokenPair, AppError> {}
    pub async fn logout(&self, token: &str) -> Result<(), AppError> {}
    pub async fn refresh(&self, token: &str) -> Result<TokenPair, AppError> {}
}
