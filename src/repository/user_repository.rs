use crate::models::user::User;
use chrono::Utc;
use sqlx::PgPool;

pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    pub async fn get_by_id(&self, id: i32) -> Result<Option<User>, Box<dyn std::error::Error>> {
        let user =
            sqlx::query_as::<_, User>("SELECT id, username, elo, email FROM users WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(user)
    }
    pub async fn get_by_username(
        &self,
        username: String,
    ) -> Result<Option<User>, Box<dyn std::error::Error>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, username, elo, email FROM users WHERE username = $1",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }
    pub async fn create(
        &self,
        username: String,
        elo: i32,
        email: i32,
        pass_hash: String,
    ) -> Result<User, Box<dyn std::error::Error>> {
        let user = sqlx::query_as::<_, User>("INSER INTO users (username, elo, email, pass_hash, created_at) VALUES ($1, $2, $3, $4, $5) RETURNING id, username, elo, email, pass_hash, created_at")
            .bind(username)
            .bind(elo)
            .bind(email)
            .bind(pass_hash)
            .bind(Utc::now())
            .fetch_one(&self.pool)
            .await?;

        Ok(user)
    }
    pub async fn delete_by_id(&self, id: uuid::Uuid) -> Result<(), Box<dyn std::error::Error>> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn update(&self, user: User) -> Result<User, Box<dyn std::error::Error>> {
        let usr = sqlx::query_as::<_, User>(
            "UPDATE users SET username = $1, email = $2, pass_hash = $3 WHERE id = $4",
        )
        .bind(user.username)
        .bind(user.email)
        .bind(user.pass_hash)
        .bind(user.id)
        .fetch_one(&self.pool)
        .await?;

        Ok(usr)
    }
}
