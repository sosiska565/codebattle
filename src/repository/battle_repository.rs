use sea_orm::DbErr;
use sqlx::PgPool;

use crate::models::{
    battles::Battle,
    dto::battle_dto::{BattleCreateRequest, BattleResponse},
};

#[async_trait::async_trait]
pub trait BattleRepository: Send + Sync {}

pub struct PgBattleRepository {}

impl PgBattleRepository {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl BattleRepository for PgBattleRepository {}
