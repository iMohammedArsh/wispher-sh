use std::fs;
use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Owns the bundled loopback-only `whisper-server` process.
///
/// Unlike the legacy CLI path, the server keeps the selected model in memory
/// between recordings. It never listens beyond 127.0.0.1 and has no cloud path.
pub struct WarmWhisperServer {
    child: Option<Child>,
    endpoint: Option<String>,
    model_path: Option<String>,
}

impl Default for WarmWhisperServer {
    fn default() -> Self {
        Self {
            child: None,
            endpoint: None,
            model_path: None,
        }
    }
}

impl WarmWhisperServer {
    pub fn ensure_started(&mut self, model_path: &str, cli_path: &str) -> Result<String, String> {
        let still_running = self
            .child
            .as_mut()
            .is_some_and(|child| matches!(child.try_wait(), Ok(None)));
        if still_running && self.model_path.as_deref() == Some(model_path) {
            if let Some(endpoint) = &self.endpoint {
                return Ok(endpoint.clone());
            }
        }

        self.stop();
        if !Path::new(model_path).is_file() {
            return Err(format!("Whisper model was not found at {model_path}."));
        }

        let server_path = server_binary_path(cli_path)?;
        let port = available_loopback_port()?;
        let endpoint = format!("http://127.0.0.1:{port}/inference");
        let child = Command::new(&server_path)
            .arg("-m")
            .arg(model_path)
            .arg("-t")
            .arg("4")
            .arg("-sns")
            .arg("-nt")
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("Could not start local Whisper server: {error}"))?;

        self.child = Some(child);
        self.endpoint = Some(endpoint.clone());
        self.model_path = Some(model_path.to_owned());
        println!("[Local Whisper] Warm server starting on 127.0.0.1:{port}");
        Ok(endpoint)
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.endpoint = None;
        self.model_path = None;
    }

    pub async fn transcribe(
        endpoint: &str,
        wav_bytes: &[u8],
        language: &str,
        custom_dictionary: &str,
    ) -> Result<String, String> {
        let language = match language {
            "en" | "ml" => language,
            _ => "auto",
        };
        let mut prompt = String::from(
            "Accurate voice dictation. Preserve short English terms exactly, including Alt, Ctrl, Shift, Space, yes, and no. For Malayalam speech, write Malayalam script exactly.",
        );
        if !custom_dictionary.trim().is_empty() {
            prompt.push_str(" Vocabulary: ");
            prompt.push_str(custom_dictionary);
        }

        // The server takes a brief moment to become reachable while it warms the
        // model. Retrying is local-only and happens before the legacy fallback.
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|error| format!("Could not create the local Whisper client: {error}"))?;
        let mut last_error = String::new();
        for attempt in 0..12 {
            let form = reqwest::multipart::Form::new()
                .part(
                    "file",
                    reqwest::multipart::Part::bytes(wav_bytes.to_vec())
                        .file_name("dictation.wav")
                        .mime_str("audio/wav")
                        .map_err(|error| format!("Could not prepare local audio: {error}"))?,
                )
                .text("response_format", "text")
                .text("language", language.to_owned())
                .text("temperature", "0.0")
                .text("prompt", prompt.clone());
            match client.post(endpoint).multipart(form).send().await {
                Ok(response) if response.status().is_success() => {
                    let text = response
                        .text()
                        .await
                        .map_err(|error| format!("Could not read local Whisper output: {error}"))?;
                    let text = sanitize_transcript(&text);
                    if !text.is_empty() {
                        return Ok(text);
                    }
                    last_error = "The warm Whisper server returned no spoken words.".into();
                }
                Ok(response) => {
                    last_error = format!("Warm Whisper server returned {}.", response.status());
                }
                Err(error) => last_error = error.to_string(),
            }
            if attempt < 11 {
                tokio::time::sleep(Duration::from_millis(180)).await;
            }
        }
        Err(format!(
            "Warm local Whisper server was unavailable: {last_error}"
        ))
    }
}

impl Drop for WarmWhisperServer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn server_binary_path(cli_path: &str) -> Result<std::path::PathBuf, String> {
    let cli = Path::new(cli_path);
    let file_name = if cfg!(target_os = "windows") {
        "whisper-server.exe"
    } else {
        "whisper-server"
    };
    let server = cli.with_file_name(file_name);
    if server.is_file() {
        Ok(server)
    } else {
        Err(format!(
            "Whisper server executable was not found at {}.",
            server.display()
        ))
    }
}

fn available_loopback_port() -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("Could not reserve a local Whisper port: {error}"))?;
    listener
        .local_addr()
        .map(|address| address.port())
        .map_err(|error| format!("Could not read the local Whisper port: {error}"))
}

/// Local whisper.cpp transcription service interface.
pub struct WhisperTranscriber {
    model_path: String,
    binary_path: String,
    language: String,
    custom_dictionary: String,
    vad_model_path: Option<std::path::PathBuf>,
}

impl WhisperTranscriber {
    pub fn new(
        model_path: impl Into<String>,
        binary_path: impl Into<String>,
        language: impl Into<String>,
        custom_dictionary: impl Into<String>,
        vad_model_path: Option<std::path::PathBuf>,
    ) -> Self {
        Self {
            model_path: model_path.into(),
            binary_path: binary_path.into(),
            language: language.into(),
            custom_dictionary: custom_dictionary.into(),
            vad_model_path,
        }
    }

    pub fn transcribe(&self, wav_bytes: &[u8]) -> Result<String, String> {
        if !Path::new(&self.model_path).is_file() {
            return Err(format!(
                "Whisper model was not found at {}.",
                self.model_path
            ));
        }
        if !Path::new(&self.binary_path).is_file() && !command_exists(&self.binary_path) {
            return Err(format!(
                "Whisper executable was not found at {}.",
                self.binary_path
            ));
        }

        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temp_wav_path = std::env::temp_dir().join(format!(
            "wisper-dictation-{}-{suffix}.wav",
            std::process::id()
        ));
        fs::write(&temp_wav_path, wav_bytes)
            .map_err(|e| format!("Could not create temporary audio file: {e}"))?;

        let language = match self.language.as_str() {
            "en" | "ml" => self.language.as_str(),
            _ => "auto",
        };
        let mut prompt = String::from(
            "Accurate voice dictation. Preserve short English terms exactly, including Alt, Ctrl, Shift, Space, yes, and no. For Malayalam speech, write Malayalam script exactly.",
        );
        if !self.custom_dictionary.trim().is_empty() {
            prompt.push_str(" Vocabulary: ");
            prompt.push_str(&self.custom_dictionary);
        }
        let mut command = Command::new(&self.binary_path);
        command
            .arg("-m")
            .arg(&self.model_path)
            .arg("-f")
            .arg(&temp_wav_path)
            .arg("-l")
            .arg(language)
            .arg("-t")
            .arg("4")
            .arg("-sns")
            .arg("--prompt")
            .arg(prompt)
            .arg("-nt");
        if let Some(vad_model_path) = &self.vad_model_path {
            command
                .arg("--vad")
                .arg("-vm")
                .arg(vad_model_path)
                .arg("-vt")
                .arg("0.28")
                .arg("-vspd")
                .arg("120")
                .arg("-vsd")
                .arg("100")
                .arg("-vp")
                .arg("100");
        }
        let result = command
            .output()
            .map_err(|e| format!("Could not execute whisper.cpp: {e}"));
        let _ = fs::remove_file(&temp_wav_path);
        let output = result?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            return Err(format!(
                "whisper.cpp exited with {}. {}",
                output.status,
                concise_log(&stderr)
            ));
        }
        let text = sanitize_transcript(&stdout);
        if text.is_empty() {
            return Err(format!(
                "Whisper received the audio but found no spoken words. {}",
                concise_log(&stderr)
            ));
        }
        Ok(text)
    }
}

fn concise_log(log: &str) -> String {
    let cleaned = log.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.is_empty() {
        "No additional whisper.cpp output was provided.".into()
    } else {
        cleaned.chars().take(260).collect()
    }
}

/// Strip whisper.cpp's timestamps and non-speech annotations while retaining actual words.
fn sanitize_transcript(text: &str) -> String {
    text.lines()
        .filter_map(|line| line.rsplit_once(']').map(|(_, words)| words).or(Some(line)))
        .map(|line| line.trim())
        .filter(|line| !line.starts_with('[') && !line.starts_with('(') && !line.starts_with('*'))
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn command_exists(command: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|path| path.join(command).is_file()))
        .unwrap_or(false)
}
