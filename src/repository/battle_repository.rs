use sqlx::PgPool;

use crate::models::dto::battle_dto::{BattleCreateRequest, BattleResponse};

#[async_trait::async_trait]
pub trait BattleRepository: Send + Sync {
    async fn create(&self, dto: BattleCreateRequest) -> Result<BattleResponse, sqlx::Error>;
}

struct PgBattleRepository {
    pool: PgPool,
}

impl PgBattleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl BattleRepository for PgBattleRepository {
    async fn create(&self, dto: BattleCreateRequest) -> Result<BattleResponse, sqlx::Error> {}
}
