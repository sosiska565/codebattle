use crate::models::users::{self, Entity as UserEntity, User};
use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter, dynamic::Column,
};

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_all(&self) -> Result<Vec<User>, DbErr>;
    async fn find_by_id(&self, id: &uuid::Uuid) -> Result<Option<User>, DbErr>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DbErr>;
    async fn exists_by_username(&self, email: &str) -> Result<bool, DbErr>;
    async fn exists_by_email(&self, email: &str) -> Result<bool, DbErr>;
    async fn create(&self, user: &User) -> Result<User, DbErr>;
    async fn update(&self, user: &User) -> Result<User, DbErr>;
    async fn delete_by_id(&self, id: &uuid::Uuid) -> Result<(), DbErr>;
}

pub struct PgUserRepository {
    db: DatabaseConnection,
}

impl PgUserRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn find_all(&self) -> Result<Vec<User>, DbErr> {
        UserEntity::find().all(&self.db).await
    }

    async fn find_by_id(&self, id: &uuid::Uuid) -> Result<Option<User>, DbErr> {
        UserEntity::find_by_id(*id).one(&self.db).await
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DbErr> {
        UserEntity::find()
            .filter(users::Column::Email.eq(email))
            .one(&self.db)
            .await
    }

    async fn exists_by_username(&self, username: &str) -> Result<bool, DbErr> {
        let count = UserEntity::find()
            .filter(users::Column::Username.eq(username))
            .count(&self.db)
            .await?;
        Ok(count > 0)
    }

    async fn exists_by_email(&self, email: &str) -> Result<bool, DbErr> {
        let count = UserEntity::find()
            .filter(users::Column::Email.eq(email))
            .count(&self.db)
            .await?;
        Ok(count > 0)
    }

    async fn create(&self, user: &User) -> Result<User, DbErr> {
        let active = users::ActiveModel {
            id: Set(user.id),
            username: Set(user.username.clone()),
            email: Set(user.email.clone()),
            pass_hash: Set(user.pass_hash.clone()),
            elo: Set(user.elo),
            created_at: Set(user.created_at),
        };

        active.insert(&self.db).await
    }

    async fn update(&self, user: &User) -> Result<User, DbErr> {
        let active = users::ActiveModel {
            id: Set(user.id),
            username: Set(user.username.clone()),
            email: Set(user.email.clone()),
            ..Default::default()
        };

        active.update(&self.db).await
    }

    async fn delete_by_id(&self, id: &uuid::Uuid) -> Result<(), DbErr> {
        UserEntity::delete_by_id(*id).exec(&self.db).await?;
        Ok(())
    }
}
