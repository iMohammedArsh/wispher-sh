use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Logs go to stderr and to a daily rolling file in the app data folder.
/// The returned guard must live for the whole process so buffered lines get flushed.
pub fn init() -> WorkerGuard {
    let file_appender = tracing_appender::rolling::daily(crate::paths::logs_dir(), "hams.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,hams_lib=debug"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(false))
        .with(fmt::layer().with_ansi(false).with_writer(file_writer))
        .init();
    guard
}
