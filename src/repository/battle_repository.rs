use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, Condition, DatabaseConnection, DbErr,
    EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};
use uuid::Uuid;

use crate::models::battles::{self, Battle, Entity as BattleEntity};

#[async_trait]
pub trait BattleRepository: Send + Sync {
    async fn create(&self, id: Uuid, user1: Uuid, user2: Uuid, level: i32) -> Result<(), DbErr>;
    async fn set_task(&self, id: Uuid, text: &str) -> Result<(), DbErr>;
    async fn finish(
        &self,
        id: Uuid,
        winner: Option<Uuid>,
        score_user1: i32,
        score_user2: i32,
        elo_delta_user1: i32,
        elo_delta_user2: i32,
    ) -> Result<(), DbErr>;
    async fn find_finished_by_user(&self, user_id: Uuid) -> Result<Vec<Battle>, DbErr>;
}

pub struct PgBattleRepository {
    db: DatabaseConnection,
}

impl PgBattleRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl BattleRepository for PgBattleRepository {
    async fn create(&self, id: Uuid, user1: Uuid, user2: Uuid, level: i32) -> Result<(), DbErr> {
        let active = battles::ActiveModel {
            id: Set(id),
            user1_id: Set(user1),
            user2_id: Set(user2),
            level: Set(level),
            task_text: Set(None),
            start_at: Set(Utc::now()),
            end_at: Set(None),
            score_user1: Set(0),
            score_user2: Set(0),
            elo_delta_user1: Set(0),
            elo_delta_user2: Set(0),
            winner_id: Set(None),
        };
        active.insert(&self.db).await?;
        Ok(())
    }

    async fn set_task(&self, id: Uuid, text: &str) -> Result<(), DbErr> {
        let active = battles::ActiveModel {
            id: Set(id),
            task_text: Set(Some(text.to_string())),
            ..Default::default()
        };
        active.update(&self.db).await?;
        Ok(())
    }

    async fn finish(
        &self,
        id: Uuid,
        winner: Option<Uuid>,
        score_user1: i32,
        score_user2: i32,
        elo_delta_user1: i32,
        elo_delta_user2: i32,
    ) -> Result<(), DbErr> {
        let active = battles::ActiveModel {
            id: Set(id),
            end_at: Set(Some(Utc::now())),
            winner_id: Set(winner),
            score_user1: Set(score_user1),
            score_user2: Set(score_user2),
            elo_delta_user1: Set(elo_delta_user1),
            elo_delta_user2: Set(elo_delta_user2),
            ..Default::default()
        };
        active.update(&self.db).await?;
        Ok(())
    }

    async fn find_finished_by_user(&self, user_id: Uuid) -> Result<Vec<Battle>, DbErr> {
        BattleEntity::find()
            .filter(
                Condition::any()
                    .add(battles::Column::User1Id.eq(user_id))
                    .add(battles::Column::User2Id.eq(user_id)),
            )
            .filter(battles::Column::EndAt.is_not_null())
            .order_by_desc(battles::Column::EndAt)
            .limit(20)
            .all(&self.db)
            .await
    }
}
