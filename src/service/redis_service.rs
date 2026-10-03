use uuid::Uuid;

use crate::error::AppError;
use crate::models::problems::TestCase;

const TESTS_TTL_SECONDS: u64 = 6 * 60 * 60;

pub struct RedisService {
    pool: r2d2::Pool<redis::Client>,
}

fn tests_key(battle_id: Uuid, round: u32) -> String {
    format!("battle:{battle_id}:round:{round}:tests")
}

impl RedisService {
    pub fn new(url: &str) -> Result<Self, AppError> {
        let client = redis::Client::open(url)?;
        let pool = r2d2::Pool::builder().max_size(8).build(client)?;
        Ok(Self { pool })
    }

    pub async fn save_tests(
        &self,
        battle_id: Uuid,
        round: u32,
        tests: &[TestCase],
    ) -> Result<(), AppError> {
        let key = tests_key(battle_id, round);
        let json = serde_json::to_string(tests)?;
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<(), AppError> {
            let mut conn = pool.get()?;
            redis::cmd("SETEX")
                .arg(&key)
                .arg(TESTS_TTL_SECONDS)
                .arg(json)
                .query::<()>(&mut *conn)?;
            Ok(())
        })
        .await
        .map_err(|e| AppError::Internal(e.into()))?
    }

    pub async fn load_tests(
        &self,
        battle_id: Uuid,
        round: u32,
    ) -> Result<Option<Vec<TestCase>>, AppError> {
        let key = tests_key(battle_id, round);
        let pool = self.pool.clone();

        let raw = tokio::task::spawn_blocking(move || -> Result<Option<String>, AppError> {
            let mut conn = pool.get()?;
            let value = redis::cmd("GET")
                .arg(&key)
                .query::<Option<String>>(&mut *conn)?;
            Ok(value)
        })
        .await
        .map_err(|e| AppError::Internal(e.into()))??;

        match raw {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    pub async fn delete_tests(&self, battle_id: Uuid, round: u32) -> Result<(), AppError> {
        let key = tests_key(battle_id, round);
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || -> Result<(), AppError> {
            let mut conn = pool.get()?;
            redis::cmd("DEL").arg(&key).query::<()>(&mut *conn)?;
            Ok(())
        })
        .await
        .map_err(|e| AppError::Internal(e.into()))?
    }
}
