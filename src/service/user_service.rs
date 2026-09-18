use crate::error::AppError;
use crate::models::dto::auth_dto::TokenResponse;
use crate::models::dto::user_dto::{UserCreateRequest, UserUpdateRequest};
use crate::models::users::User;
use crate::repository::user_repository::UserRepository;
use bcrypt::{DEFAULT_COST, hash};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

pub struct UserService<R: UserRepository> {
    repo: Arc<R>,
}

impl<R: UserRepository> UserService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn get_all(&self) -> Result<Vec<User>, AppError> {
        Ok(self.repo.find_all().await?)
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<User, AppError> {
        self.repo.find_by_id(&id).await?.ok_or(AppError::NotFound)
    }

    pub async fn get_by_email(&self, email: String) -> Result<User, AppError> {
        self.repo
            .find_by_email(&email)
            .await?
            .ok_or(AppError::NotFound)
    }

    pub async fn create(&self, dto: UserCreateRequest) -> Result<User, AppError> {
        if self.repo.exists_by_username(&dto.username).await? {
            return Err(AppError::Conflict("username already taken".into()));
        }
        if self.repo.exists_by_email(&dto.email).await? {
            return Err(AppError::Conflict("email already registered".into()));
        }

        let password = dto.password;
        let pass_hash = tokio::task::spawn_blocking(move || hash(password, DEFAULT_COST))
            .await
            .map_err(|e| AppError::Internal(e.into()))?
            .map_err(AppError::Hash)?;

        let user = User {
            id: Uuid::new_v4(),
            username: dto.username,
            email: dto.email,
            pass_hash,
            elo: 1000,
            created_at: Utc::now(),
        };

        self.repo.create(&user).await.map_err(AppError::from)
    }

    pub async fn update(&self, id: Uuid, dto: UserUpdateRequest) -> Result<User, AppError> {
        let mut user = self.repo.find_by_id(&id).await?.ok_or(AppError::NotFound)?;

        if let Some(new_email) = &dto.email
            && new_email != &user.email
            && self.repo.exists_by_email(new_email).await?
        {
            return Err(AppError::Conflict("email already in use".into()));
        }

        if let Some(new_username) = &dto.username
            && new_username != &user.username
            && self.repo.exists_by_username(new_username).await?
        {
            return Err(AppError::Conflict("username already taken".into()));
        }

        if let Some(new_email) = dto.email {
            user.email = new_email
        }

        if let Some(new_username) = dto.username {
            user.username = new_username
        }

        self.repo.update(&user).await.map_err(AppError::from)
    }

    pub async fn delete_by_id(&self, id: Uuid) -> Result<(), AppError> {
        self.repo.find_by_id(&id).await?.ok_or(AppError::NotFound)?;
        self.repo.delete_by_id(&id).await.map_err(AppError::from)
    }
}
