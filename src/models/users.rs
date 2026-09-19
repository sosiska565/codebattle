use chrono::DateTime;
use chrono::Utc;
use sea_orm::DeriveEntityModel;
use sea_orm::EnumIter;
use sea_orm::RelationDef;
use sea_orm::RelationTrait;
use sea_orm::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Default, Debug, Clone, Deserialize, Serialize, DeriveEntityModel, PartialEq)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub username: String,
    pub elo: i32,
    pub pass_hash: String,
    pub created_at: DateTime<Utc>,
    pub email: String,
}

#[derive(Clone, Copy, Debug, EnumIter)]
pub enum Relation {}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        panic!("no relations defined for users")
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub type User = Model;
