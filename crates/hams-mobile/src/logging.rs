//! Engine logs go to the phone's log (logcat / os_log) through `Host::log`.

use std::io::Write;
use std::sync::{Arc, OnceLock};

use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::EnvFilter;

use crate::Host;

static HOST: OnceLock<Arc<dyn Host>> = OnceLock::new();

/// Install the log forwarder once per process. Later engines reuse the first host.
pub fn init(host: Arc<dyn Host>) {
    if HOST.set(host).is_err() {
        return;
    }
    let filter = EnvFilter::try_new("info,hams_core=debug,hams_mobile=debug,whisper_rs=warn").unwrap_or_default();
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .without_time()
        .with_target(false)
        .with_level(true)
        .with_writer(HostWriter)
        .try_init();
}

#[derive(Clone, Copy)]
struct HostWriter;

impl<'a> MakeWriter<'a> for HostWriter {
    type Writer = LineBuffer;
    fn make_writer(&'a self) -> Self::Writer {
        LineBuffer(Vec::with_capacity(160))
    }
}

/// Collects one formatted event and hands it to the host when dropped.
struct LineBuffer(Vec<u8>);

impl Write for LineBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Drop for LineBuffer {
    fn drop(&mut self) {
        let Some(host) = HOST.get() else { return };
        let line = String::from_utf8_lossy(&self.0);
        let line = line.trim_end();
        if line.is_empty() {
            return;
        }
        let level = match line.split_whitespace().next() {
            Some("ERROR") => 1,
            Some("WARN") => 2,
            Some("INFO") => 3,
            Some("DEBUG") => 4,
            _ => 5,
        };
        host.log(level, line.to_string());
    }
}
