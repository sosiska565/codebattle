use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Default, Debug, Clone, FromRow, Deserialize, Serialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub elo: i32,
    pub pass_hash: String,
    pub created_at: chrono::NaiveDate,
    pub email: String,
}

#[derive(Default, Debug, Clone, FromRow, Deserialize, Serialize)]
pub struct UserCreateDto {
    pub username: String,
    pub raw_password: String,
    pub email: String,
}

#[derive(Default, Debug, Clone, FromRow, Deserialize, Serialize)]
pub struct UserGetIdDto {
    pub username: String,
    pub email: String,
    pub elo: i32,
}

#[derive(Default, Debug, Clone, FromRow, Deserialize, Serialize)]
pub struct UserGetUsernameDto {
    pub id: Uuid,
    pub email: String,
    pub elo: i32,
}
