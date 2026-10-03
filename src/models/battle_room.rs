use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;

pub const START_HP: i32 = 100;
pub const DAMAGE_PER_SOLVE: i32 = 34;

#[derive(Clone, Debug, Serialize)]
pub struct PlayerInfo {
    pub id: Uuid,
    pub username: String,
    pub elo: i32,
}

pub struct Player {
    pub conn_id: Uuid,
    pub sender: mpsc::UnboundedSender<ServerMsg>,
}

#[derive(Clone, Copy, Debug)]
pub struct Stats {
    pub hp: i32,
    pub score: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
    Idle,
    Generating,
    Ready,
}

pub struct Finish {
    pub winner: Option<Uuid>,
    pub reason: &'static str,
    pub elo_delta: HashMap<Uuid, i32>,
}

pub struct BattleRoom {
    pub battle_id: Uuid,
    pub level: i32,
    pub members: Vec<PlayerInfo>,
    pub players: HashMap<Uuid, Player>,
    pub stats: HashMap<Uuid, Stats>,
    pub judging: HashSet<Uuid>,
    pub task_state: TaskState,
    pub round: u32,
    pub task_text: String,
    pub task_names: Vec<String>,
    pub all_tasks: String,
    pub finish: Option<Finish>,
}

impl BattleRoom {
    pub fn new(battle_id: Uuid, level: i32, a: &PlayerInfo, b: &PlayerInfo) -> Self {
        let mut stats = HashMap::new();
        stats.insert(
            a.id,
            Stats {
                hp: START_HP,
                score: 0,
            },
        );
        stats.insert(
            b.id,
            Stats {
                hp: START_HP,
                score: 0,
            },
        );
        Self {
            battle_id,
            level,
            members: vec![a.clone(), b.clone()],
            players: HashMap::new(),
            stats,
            judging: HashSet::new(),
            task_state: TaskState::Idle,
            round: 1,
            task_text: String::new(),
            task_names: Vec::new(),
            all_tasks: String::new(),
            finish: None,
        }
    }

    pub fn opponent_of(&self, user: Uuid) -> Option<Uuid> {
        self.members.iter().find(|m| m.id != user).map(|m| m.id)
    }

    pub fn stats_of(&self, user: Uuid) -> Stats {
        self.stats.get(&user).copied().unwrap_or(Stats {
            hp: START_HP,
            score: 0,
        })
    }

    pub fn broadcast(&self, msg: ServerMsg) {
        for player in self.players.values() {
            let _ = player.sender.send(msg.clone());
        }
    }

    pub fn send_to(&self, user: Uuid, msg: ServerMsg) {
        if let Some(player) = self.players.get(&user) {
            let _ = player.sender.send(msg);
        }
    }

    pub fn state_for(&self, user: Uuid, solved_by: Option<Uuid>) -> ServerMsg {
        let me = self.stats_of(user);
        let opp = self
            .opponent_of(user)
            .map(|o| self.stats_of(o))
            .unwrap_or(Stats {
                hp: START_HP,
                score: 0,
            });
        ServerMsg::StateUpdate {
            my_hp: me.hp,
            opponent_hp: opp.hp,
            my_score: me.score,
            opponent_score: opp.score,
            solved_by,
        }
    }

    pub fn broadcast_state(&self, solved_by: Option<Uuid>) {
        for (id, player) in &self.players {
            let _ = player.sender.send(self.state_for(*id, solved_by));
        }
    }

    pub fn match_end_for(&self, user: Uuid) -> Option<ServerMsg> {
        let fin = self.finish.as_ref()?;
        let me = self.stats_of(user);
        let opp = self
            .opponent_of(user)
            .map(|o| self.stats_of(o))
            .unwrap_or(Stats {
                hp: START_HP,
                score: 0,
            });
        let result = match fin.winner {
            Some(w) if w == user => "win",
            Some(_) => "loss",
            None => "draw",
        };
        Some(ServerMsg::MatchEnd {
            result: result.to_string(),
            reason: fin.reason.to_string(),
            winner_id: fin.winner,
            your_score: me.score,
            opponent_score: opp.score,
            elo_delta: fin.elo_delta.get(&user).copied().unwrap_or(0),
        })
    }

    pub fn broadcast_match_end(&self) {
        for id in self.players.keys() {
            if let Some(msg) = self.match_end_for(*id) {
                self.send_to(*id, msg);
            }
        }
    }
}

pub type BattleManager = Mutex<HashMap<Uuid, Arc<Mutex<BattleRoom>>>>;

#[derive(Clone, Serialize)]
#[serde(tag = "type", content = "payload")]
pub enum ServerMsg {
    MatchFound {
        battle_id: Uuid,
        opponent: PlayerInfo,
    },
    BattleInit {
        battle_id: Uuid,
        level: i32,
        me: PlayerInfo,
        opponent: PlayerInfo,
        max_hp: i32,
        my_hp: i32,
        opponent_hp: i32,
        my_score: i32,
        opponent_score: i32,
    },
    WaitingOpponent,
    TaskGenerating,
    TaskStatus {
        text: String,
    },
    TaskChunk {
        text: String,
    },
    TaskReady {
        text: String,
    },
    Judging,
    SubmitResult {
        verdict: String,
        passed: usize,
        total: usize,
        message: String,
    },
    StateUpdate {
        my_hp: i32,
        opponent_hp: i32,
        my_score: i32,
        opponent_score: i32,
        solved_by: Option<Uuid>,
    },
    MatchEnd {
        result: String,
        reason: String,
        winner_id: Option<Uuid>,
        your_score: i32,
        opponent_score: i32,
        elo_delta: i32,
    },
    Error {
        message: String,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ClienMsg {
    Submit { language: String, code: String },
    Surrender,
}
