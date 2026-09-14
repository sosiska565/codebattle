use crate::models::user::User;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    pub async fn get_all(&self) -> Result<Vec<User>, sqlx::Error> {
        let users = sqlx::query_as::<_, User>("SELECT id, username, elo, email FROM users")
            .fetch_all(&self.pool)
            .await?;

        Ok(users)
    }
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        let user =
            sqlx::query_as::<_, User>("SELECT id, username, elo, email FROM users WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(user)
    }
    pub async fn get_by_username(&self, username: String) -> Result<Option<User>, sqlx::Error> {
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
        email: String,
        pass_hash: String,
    ) -> Result<User, sqlx::Error> {
        let user = sqlx::query_as::<_, User>("INSERT INTO users (username, email, pass_hash, created_at) VALUES ($1, $2, $3, $4) RETURNING id, username, email, pass_hash, created_at")
            .bind(username)
            .bind(email)
            .bind(pass_hash)
            .bind(Utc::now())
            .fetch_one(&self.pool)
            .await?;

        Ok(user)
    }
    pub async fn delete_by_id(&self, id: uuid::Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn update(&self, user: User) -> Result<User, sqlx::Error> {
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
