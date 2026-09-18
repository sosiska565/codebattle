use chrono::DateTime;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(Serialize, Deserialize, Debug, Default, FromRow)]
pub struct Battle {
    id: uuid::Uuid,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    score_user1: i32,
    score_user2: i32,
    hp_user1: i32,
    hp_user2: i32,
}
