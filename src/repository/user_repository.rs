use crate::models::users::User;
use async_trait::async_trait;
use sqlx::PgPool;

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_all(&self) -> Result<Vec<User>, sqlx::Error>;
    async fn find_by_id(&self, id: &uuid::Uuid) -> Result<Option<User>, sqlx::Error>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error>;
    async fn exists_by_username(&self, email: &str) -> Result<bool, sqlx::Error>;
    async fn exists_by_email(&self, email: &str) -> Result<bool, sqlx::Error>;
    async fn create(&self, user: &User) -> Result<User, sqlx::Error>;
    async fn update(&self, user: &User) -> Result<User, sqlx::Error>;
    async fn delete_by_id(&self, id: &uuid::Uuid) -> Result<(), sqlx::Error>;
}

pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn find_all(&self) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, elo, email, pass_hash, created_at FROM users",
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn find_by_id(&self, id: &uuid::Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await
    }

    async fn exists_by_username(&self, username: &str) -> Result<bool, sqlx::Error> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)")
                .bind(username)
                .fetch_one(&self.pool)
                .await?;

        Ok(exists)
    }

    async fn exists_by_email(&self, email: &str) -> Result<bool, sqlx::Error> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
                .bind(email)
                .fetch_one(&self.pool)
                .await?;

        Ok(exists)
    }

    async fn create(&self, user: &User) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "INSERT INTO users (id, username, email, pass_hash, created_at, elo)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, username, email, pass_hash, created_at, elo",
        )
        .bind(&user.id)
        .bind(&user.username)
        .bind(&user.email)
        .bind(&user.pass_hash)
        .bind(&user.created_at)
        .bind(&user.elo)
        .fetch_one(&self.pool)
        .await
    }

    async fn update(&self, user: &User) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "UPDATE users SET username = $1, email = $2 WHERE id = $3
         RETURNING id, username, email, pass_hash, created_at, elo",
        )
        .bind(&user.username)
        .bind(&user.email)
        .bind(&user.id)
        .fetch_one(&self.pool)
        .await
    }

    async fn delete_by_id(&self, id: &uuid::Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
