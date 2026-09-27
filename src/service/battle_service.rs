use crate::error::AppError;
use axum::extract::Multipart;
use tokio::fs;

pub struct BattleService;

impl BattleService {
    pub fn new() -> Self {
        Self {}
    }

    //file name = (user_uuid)_(problem_number).(py)
    pub async fn upload_file(&self, multipart: &mut Multipart) -> Result<(), AppError> {
        fs::create_dir_all("upload_files")
            .await
            .map_err(|e| AppError::Internal(e.into()));

        while let Some(mut filed) = multipart
            .next_field()
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?
        {
            let filed_name = filed.name().unwrap_or_default();

            if filed_name != "file" {
                continue;
            }

            let original_name = filed.file_name().unwrap_or("unknown.bin").to_string();

            let save_name = format("{}")
        }

        Err(AppError::BadRequest("filed file not found".to_string()))
    }
}
