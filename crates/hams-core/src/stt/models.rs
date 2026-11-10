//! Whisper model catalog and downloader. Models live in the app data folder and
//! are downloaded on demand, so the installer stays small.

use std::path::PathBuf;

use futures_util::StreamExt;
use serde::Serialize;
use tokio::io::AsyncWriteExt;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ModelInfo {
    pub id: &'static str,
    pub file: &'static str,
    pub label: &'static str,
    pub size_mb: u32,
    pub multilingual: bool,
    pub note: &'static str,
    /// Small, quantized models meant for phones. Desktop lists only the others.
    pub mobile: bool,
}

pub const CATALOG: &[ModelInfo] = &[
    ModelInfo {
        id: "base",
        file: "ggml-base.bin",
        label: "Base",
        size_mb: 142,
        multilingual: true,
        note: "Fastest. Good for clear English.",
        mobile: false,
    },
    ModelInfo {
        id: "small",
        file: "ggml-small.bin",
        label: "Small",
        size_mb: 466,
        multilingual: true,
        note: "Recommended balance. Handles Malayalam and mixed speech.",
        mobile: false,
    },
    ModelInfo {
        id: "large-v3-turbo",
        file: "ggml-large-v3-turbo-q5_0.bin",
        label: "Large v3 Turbo",
        size_mb: 547,
        multilingual: true,
        note: "Most accurate. Needs Apple Silicon or a fast CPU.",
        mobile: false,
    },
    ModelInfo {
        id: "base-q5_1",
        file: "ggml-base-q5_1.bin",
        label: "Base",
        size_mb: 57,
        multilingual: true,
        note: "Fast on any phone. Good for clear English.",
        mobile: true,
    },
    ModelInfo {
        id: "small-q5_1",
        file: "ggml-small-q5_1.bin",
        label: "Small",
        size_mb: 181,
        multilingual: true,
        note: "More accurate. Handles Malayalam and mixed speech. Best on recent phones.",
        mobile: true,
    },
];

/// The model a phone starts with.
pub const MOBILE_DEFAULT: &str = "base-q5_1";

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
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct ModelStatus {
    #[serde(flatten)]
    pub info: ModelInfo,
    pub installed: bool,
}

/// Models for this kind of device: phones get the `mobile` ones, computers the rest.
pub fn statuses(mobile: bool) -> Vec<ModelStatus> {
    CATALOG
        .iter()
        .filter(|m| m.mobile == mobile)
        .map(|m| ModelStatus { info: *m, installed: is_installed(m.id) })
        .collect()
}

/// Download a model, reporting progress in 0..1. Writes to `<file>.part` and
/// renames on success, so a cancelled download never looks installed. A `.part`
/// left by an interrupted download is resumed with an HTTP range request.
pub async fn download(id: &str, mut progress: impl FnMut(f32)) -> AppResult<PathBuf> {
    let model = info(id).ok_or_else(|| AppError::Model(format!("unknown model \"{id}\"")))?;
    let target = crate::paths::models_dir().join(model.file);
    if target.is_file() {
        return Ok(target);
    }
    let partial = target.with_extension("bin.part");
    // Resume a download that was cut off (phones lose signal; laptops sleep).
    let resume_from = tokio::fs::metadata(&partial).await.map(|m| m.len()).unwrap_or(0);
    let mut request = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Model(e.to_string()))?
        .get(format!("{BASE_URL}/{}", model.file));
    if resume_from > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={resume_from}-"));
    }
    let response = request
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| AppError::Model(format!("download failed: {e}")))?;
    let resumed = resume_from > 0 && response.status() == reqwest::StatusCode::PARTIAL_CONTENT;
    let already = if resumed { resume_from } else { 0 };
    let total = response.content_length().map(|n| n + already).unwrap_or(model.size_mb as u64 * 1_048_576);
    let mut file = if resumed {
        tokio::fs::OpenOptions::new().append(true).open(&partial).await?
    } else {
        tokio::fs::File::create(&partial).await?
    };
    let mut received: u64 = already;
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
        // Keep the .part: the next attempt resumes from here.
        return Err(AppError::Model("download ended early; try again to resume".into()));
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
