use std::sync::Arc;

use anyhow::anyhow;
use futures_util::{StreamExt, future::join_all};
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::{
    error::AppError,
    models::problems::TestCase,
    service::piston_service::{ExecOutcome, PistonService},
};

const OLLAMA_CHAT_URL: &str = "http://127.0.0.1:11434/api/chat";

const MAX_TESTS: usize = 10;
const EXAMPLES_IN_STATEMENT: usize = 3;

const MIN_TESTS: usize = 3;
const TESTS_ATTEMPTS: usize = 3;
const OLLAMA_MODEL: &str = "qwen2.5:7b";

const MAX_POOL_INPUTS: usize = 14;

const TESTS_PROMPT: &str = "Тебе дано условие задачи. Напиши ОДНУ эталонную программу на Python 3, которая её решает.
Требования к программе:
- только стандартная библиотека Python (sys, math, collections, itertools, heapq, bisect и т.п.); никаких внешних пакетов (numpy, pandas, scipy, networkx и других);
- простой алгоритм без классов; не используй вложенные функции, которые меняют переменные внешней функции; если нужна рекурсия, в начале вызови sys.setrecursionlimit(10000);
- читай stdin через sys.stdin.read() или input() и печатай ТОЛЬКО ответ в stdout, без подсказок вроде 'Enter number';
- ответ однозначный: для одного входа всегда один и тот же вывод.
Затем придумай от 5 до 10 тестовых входов (inputs): небольших, строго в рамках ограничений из условия, включая один крайний случай. Каждый вход — это полное содержимое stdin одного теста (строки разделены \\n).
ВАЖНО: входы должны строго соответствовать формату, который читает твоя программа, иначе она упадёт с ошибкой (EOFError).
Отвечай только JSON с полями \"solution\" (код) и \"inputs\" (массив строк).";

fn difficulty_hint(level: i32) -> &'static str {
    match level {
        i32::MIN..=3 => {
            "Это ЛЁГКАЯ задача: одна идея, решается циклом и парой условий за 5-15 строк. \
             Темы: арифметика, строки, массивы, подсчёт, сортировка, простые условия. \
             ЗАПРЕЩЕНЫ графы, деревья, динамическое программирование, рекурсия, геометрия. \
             Вход: одна или две строки (числа или короткая строка), не больше 20 элементов."
        }
        4..=6 => {
            "Это задача СРЕДНЕЙ сложности: словари и множества, двойные циклы, префиксные суммы, \
             двоичный поиск, простой жадный алгоритм или динамика за один проход. \
             Графы и деревья не используй. Вход: не больше 100 элементов."
        }
        _ => {
            "Это задача ПОВЫШЕННОЙ сложности: можно использовать обход графа на небольшом входе, \
             динамическое программирование или жадный алгоритм. Ответ: одно число или одна строка. \
             Вход: не больше 100 элементов."
        }
    }
}

pub enum ProblemEvent {
    Chunk(String),
    Verifying,
}

struct TestDraft {
    solution: String,
    inputs: Vec<String>,
}

pub struct GeneratedProblem {
    pub name: String,
    pub text: String,
    pub tests: Vec<TestCase>,
}

pub struct ProblemService {
    piston: Arc<PistonService>,
}

impl ProblemService {
    pub fn new(piston: Arc<PistonService>) -> Self {
        Self { piston }
    }

    async fn build_request(level: i32, avoid: &[String]) -> Result<Value, AppError> {
        let request_json = tokio::fs::read_to_string("request_ollama.json")
            .await
            .map_err(|e| AppError::Internal(e.into()))?;

        let mut body: Value = serde_json::from_str(&request_json)?;

        if let Some(map) = body.as_object_mut() {
            map.remove("format");
        }

        let user_content = body.pointer_mut("/messages/1/content").ok_or_else(|| {
            AppError::BadRequest(
                "В request_ollama.json отсутствует messages[1].content".to_string(),
            )
        })?;

        let mut prompt = format!(
            "Сгенерируй задачу уровня {} из 10. {}",
            level,
            difficulty_hint(level)
        );
        if !avoid.is_empty() {
            prompt.push_str(&format!(
                ". Не повторяй уже использованные задачи: {}",
                avoid.join("; ")
            ));
        }
        *user_content = json!(prompt);

        Ok(body)
    }

    pub async fn generate(
        &self,
        level: i32,
        avoid: &[String],
        tx: mpsc::UnboundedSender<ProblemEvent>,
    ) -> Result<GeneratedProblem, AppError> {
        let mut body = Self::build_request(level, avoid).await?;
        body["stream"] = json!(true);

        let response = reqwest::Client::new()
            .post(OLLAMA_CHAT_URL)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        let mut stream = response.bytes_stream();
        let mut line_buf: Vec<u8> = Vec::new();

        let mut statement_md = String::new();

        'read: while let Some(chunk) = stream.next().await {
            line_buf.extend_from_slice(&chunk?);

            while let Some(pos) = line_buf.iter().position(|b| *b == b'\n') {
                let raw: Vec<u8> = line_buf.drain(..=pos).collect();
                let line = String::from_utf8_lossy(&raw);
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                let value: Value = match serde_json::from_str(line) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!("Failed to parse stream line: {} | Line: {}", e, line);
                        continue;
                    }
                };

                if let Some(err) = value.get("error").and_then(Value::as_str) {
                    return Err(AppError::Internal(anyhow!("Ollama error: {err}")));
                }

                if let Some(fragment) = value.pointer("/message/content").and_then(Value::as_str) {
                    statement_md.push_str(fragment);

                    if !fragment.is_empty()
                        && tx.send(ProblemEvent::Chunk(fragment.to_string())).is_err()
                    {
                        return Err(AppError::Internal(anyhow!("stream receiver dropped")));
                    }
                }

                if value.get("done").and_then(Value::as_bool).unwrap_or(false) {
                    break 'read;
                }
            }
        }

        let statement_md = statement_md.trim().to_string();
        if statement_md.is_empty() {
            return Err(AppError::Internal(anyhow!("Модель вернула пустой ответ")));
        }

        let task_name = statement_md
            .lines()
            .find(|line| line.trim_start().starts_with("# "))
            .map(|line| line.trim_start()[2..].trim().to_string())
            .unwrap_or_else(|| "Без названия".to_string());

        let _ = tx.send(ProblemEvent::Verifying);

        let mut tests = Vec::new();
        for attempt in 1..=TESTS_ATTEMPTS {
            match self.build_tests(&statement_md).await {
                Ok(t) if t.len() >= MIN_TESTS => {
                    tests = t;
                    break;
                }
                Ok(t) => tracing::warn!(
                    "попытка {attempt}: валидных тестов {} (нужно ≥ {MIN_TESTS})",
                    t.len()
                ),
                Err(e) => tracing::warn!("попытка {attempt}: {e:?}"),
            }
        }

        if tests.is_empty() {
            return Err(AppError::Internal(anyhow!(
                "не удалось сгенерировать валидные тесты"
            )));
        }

        let examples = examples_markdown(&tests);
        let _ = tx.send(ProblemEvent::Chunk(examples.clone()));

        Ok(GeneratedProblem {
            name: task_name,
            text: format!("{}{}", statement_md, examples),
            tests,
        })
    }

    async fn run_all(&self, solution: &str, inputs: &[String]) -> Vec<Option<String>> {
        join_all(inputs.iter().map(|input| async move {
            let stdin = format!("{}\n", input.trim_end());
            match self
                .piston
                .execute("python", "main.py", solution, &stdin)
                .await
            {
                Ok(ExecOutcome::Finished {
                    stdout,
                    exit_code: 0,
                    ..
                }) => Some(normalize_output(&stdout)).filter(|o| !o.is_empty()),
                Ok(ExecOutcome::Finished {
                    exit_code, stderr, ..
                }) => {
                    tracing::warn!(
                        "Python script failed (code {}). Stderr:\n{}",
                        exit_code,
                        stderr
                    );
                    None
                }
                Err(e) => {
                    tracing::warn!("Piston execution error: {:?}", e);
                    None
                }
                _ => None,
            }
        }))
        .await
    }

    async fn build_tests(&self, statement_md: &str) -> Result<Vec<TestCase>, AppError> {
        let draft = ask_tests(statement_md, 0.2).await?;
        let check = ask_tests(statement_md, 0.8).await?;

        let mut inputs: Vec<String> = Vec::new();
        for raw in draft.inputs.iter().chain(check.inputs.iter()) {
            let input = raw.trim_end().to_string();
            if !input.is_empty() && !inputs.contains(&input) {
                inputs.push(input);
            }
        }
        inputs.truncate(MAX_POOL_INPUTS);

        let (out_a, out_b) = tokio::join!(
            self.run_all(&draft.solution, &inputs),
            self.run_all(&check.solution, &inputs)
        );

        let mut tests: Vec<TestCase> = Vec::new();
        let mut disagreements = 0usize;
        for ((input, a), b) in inputs.into_iter().zip(out_a).zip(out_b) {
            match (a, b) {
                (Some(a), Some(b)) if a == b => {
                    tests.push(TestCase { input, output: a });
                    if tests.len() >= MAX_TESTS {
                        break;
                    }
                }
                (Some(_), Some(_)) => disagreements += 1,
                _ => {}
            }
        }

        if tests.len() < MIN_TESTS {
            tracing::warn!(
                "Подтверждено тестов: {} (расхождений между решениями: {}).\nРешение A:\n{}\n\nРешение B:\n{}",
                tests.len(),
                disagreements,
                draft.solution,
                check.solution
            );
        }

        Ok(tests)
    }
}

async fn ask_tests(statement_md: &str, temperature: f64) -> Result<TestDraft, AppError> {
    let body = json!({
        "model": OLLAMA_MODEL,
        "stream": false,
        "options": { "temperature": temperature },
        "format": {
            "type": "object",
            "properties": {
                "solution": { "type": "string" },
                "inputs": { "type": "array", "items": { "type": "string" } }
            },
            "required": ["solution", "inputs"],
            "additionalProperties": false
        },
        "messages": [
            { "role": "system", "content": TESTS_PROMPT },
            { "role": "user", "content": statement_md }
        ]
    });

    let resp: Value = reqwest::Client::new()
        .post(OLLAMA_CHAT_URL)
        .json(&body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let content = resp
        .pointer("/message/content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Internal(anyhow!("пустой ответ Ollama")))?;

    let v: Value = serde_json::from_str(content)?;

    let solution = v
        .get("solution")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Internal(anyhow!("в ответе нет solution")))?
        .to_string();

    let inputs = v
        .get("inputs")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    Ok(TestDraft { solution, inputs })
}

pub fn normalize_output(s: &str) -> String {
    s.replace("\r\n", "\n")
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
}

fn examples_markdown(tests: &[TestCase]) -> String {
    let mut out = String::from("\n\n## Примеры");
    for (i, t) in tests.iter().take(EXAMPLES_IN_STATEMENT).enumerate() {
        out.push_str(&format!(
            "\n\n### Пример {}\n\n**Входные данные**\n\n```\n{}\n```\n\n**Выходные данные**\n\n```\n{}\n```",
            i + 1,
            t.input,
            t.output
        ));
    }
    out
}
