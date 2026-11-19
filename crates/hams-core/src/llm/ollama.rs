//! Asking a local Ollama what it has installed, so Settings can say "Ollama is
//! running, with these models" instead of leaving a wrong name to fail later.

use std::time::Duration;

use serde_json::Value;

use crate::error::{AppError, AppResult};

/// Where Ollama listens unless Settings says otherwise (its OpenAI-compatible
/// path is this plus `/v1`; the model list is on the root).
pub const DEFAULT_BASE: &str = "http://127.0.0.1:11434/v1";

/// The server root for a configured base URL: `http://host:11434/v1/` → `http://host:11434`.
fn root(base_url: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    let base = if base.is_empty() { DEFAULT_BASE } else { base };
    base.strip_suffix("/v1").unwrap_or(base).to_string()
}

/// The names of the models Ollama has downloaded (`llama3.2:latest`, …).
/// Fails with a readable reason when Ollama isn't running or isn't Ollama.
pub async fn installed_models(base_url: &str) -> AppResult<Vec<String>> {
    let url = format!("{}/api/tags", root(base_url));
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AppError::Polish("the Ollama address should start with http:// or https://".into()));
    }
    let not_running = |why: String| {
        AppError::Polish(format!("Ollama isn't answering at {}. Install it from ollama.com and start it ({why}).", root(base_url)))
    };
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|e| AppError::Polish(e.to_string()))?
        .get(&url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| not_running(e.to_string()))?;
    let body: Value = response.json().await.map_err(|_| AppError::Polish("that address answered, but it isn't Ollama".into()))?;
    let models = body
        .get("models")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::Polish("that address answered, but it isn't Ollama".into()))?;
    Ok(models.iter().filter_map(|m| m.get("name").and_then(Value::as_str)).map(str::to_string).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[test]
    fn the_server_root_is_found_from_any_way_of_writing_the_base_url() {
        assert_eq!(root(""), "http://127.0.0.1:11434");
        assert_eq!(root("http://127.0.0.1:11434/v1"), "http://127.0.0.1:11434");
        assert_eq!(root(" http://box.local:11434/v1/ "), "http://box.local:11434");
        assert_eq!(root("http://box.local:11434"), "http://box.local:11434");
    }

    /// A one-shot HTTP server that answers every request with `body`.
    async fn serve(body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else { return };
                tokio::spawn(async move {
                    let mut buf = [0u8; 1024];
                    let _ = socket.read(&mut buf).await;
                    let head = format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n", body.len());
                    let _ = socket.write_all(head.as_bytes()).await;
                    let _ = socket.write_all(body.as_bytes()).await;
                });
            }
        });
        format!("http://{address}/v1")
    }

    #[tokio::test]
    async fn the_installed_models_are_listed() {
        let base = serve(r#"{"models":[{"name":"llama3.2:latest","size":1},{"name":"qwen2.5:7b"}]}"#).await;
        assert_eq!(installed_models(&base).await.unwrap(), vec!["llama3.2:latest", "qwen2.5:7b"]);
    }

    #[tokio::test]
    async fn an_ollama_with_nothing_installed_gives_an_empty_list() {
        let base = serve(r#"{"models":[]}"#).await;
        assert!(installed_models(&base).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn something_that_is_not_ollama_is_recognised() {
        let base = serve(r#"{"hello":"world"}"#).await;
        let error = installed_models(&base).await.unwrap_err();
        assert!(error.to_string().contains("isn't Ollama"), "{error}");
    }

    #[tokio::test]
    async fn nothing_listening_says_how_to_get_ollama() {
        // Bind and drop to get a port nothing is listening on.
        let port = TcpListener::bind("127.0.0.1:0").await.unwrap().local_addr().unwrap().port();
        let error = installed_models(&format!("http://127.0.0.1:{port}/v1")).await.unwrap_err();
        let message = error.to_string();
        assert!(message.contains("ollama.com") && message.contains(&port.to_string()), "{message}");
    }

    #[tokio::test]
    async fn a_non_http_address_is_refused_before_any_request() {
        let error = installed_models("file:///etc/passwd").await.unwrap_err();
        assert!(error.to_string().contains("http://"), "{error}");
    }
}
