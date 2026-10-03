use anyhow::anyhow;
use serde_json::{Value, json};

use crate::error::AppError;

pub fn piston_language(lang: &str) -> Option<(&'static str, &'static str)> {
    Some(match lang {
        "python" => ("python", "main.py"),
        "cpp" => ("c++", "main.cpp"),
        "c" => ("c", "main.c"),
        "java" => ("java", "Main.java"),
        "rust" => ("rust", "main.rs"),
        "javascript" => ("javascript", "main.js"),
        "typescript" => ("typescript", "main.ts"),
        "csharp" => ("dotnet", "Main.cs"),
        "go" => ("go", "main.go"),
        "kotlin" => ("kotlin", "Main.kt"),
        "swift" => ("swift", "main.swift"),
        "ruby" => ("ruby", "main.rb"),
        "php" => ("php", "main.php"),
        _ => return None,
    })
}

#[derive(Debug)]
pub enum ExecOutcome {
    CompileError(String),
    Timeout,
    Finished {
        stdout: String,
        stderr: String,
        exit_code: i64,
    },
}

pub struct PistonService {
    client: reqwest::Client,
    base_url: String,
}

impl PistonService {
    pub fn new(base_url: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("failed to build reqwest client");
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn execute(
        &self,
        language: &str,
        file_name: &str,
        code: &str,
        stdin: &str,
    ) -> Result<ExecOutcome, AppError> {
        let body = json!({
            "language": language,
            "version": "*",
            "files": [{ "name": file_name, "content": code }],
            "stdin": stdin,
        });

        let response = self
            .client
            .post(format!("{}/api/v2/execute", self.base_url))
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let value: Value = response.json().await?;

        if !status.is_success() {
            let msg = value
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown error");
            return Err(AppError::Internal(anyhow!("piston {status}: {msg}")));
        }

        if let Some(compile) = value.get("compile") {
            let code = compile.get("code").and_then(Value::as_i64).unwrap_or(0);
            let signal = compile.get("signal").and_then(Value::as_str);
            if code != 0 || signal.is_some() {
                let output = compile
                    .get("output")
                    .and_then(Value::as_str)
                    .or_else(|| compile.get("stderr").and_then(Value::as_str))
                    .unwrap_or_default();
                return Ok(ExecOutcome::CompileError(output.to_string()));
            }
        }

        let run = value
            .get("run")
            .ok_or_else(|| AppError::Internal(anyhow!("piston: в ответе нет поля run")))?;

        let signal = run.get("signal").and_then(Value::as_str);
        let run_status = run.get("status").and_then(Value::as_str);
        if run_status == Some("TO") || signal == Some("SIGKILL") {
            return Ok(ExecOutcome::Timeout);
        }

        Ok(ExecOutcome::Finished {
            stdout: run
                .get("stdout")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            stderr: run
                .get("stderr")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            exit_code: run.get("code").and_then(Value::as_i64).unwrap_or(-1),
        })
    }
}
