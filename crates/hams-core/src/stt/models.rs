//! Whisper model catalog and downloader. Models live in the app data folder and
//! are downloaded on demand, so the installer stays small.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

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
    ModelInfo {
        id: "base.en-q5_1",
        file: "ggml-base.en-q5_1.bin",
        label: "Base · English",
        size_mb: 57,
        multilingual: false,
        note: "English only. Fast, and more accurate for English than Base.",
        mobile: true,
    },
    ModelInfo {
        id: "small.en-q5_1",
        file: "ggml-small.en-q5_1.bin",
        label: "Small · English",
        size_mb: 181,
        multilingual: false,
        note: "English only. The most accurate English on a phone. Best on recent phones.",
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

/// How long a download may receive nothing before it's reported as stalled.
const STALL_TIMEOUT: Duration = Duration::from_secs(30);

/// The message of the error a cancelled download returns; see [`was_cancelled`].
const CANCELLED: &str = "download cancelled";

/// Whether `error` came from a download cancelled through its flag (not a failure).
pub fn was_cancelled(error: &AppError) -> bool {
    matches!(error, AppError::Model(message) if message == CANCELLED)
}

/// Download a model, reporting progress in 0..1. Writes to `<file>.part` and
/// renames on success, so a cancelled download never looks installed. A `.part`
/// left by an interrupted download is resumed with an HTTP range request.
pub async fn download(id: &str, progress: impl FnMut(f32)) -> AppResult<PathBuf> {
    download_cancellable(id, Arc::new(AtomicBool::new(false)), progress).await
}

/// Like [`download`], but stops soon after `cancel` is set. The `.part` file is
/// kept, so downloading the model again continues where it stopped.
pub async fn download_cancellable(id: &str, cancel: Arc<AtomicBool>, progress: impl FnMut(f32)) -> AppResult<PathBuf> {
    let model = info(id).ok_or_else(|| AppError::Model(format!("unknown model \"{id}\"")))?;
    let target = crate::paths::models_dir().join(model.file);
    if target.is_file() {
        return Ok(target);
    }
    let url = format!("{BASE_URL}/{}", model.file);
    fetch(&url, &target, model.size_mb as u64 * 1_048_576, &cancel, STALL_TIMEOUT, progress).await?;
    Ok(target)
}

/// Fetch `url` into `target` through `<target>.part`. `size_hint` only matters when
/// the server doesn't say how long the file is.
async fn fetch(
    url: &str,
    target: &Path,
    size_hint: u64,
    cancel: &AtomicBool,
    stall_timeout: Duration,
    mut progress: impl FnMut(f32),
) -> AppResult<()> {
    let partial = target.with_extension("bin.part");
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Model(e.to_string()))?;
    // Resume a download that was cut off (phones lose signal; laptops sleep).
    let mut resume_from = tokio::fs::metadata(&partial).await.map(|m| m.len()).unwrap_or(0);
    let response = loop {
        let mut request = client.get(url);
        if resume_from > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={resume_from}-"));
        }
        let response = request.send().await.map_err(|e| AppError::Model(format!("download failed: {e}")))?;
        if resume_from > 0 && response.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
            // The partial file can't be continued (it is already whole, or it belongs
            // to a different file): start again instead of failing forever.
            let _ = tokio::fs::remove_file(&partial).await;
            resume_from = 0;
            continue;
        }
        break response.error_for_status().map_err(|e| AppError::Model(format!("download failed: {e}")))?;
    };
    let resumed = resume_from > 0 && response.status() == reqwest::StatusCode::PARTIAL_CONTENT;
    let already = if resumed { resume_from } else { 0 };
    // The exact size, when the server tells us; otherwise an estimate for the progress bar.
    let exact = response.content_length().map(|n| n + already);
    let total = exact.unwrap_or(size_hint);
    let mut file = if resumed {
        tokio::fs::OpenOptions::new().append(true).open(&partial).await?
    } else {
        tokio::fs::File::create(&partial).await?
    };
    let mut received: u64 = already;
    let mut stream = response.bytes_stream();
    let mut last_report = 0.0_f32;
    let mut idle = Duration::ZERO;
    const STEP: Duration = Duration::from_millis(200);
    let outcome: AppResult<()> = async {
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(AppError::Model(CANCELLED.into()));
            }
            // Wake up regularly, so a cancel is noticed even when no data arrives.
            match tokio::time::timeout(STEP, stream.next()).await {
                Ok(Some(chunk)) => {
                    idle = Duration::ZERO;
                    let chunk = chunk.map_err(|e| AppError::Model(format!("download interrupted: {e}")))?;
                    file.write_all(&chunk).await?;
                    received += chunk.len() as u64;
                    let fraction = (received as f32 / total.max(1) as f32).min(1.0);
                    if fraction - last_report >= 0.01 {
                        last_report = fraction;
                        progress(fraction);
                    }
                }
                Ok(None) => return Ok(()),
                Err(_) => {
                    idle += STEP;
                    if idle >= stall_timeout {
                        return Err(AppError::Model("the download stalled; try again to resume".into()));
                    }
                }
            }
        }
    }
    .await;
    // Flush even on failure, so the .part holds everything received and resumes cleanly.
    file.flush().await?;
    drop(file);
    outcome?;
    let whole = match exact {
        Some(length) => received == length,
        None => received >= total * 95 / 100,
    };
    if !whole {
        // Keep the .part: the next attempt resumes from here.
        return Err(AppError::Model("download ended early; try again to resume".into()));
    }
    tokio::fs::rename(&partial, target).await?;
    progress(1.0);
    Ok(())
}

pub fn delete(id: &str) -> AppResult<()> {
    if let Some(path) = path_for(id) {
        if path.is_file() {
            std::fs::remove_file(path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;

    #[derive(Clone, Copy)]
    enum Serve {
        /// Honours Range requests.
        Normal,
        /// Advertises the whole file but closes the connection after this many bytes.
        CutAfter(usize),
        /// Sends this many bytes, then goes silent without closing.
        StallAfter(usize),
        /// Answers any Range request with 416, as for a `.part` that can't be continued.
        RejectRange,
    }

    fn body() -> Vec<u8> {
        (0..1000).map(|i| (i % 251) as u8).collect()
    }

    /// A one-file HTTP server on localhost. Returns its URL.
    async fn serve(body: Vec<u8>, mode: Serve) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/m.bin", listener.local_addr().unwrap());
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else { return };
                let body = body.clone();
                tokio::spawn(async move {
                    let mut request = Vec::new();
                    let mut buf = [0u8; 1024];
                    while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                        match socket.read(&mut buf).await {
                            Ok(n) if n > 0 => request.extend_from_slice(&buf[..n]),
                            _ => return,
                        }
                    }
                    let text = String::from_utf8_lossy(&request).to_lowercase();
                    let from = text
                        .lines()
                        .find_map(|l| l.strip_prefix("range: bytes="))
                        .and_then(|r| r.trim().trim_end_matches('-').parse::<usize>().ok());
                    if from.is_some() && matches!(mode, Serve::RejectRange) {
                        let _ = socket
                            .write_all(b"HTTP/1.1 416 Range Not Satisfiable\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
                            .await;
                        return;
                    }
                    let start = from.unwrap_or(0);
                    let payload = &body[start..];
                    let head = if from.is_some() {
                        format!(
                            "HTTP/1.1 206 Partial Content\r\ncontent-length: {}\r\ncontent-range: bytes {start}-{}/{}\r\nconnection: close\r\n\r\n",
                            payload.len(),
                            body.len() - 1,
                            body.len()
                        )
                    } else {
                        format!("HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n", payload.len())
                    };
                    let send = match mode {
                        Serve::CutAfter(n) | Serve::StallAfter(n) => n.min(payload.len()),
                        _ => payload.len(),
                    };
                    let _ = socket.write_all(head.as_bytes()).await;
                    let _ = socket.write_all(&payload[..send]).await;
                    let _ = socket.flush().await;
                    if matches!(mode, Serve::StallAfter(_)) {
                        tokio::time::sleep(Duration::from_secs(60)).await;
                    }
                });
            }
        });
        url
    }

    fn temp_target() -> PathBuf {
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!("hams-models-test-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("m.bin")
    }

    async fn run(url: &str, target: &Path, cancel: &AtomicBool, stall: Duration) -> AppResult<()> {
        fetch(url, target, 1000, cancel, stall, |_| {}).await
    }

    fn part(target: &Path) -> PathBuf {
        target.with_extension("bin.part")
    }

    #[tokio::test]
    async fn a_whole_download_is_installed_and_reports_done() {
        let url = serve(body(), Serve::Normal).await;
        let target = temp_target();
        let mut last = 0.0;
        fetch(&url, &target, 1000, &AtomicBool::new(false), STALL_TIMEOUT, |f| last = f).await.unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), body());
        assert!(!part(&target).exists());
        assert_eq!(last, 1.0);
    }

    /// A model missing its last 2% is corrupt (Whisper can't load it) and must not
    /// look installed. The HTTP layer already rejects a body shorter than its
    /// Content-Length, which is what fires here; the exact-length check in `fetch`
    /// is a second guard behind it.
    #[tokio::test]
    async fn a_download_missing_its_last_bytes_is_not_installed_and_later_resumes() {
        let cancel = AtomicBool::new(false);
        let target = temp_target();
        let cut = serve(body(), Serve::CutAfter(980)).await;
        let error = run(&cut, &target, &cancel, STALL_TIMEOUT).await.unwrap_err();
        let message = error.to_string();
        assert!(message.contains("interrupted") || message.contains("ended early"), "{message}");
        assert!(!target.exists());
        assert_eq!(std::fs::metadata(part(&target)).unwrap().len(), 980);

        let whole = serve(body(), Serve::Normal).await;
        run(&whole, &target, &cancel, STALL_TIMEOUT).await.unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), body(), "resumed from the .part, not restarted");
        assert!(!part(&target).exists());
    }

    #[tokio::test]
    async fn a_part_file_the_server_rejects_is_replaced_not_stuck_forever() {
        let target = temp_target();
        std::fs::write(part(&target), b"junk from another file").unwrap();
        let url = serve(body(), Serve::RejectRange).await;
        run(&url, &target, &AtomicBool::new(false), STALL_TIMEOUT).await.unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), body());
    }

    #[tokio::test]
    async fn a_stalled_download_times_out_and_keeps_what_it_got() {
        let url = serve(body(), Serve::StallAfter(100)).await;
        let target = temp_target();
        let error = run(&url, &target, &AtomicBool::new(false), Duration::from_millis(600)).await.unwrap_err();
        assert!(error.to_string().contains("stalled"), "{error}");
        assert_eq!(std::fs::metadata(part(&target)).unwrap().len(), 100);
    }

    #[tokio::test]
    async fn cancelling_stops_a_stalled_download_at_once_and_keeps_the_part() {
        let url = serve(body(), Serve::StallAfter(100)).await;
        let target = temp_target();
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancel);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(400)).await;
            flag.store(true, Ordering::Relaxed);
        });
        let started = std::time::Instant::now();
        let error = run(&url, &target, &cancel, STALL_TIMEOUT).await.unwrap_err();
        assert!(was_cancelled(&error), "{error}");
        assert!(started.elapsed() < Duration::from_secs(5), "cancel must not wait for the stall timeout");
        assert_eq!(std::fs::metadata(part(&target)).unwrap().len(), 100);
        assert!(!target.exists());
    }
}
