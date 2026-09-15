use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::DateTime;
use chrono::Utc;

#[derive(Default, Debug, Clone, FromRow, Deserialize, Serialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub elo: i32,
    pub pass_hash: String,
    pub created_at: DateTime<Utc>,
    pub email: String,
}
