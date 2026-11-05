//! Whisper model catalog and downloader. Models live in the app data folder and
//! are downloaded on demand, so the installer stays small.

use std::path::PathBuf;

use futures_util::StreamExt;
use serde::Serialize;
use tokio::io::AsyncWriteExt;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ModelInfo {
    pub id: &'static str,
    pub file: &'static str,
    pub label: &'static str,
    pub size_mb: u32,
    pub multilingual: bool,
    pub note: &'static str,
}

pub const CATALOG: &[ModelInfo] = &[
    ModelInfo {
        id: "base",
        file: "ggml-base.bin",
        label: "Base",
        size_mb: 142,
        multilingual: true,
        note: "Fastest. Good for clear English.",
    },
    ModelInfo {
        id: "small",
        file: "ggml-small.bin",
        label: "Small",
        size_mb: 466,
        multilingual: true,
        note: "Recommended balance. Handles Malayalam and mixed speech.",
    },
    ModelInfo {
        id: "large-v3-turbo",
        file: "ggml-large-v3-turbo-q5_0.bin",
        label: "Large v3 Turbo",
        size_mb: 547,
        multilingual: true,
        note: "Most accurate. Needs Apple Silicon or a fast CPU.",
    },
];

const BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

pub fn info(id: &str) -> Option<&'static ModelInfo> {
    CATALOG.iter().find(|m| m.id == id)
}

pub fn path_for(id: &str) -> Option<PathBuf> {
    info(id).map(|m| crate::paths::models_dir().join(m.file))
}

pub fn is_installed(id: &str) -> bool {
    path_for(id).is_some_and(|p| p.is_file())
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelStatus {
    #[serde(flatten)]
    pub info: ModelInfo,
    pub installed: bool,
}

pub fn statuses() -> Vec<ModelStatus> {
    CATALOG
        .iter()
        .map(|m| ModelStatus { info: *m, installed: is_installed(m.id) })
        .collect()
}

/// Download a model, reporting progress in 0..1. Writes to `<file>.part` and
/// renames on success, so a cancelled download never looks installed.
pub async fn download(id: &str, mut progress: impl FnMut(f32)) -> AppResult<PathBuf> {
    let model = info(id).ok_or_else(|| AppError::Model(format!("unknown model \"{id}\"")))?;
    let target = crate::paths::models_dir().join(model.file);
    if target.is_file() {
        return Ok(target);
    }
    let partial = target.with_extension("bin.part");
    let response = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Model(e.to_string()))?
        .get(format!("{BASE_URL}/{}", model.file))
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| AppError::Model(format!("download failed: {e}")))?;
    let total = response.content_length().unwrap_or(model.size_mb as u64 * 1_048_576);
    let mut file = tokio::fs::File::create(&partial).await?;
    let mut received: u64 = 0;
    let mut stream = response.bytes_stream();
    let mut last_report = 0.0_f32;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AppError::Model(format!("download interrupted: {e}")))?;
        file.write_all(&chunk).await?;
        received += chunk.len() as u64;
        let fraction = (received as f32 / total.max(1) as f32).min(1.0);
        if fraction - last_report >= 0.01 {
            last_report = fraction;
            progress(fraction);
        }
    }
    file.flush().await?;
    drop(file);
    if received < total * 95 / 100 {
        let _ = tokio::fs::remove_file(&partial).await;
        return Err(AppError::Model("download ended early; please try again".into()));
    }
    tokio::fs::rename(&partial, &target).await?;
    progress(1.0);
    Ok(target)
}

pub fn delete(id: &str) -> AppResult<()> {
    if let Some(path) = path_for(id) {
        if path.is_file() {
            std::fs::remove_file(path)?;
        }
    }
    Ok(())
}
