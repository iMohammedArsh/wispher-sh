//! Generates `src/lib/bindings.ts` from the Rust types the frontend receives,
//! so the TypeScript side can't drift from serde. Runs as a test (specta is a
//! dev-dependency only, so release builds don't carry it):
//!
//!     cargo test bindings
//!
//! The test fails when the file was out of date, after rewriting it.

use specta::Types;
use specta_typescript::Typescript;

use crate::commands::{ModelProgress, RestyleResult, SettingsView, StyleInfo};
use crate::controller::Status;
use crate::platform::Permissions;
use crate::store::{DictionaryEntry, HistoryItem, Note, Snippet, Stats, Suggestion};
use crate::stt::models::ModelStatus;
use crate::updater::{UpdateInfo, UpdateProgress};

const HEADER: &str = "// Generated from the Rust types by `cargo test bindings` (src-tauri/src/bindings.rs). Don't edit by hand.";

fn generate() -> String {
    let types = Types::default()
        .register::<SettingsView>()
        .register::<Status>()
        .register::<HistoryItem>()
        .register::<Stats>()
        .register::<Note>()
        .register::<DictionaryEntry>()
        .register::<Snippet>()
        .register::<Suggestion>()
        .register::<ModelStatus>()
        .register::<ModelProgress>()
        .register::<StyleInfo>()
        .register::<Permissions>()
        .register::<RestyleResult>()
        .register::<UpdateInfo>()
        .register::<UpdateProgress>();
    // Ids, counts and Unix-ms timestamps all stay far below 2^53.
    let types = specta_util::Remapper::new().dangerous_bigints_as_number().remap_types(types);
    Typescript::default().header(HEADER).export(&types, specta_serde::PhasesFormat).expect("export TypeScript bindings")
}

#[test]
fn bindings_are_up_to_date() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/lib/bindings.ts");
    let fresh = generate();
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    if current != fresh {
        std::fs::write(&path, &fresh).expect("write bindings.ts");
        panic!("src/lib/bindings.ts was out of date and has been regenerated. Check the diff and re-run.");
    }
}
