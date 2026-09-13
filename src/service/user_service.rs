use crate::models::user::User;
use crate::repository::user_repository;
use sqlx::PgPool;

pub struct UserService {
    repo: user_repository::UserRepository,
}

impl UserService {
    pub fn new(repo: user_repository::UserRepository) -> Self {
        Self { repo }
    }
    pub async fn create(&self, user: User) -> Result<User, Box<dyn std::error::Error>> {}
}
