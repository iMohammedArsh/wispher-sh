//! Optional AI polish: one HTTP call to the user's chosen provider, with a hard
//! timeout. Any failure falls back to the locally cleaned text.

pub mod prompts;
pub mod styles;

use std::time::Duration;

use futures_util::future::BoxFuture;
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::settings::{AiProvider, AiSettings};

#[derive(Debug, Clone)]
pub enum Task {
    /// Clean up a dictation.
    Polish { transcript: String },
    /// Command mode: apply a spoken instruction to the selection (or write new text).
    Edit { instruction: String, selection: Option<String> },
    /// Notes mode: title and key points for a long transcript.
    Summarize { transcript: String },
}

impl Task {
    /// Summaries read a long transcript, so they get more time than a paste-path polish.
    fn min_timeout(&self) -> Duration {
        match self {
            Task::Summarize { .. } => Duration::from_secs(60),
            _ => Duration::ZERO,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Request {
    pub task: Task,
    pub style_prompt: String,
    pub vocabulary: Vec<String>,
}

pub trait Polisher: Send + Sync {
    fn enabled(&self) -> bool;
    fn run(&self, request: Request) -> BoxFuture<'static, AppResult<String>>;
}

/// Talks to Gemini, Anthropic, or anything OpenAI-compatible.
pub struct LlmClient {
    settings: AiSettings,
    api_key: Option<String>,
    http: reqwest::Client,
}

impl LlmClient {
    pub fn new(settings: AiSettings, api_key: Option<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(settings.timeout_ms))
            .connect_timeout(Duration::from_secs(4))
            .build()
            .unwrap_or_default();
        Self { settings, api_key, http }
    }

    pub fn default_model(provider: AiProvider) -> &'static str {
        match provider {
            AiProvider::Gemini => "gemini-3.1-flash-lite",
            AiProvider::OpenAi => "gpt-4.1-mini",
            AiProvider::Groq => "llama-3.3-70b-versatile",
            AiProvider::OpenRouter => "openai/gpt-4.1-mini",
            AiProvider::Anthropic => "claude-haiku-5-5",
            AiProvider::Ollama => "llama3.2",
            AiProvider::Custom | AiProvider::Off => "",
        }
    }

    fn model(&self) -> String {
        if self.settings.model.trim().is_empty() {
            Self::default_model(self.settings.provider).to_string()
        } else {
            self.settings.model.trim().to_string()
        }
    }

    fn openai_base(&self) -> String {
        let custom = self.settings.base_url.trim().trim_end_matches('/');
        match self.settings.provider {
            AiProvider::OpenAi => "https://api.openai.com/v1".into(),
            AiProvider::Groq => "https://api.groq.com/openai/v1".into(),
            AiProvider::OpenRouter => "https://openrouter.ai/api/v1".into(),
            AiProvider::Ollama if custom.is_empty() => "http://127.0.0.1:11434/v1".into(),
            _ => custom.to_string(),
        }
    }

    async fn call(self: std::sync::Arc<Self>, system: String, user: String, min_timeout: Duration) -> AppResult<String> {
        let provider = self.settings.provider;
        let key = self.api_key.clone().unwrap_or_default();
        if provider.needs_key() && key.is_empty() {
            return Err(AppError::Polish("no API key saved for this provider".into()));
        }
        let model = self.model();
        if model.is_empty() {
            return Err(AppError::Polish("no model configured".into()));
        }
        let request = match provider {
            AiProvider::Off => return Err(AppError::Polish("AI polish is off".into())),
            AiProvider::Gemini => self
                .http
                .post(format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"))
                .header("x-goog-api-key", key)
                .json(&json!({
                    "systemInstruction": { "parts": [{ "text": system }] },
                    "contents": [{ "role": "user", "parts": [{ "text": user }] }],
                    "generationConfig": { "temperature": 0.1 },
                })),
            AiProvider::Anthropic => self
                .http
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", key)
                .header("anthropic-version", "2023-06-01")
                .json(&json!({
                    "model": model,
                    "max_tokens": 2048,
                    "temperature": 0.1,
                    "system": system,
                    "messages": [{ "role": "user", "content": user }],
                })),
            _ => {
                let base = self.openai_base();
                if base.is_empty() {
                    return Err(AppError::Polish("no endpoint URL configured".into()));
                }
                let mut builder = self.http.post(format!("{base}/chat/completions")).json(&json!({
                    "model": model,
                    "temperature": 0.1,
                    "messages": [
                        { "role": "system", "content": system },
                        { "role": "user", "content": user },
                    ],
                }));
                if !key.is_empty() {
                    builder = builder.bearer_auth(key);
                }
                builder
            }
        };

        let timeout = Duration::from_millis(self.settings.timeout_ms).max(min_timeout);
        let response = request.timeout(timeout).send().await.map_err(|e| {
            // Never echo the URL: some providers put credentials there.
            AppError::Polish(if e.is_timeout() { "timed out".into() } else { "could not reach the AI provider".into() })
        })?;
        let status = response.status();
        let body: Value = response.json().await.unwrap_or(Value::Null);
        if !status.is_success() {
            let message = body
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("request failed");
            return Err(AppError::Polish(format!("{status}: {}", truncate(message, 160))));
        }
        let text = match provider {
            AiProvider::Gemini => body
                .pointer("/candidates/0/content/parts")
                .and_then(Value::as_array)
                .map(|parts| parts.iter().filter_map(|p| p.get("text").and_then(Value::as_str)).collect::<String>()),
            AiProvider::Anthropic => body
                .get("content")
                .and_then(Value::as_array)
                .map(|blocks| blocks.iter().filter_map(|b| b.get("text").and_then(Value::as_str)).collect::<String>()),
            _ => body.pointer("/choices/0/message/content").and_then(Value::as_str).map(str::to_string),
        };
        let text = text.map(|t| prompts::strip_wrapping(&t)).unwrap_or_default();
        if text.is_empty() {
            return Err(AppError::Polish("the AI returned no text".into()));
        }
        Ok(text)
    }
}

impl Polisher for std::sync::Arc<LlmClient> {
    fn enabled(&self) -> bool {
        self.settings.provider != AiProvider::Off
    }

    fn run(&self, request: Request) -> BoxFuture<'static, AppResult<String>> {
        let (system, user) = prompts::build(&request);
        let client = std::sync::Arc::clone(self);
        Box::pin(client.call(system, user, request.task.min_timeout()))
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        format!("{}…", text.chars().take(max).collect::<String>())
    }
}
