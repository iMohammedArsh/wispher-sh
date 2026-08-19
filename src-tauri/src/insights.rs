use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const MAX_EVENTS: usize = 5_000;
const SCHEMA_VERSION: u8 = 1;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InsightEvent {
    pub created_at_ms: u128,
    pub word_count: u32,
    pub duration_ms: u64,
    pub category: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CorrectionRule {
    pub source: String,
    pub replacement: String,
    pub uses: u32,
    pub enabled: bool,
    pub created_at_ms: u128,
    pub last_used_at_ms: u128,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct InsightsData {
    #[serde(default)]
    schema_version: u8,
    #[serde(default)]
    events: Vec<InsightEvent>,
    #[serde(default)]
    correction_rules: Vec<CorrectionRule>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct InsightsSummary {
    pub tracked_words: u64,
    pub completed_dictations: u64,
    pub average_wpm: u32,
    pub current_streak_days: u32,
    pub longest_streak_days: u32,
    pub profile_unlocked: bool,
    pub category_words: BTreeMap<String, u64>,
    pub activity_days: BTreeMap<String, u32>,
    pub correction_rules: Vec<CorrectionRule>,
}

#[derive(Debug, Default)]
pub struct LocalInsights {
    data: InsightsData,
}

impl LocalInsights {
    fn file_path() -> PathBuf {
        let mut directory = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        directory.push("wisper");
        directory.push("insights.json");
        directory
    }

    pub fn load() -> Self {
        let path = Self::file_path();
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                match serde_json::from_str::<InsightsData>(&content) {
                    Ok(mut data) => {
                        data.schema_version = SCHEMA_VERSION;
                        Self { data }
                    }
                    Err(error) => {
                        eprintln!("[Insights] Ignoring malformed local Insights data at {path:?}: {error}");
                        Self::default()
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(error) => {
                eprintln!("[Insights] Could not read local Insights data at {path:?}: {error}");
                Self::default()
            }
        }
    }

    pub fn add_completed_dictation(
        &mut self,
        text: &str,
        duration_ms: u64,
        category: String,
    ) -> Result<(), String> {
        let word_count = text.split_whitespace().count() as u32;
        if word_count == 0 {
            return Ok(());
        }
        let created_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("Could not timestamp Insights event: {error}"))?
            .as_millis();
        self.data.events.push(InsightEvent {
            created_at_ms,
            word_count,
            duration_ms,
            category,
        });
        if self.data.events.len() > MAX_EVENTS {
            let remove = self.data.events.len() - MAX_EVENTS;
            self.data.events.drain(0..remove);
        }
        self.persist()
    }

    pub fn summary(&self) -> InsightsSummary {
        let mut category_words = BTreeMap::new();
        let mut activity_days = BTreeMap::new();
        let mut total_words = 0_u64;
        let mut total_duration_ms = 0_u64;
        let mut days = Vec::new();
        for event in &self.data.events {
            total_words += u64::from(event.word_count);
            total_duration_ms += event.duration_ms;
            *category_words.entry(event.category.clone()).or_insert(0) +=
                u64::from(event.word_count);
            let day = local_day_key(event.created_at_ms);
            *activity_days.entry(day.clone()).or_insert(0) += event.word_count;
            days.push(day);
        }
        days.sort();
        days.dedup();
        let (current_streak_days, longest_streak_days) = streaks(&days);
        let average_wpm = if total_duration_ms == 0 {
            0
        } else {
            ((total_words as f64 / total_duration_ms as f64) * 60_000.0).round() as u32
        };
        InsightsSummary {
            tracked_words: total_words,
            completed_dictations: self.data.events.len() as u64,
            average_wpm,
            current_streak_days,
            longest_streak_days,
            profile_unlocked: total_words >= 2_000,
            category_words,
            activity_days,
            correction_rules: self.data.correction_rules.clone(),
        }
    }

    pub fn add_correction_rule(
        &mut self,
        source: String,
        replacement: String,
    ) -> Result<(), String> {
        let source = source.trim();
        let replacement = replacement.trim();
        if source.is_empty() || replacement.is_empty() {
            return Err("Both the heard phrase and approved replacement are required.".into());
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("Could not timestamp correction rule: {error}"))?
            .as_millis();
        if let Some(rule) = self
            .data
            .correction_rules
            .iter_mut()
            .find(|rule| rule.source.eq_ignore_ascii_case(source))
        {
            rule.replacement = replacement.to_owned();
            rule.enabled = true;
            rule.last_used_at_ms = now;
        } else {
            self.data.correction_rules.push(CorrectionRule {
                source: source.to_owned(),
                replacement: replacement.to_owned(),
                uses: 0,
                enabled: true,
                created_at_ms: now,
                last_used_at_ms: now,
            });
        }
        self.persist()
    }

    pub fn apply_correction_rules(&mut self, text: &str) -> Result<String, String> {
        let mut result = text.to_owned();
        let mut changed = false;
        for rule in self
            .data
            .correction_rules
            .iter_mut()
            .filter(|rule| rule.enabled)
        {
            if result.contains(&rule.source) {
                result = result.replace(&rule.source, &rule.replacement);
                rule.uses += 1;
                rule.last_used_at_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|error| format!("Could not timestamp correction use: {error}"))?
                    .as_millis();
                changed = true;
            }
        }
        if changed {
            self.persist()?;
        }
        Ok(result)
    }

    fn persist(&self) -> Result<(), String> {
        let path = Self::file_path();
        let directory = path.parent().expect("Insights path has a parent directory");
        std::fs::create_dir_all(directory)
            .map_err(|error| format!("Could not create Insights directory: {error}"))?;
        let serialized = serde_json::to_string_pretty(&self.data)
            .map_err(|error| format!("Could not serialize Insights data: {error}"))?;
        let temporary_path = path.with_extension("json.tmp");
        std::fs::write(&temporary_path, serialized)
            .map_err(|error| format!("Could not write local Insights data: {error}"))?;
        std::fs::rename(&temporary_path, &path)
            .map_err(|error| format!("Could not save local Insights data: {error}"))
    }
}

fn local_day_key(timestamp_ms: u128) -> String {
    // This intentionally uses a stable local-calendar approximation for the current desktop
    // build. Exact display formatting happens in the UI; metrics never store transcript text.
    let days_since_epoch = timestamp_ms / 86_400_000;
    days_since_epoch.to_string()
}

fn streaks(days: &[String]) -> (u32, u32) {
    if days.is_empty() {
        return (0, 0);
    }
    let parsed: Vec<u128> = days.iter().filter_map(|value| value.parse().ok()).collect();
    let mut longest = 1_u32;
    let mut running = 1_u32;
    for pair in parsed.windows(2) {
        if pair[1] == pair[0] + 1 {
            running += 1;
        } else {
            running = 1;
        }
        longest = longest.max(running);
    }
    let today = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() / 86_400_000)
        .unwrap_or_default();
    let mut current = 0_u32;
    let mut expected = today;
    for day in parsed.iter().rev() {
        if *day == expected {
            current += 1;
            expected = expected.saturating_sub(1);
        } else if *day < expected {
            break;
        }
    }
    (current, longest)
}

pub fn category_for_process(process: Option<&str>) -> String {
    let lower = process.unwrap_or_default().to_ascii_lowercase();
    let categories: HashMap<&str, &str> = HashMap::from([
        ("code.exe", "coding"),
        ("devenv.exe", "coding"),
        ("zed.exe", "coding"),
        ("outlook.exe", "email"),
        ("thunderbird.exe", "email"),
        ("winword.exe", "documents"),
        ("powerpnt.exe", "documents"),
        ("excel.exe", "documents"),
        ("chrome.exe", "browser"),
        ("msedge.exe", "browser"),
        ("firefox.exe", "browser"),
        ("whatsapp.exe", "messages"),
        ("slack.exe", "messages"),
        ("teams.exe", "messages"),
    ]);
    categories
        .get(lower.as_str())
        .copied()
        .unwrap_or("other")
        .to_string()
}

#[cfg(target_os = "windows")]
pub fn foreground_process_name() -> Option<String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };

    let window = unsafe { GetForegroundWindow() };
    if window.is_null() {
        return None;
    }
    let mut process_id = 0_u32;
    unsafe { GetWindowThreadProcessId(window, &mut process_id) };
    if process_id == 0 {
        return None;
    }
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
    if process.is_null() {
        return None;
    }
    let mut buffer = vec![0_u16; 32_768];
    let mut length = buffer.len() as u32;
    let success =
        unsafe { QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length) } != 0;
    unsafe { CloseHandle(process) };
    if !success {
        return None;
    }
    let path = String::from_utf16_lossy(&buffer[..length as usize]);
    std::path::Path::new(&path)
        .file_name()?
        .to_str()
        .map(str::to_owned)
}

#[cfg(not(target_os = "windows"))]
pub fn foreground_process_name() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn categorizes_known_processes_locally() {
        assert_eq!(category_for_process(Some("Code.exe")), "coding");
        assert_eq!(category_for_process(Some("OUTLOOK.EXE")), "email");
        assert_eq!(category_for_process(Some("unknown.exe")), "other");
    }
}
