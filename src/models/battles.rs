use chrono::DateTime;
use chrono::Utc;
use sea_orm::ActiveModelBehavior;
use sea_orm::DeriveEntityModel;
use sea_orm::DerivePrimaryKey;
use sea_orm::EntityTrait;
use sea_orm::EnumIter;
use sea_orm::PrimaryKeyTrait;
use sea_orm::RelationDef;
use sea_orm::RelationTrait;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default, DeriveEntityModel, PartialEq)]
#[sea_orm(table_name = "battles")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    id: uuid::Uuid,
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    score_user1: i32,
    score_user2: i32,
}

#[derive(Clone, Copy, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("no relations defined for users")
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type Battle = Model;
