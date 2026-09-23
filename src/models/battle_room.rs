use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc};

pub struct Player {
    pub user_id: uuid::Uuid,
    pub hp: i32,
    pub score: i32,
    pub sender: mpsc::UnboundedSender<String>,
}

pub struct BattleRoom {
    pub battle_id: uuid::Uuid,
    pub players: HashMap<uuid::Uuid, Player>,
}

pub type BattleManager = Mutex<HashMap<uuid::Uuid, Arc<Mutex<BattleRoom>>>>;

#[derive(Serialize)]
#[serde(tag = "type", content = "payload")]
pub enum ServerMsg {
    TaskReady {
        text: String,
    },
    EnemyProgress {
        hp: i32,
    },
    MatchEnd {
        winner_id: uuid::Uuid,
        winner_score: i32,
        your_score: i32,
    },
    Error {
        message: String,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ClienMsg {
    Submit { language: String, code: String },
}
