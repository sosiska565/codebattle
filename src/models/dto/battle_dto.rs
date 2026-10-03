use chrono::{DateTime, Utc};
use serde::Serialize;

// pub struct BattleCreateRequest {}
//
// pub struct BattleResponse {}

#[derive(Debug, Serialize)]
pub struct BattleHistoryItem {
    pub opponent: String,
    pub task: String,
    pub result: String,
    pub elo_delta: i32,
    pub date: DateTime<Utc>,
}
