use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use crate::models::users::User;

#[derive(Default, Debug, Clone, FromRow, Deserialize, Serialize)]
pub struct UserCreateRequest {
    pub username: String,
    pub email: String,
    pub password: String
}

#[derive(Debug, Serialize)]
pub struct UserUpdateRequest {
    pub username: Option<String>,
    pub email: Option<String>
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid, 
    pub username: String, 
    pub elo: i32 
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self { id: u.id, username: u.username, elo: u.elo }
    }
}