use anyhow::anyhow;
use serde_json::{Value, json};

use crate::{
    error::AppError,
    models::dto::problem_dto::{ProblemCreateRequest, ProblemResponse},
};

pub struct ProblemService {}

impl ProblemService {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn generate_problem(
        &self,
        dto: ProblemCreateRequest,
    ) -> Result<ProblemResponse, AppError> {
        let client = reqwest::Client::new();
        let request_json = tokio::fs::read_to_string("request_ollama.json")
            .await
            .map_err(|e| AppError::Internal(e.into()))?;

        let mut body: Value =
            serde_json::from_str(&request_json).map_err(|e| AppError::ParseJson(e))?;

        let user_content = body.pointer_mut("/messages/1/content").ok_or_else(|| {
            AppError::BadRequest(
                "В request_ollama.json отсутствует messages[1].content".to_string(),
            )
        })?;

        *user_content = json!(format!("Сгенерируй задачу уровня {}", dto.level));

        let response: Value = client
            .post("http://127.0.0.1:11434/api/chat")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let content = response
            .get("message")
            .and_then(|message| message.get("content"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AppError::Internal(anyhow!(
                    "Ollama response does not contain message.content: {response}"
                ))
            })?;

        let task: Value = serde_json::from_str(content)?;

        tracing::debug!(
            task = %serde_json::to_string(&task)?,
            "generated task"
        );

        Ok(ProblemResponse {
            text: serde_json::to_string(&task)?,
        })
    }
}
