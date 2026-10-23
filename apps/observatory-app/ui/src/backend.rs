//! `BrowserBackend` (architecture §7): the file picker's files are mapped and folded in wasm by
//! `observatory-core`, with no store. The Tauri window's webview runs the same UI over the shell
//! commands instead (`shell`, `capture`): the library is the store's and a row reopens from it.

use std::collections::BTreeMap;
use std::path::PathBuf;

use js_sys::Uint8Array;
use observatory_core::ingest::{MemoryReplay, replay_files};
use observatory_core::view::MissionRow;
use wasm_bindgen_futures::JsFuture;
use web_sys::{File, FileList};

const LIBRARY_KEY: &str = "observatory.library.v1";
/// The file kinds a capture is made of; everything else in a picked folder is not read.
const WANTED: [&str; 3] = [".jsonl", ".settings.json", ".capture.json"];

/// The path a picked file is known by: the folder-relative path when the browser gives one.
fn path_of(file: &File) -> String {
    // `webkitRelativePath` is empty for a multi-file selection and set for a folder.
    let relative = js_sys::Reflect::get(file, &"webkitRelativePath".into())
        .ok()
        .and_then(|value| value.as_string())
        .unwrap_or_default();
    if relative.is_empty() {
        file.name()
    } else {
        relative
    }
}

/// Opens a picker selection: reads the capture files and replays them.
pub async fn open_selection(picked: Vec<File>) -> Result<MemoryReplay, String> {
    let mut files = BTreeMap::new();
    for file in picked {
        let name = path_of(&file);
        if !WANTED.iter().any(|suffix| name.ends_with(suffix)) {
            continue;
        }
        let buffer = JsFuture::from(file.array_buffer())
            .await
            .map_err(|err| format!("cannot read {name}: {err:?}"))?;
        files.insert(PathBuf::from(name), Uint8Array::new(&buffer).to_vec());
    }
    if files.is_empty() {
        return Err("no .jsonl, .settings.json or .capture.json file in the selection".to_owned());
    }
    replay_files(files).map_err(|err| err.to_string())
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

/// The captures opened in this browser before; the store-backed library is the shell's.
pub fn load_library() -> Vec<MissionRow> {
    storage()
        .and_then(|storage| storage.get_item(LIBRARY_KEY).ok().flatten())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save_library(rows: &[MissionRow]) {
    if let (Some(storage), Ok(text)) = (storage(), serde_json::to_string(rows)) {
        let _ = storage.set_item(LIBRARY_KEY, &text);
    }
}

/// `rows` with `row` in place of the line of the same mission, else appended.
pub fn upsert(rows: &mut Vec<MissionRow>, row: MissionRow) {
    match rows.iter_mut().find(|existing| existing.id == row.id) {
        Some(existing) => *existing = row,
        None => rows.push(row),
    }
}

/// The selected files of an `<input type=file>` event.
pub fn files_of(ev: &leptos::ev::Event) -> Vec<File> {
    let input = leptos::prelude::event_target::<web_sys::HtmlInputElement>(ev);
    let picked = input
        .files()
        .map(|list| files_in(&list))
        .unwrap_or_default();
    // The list is live: clearing the value empties it, so the files are taken first. Without
    // the clearing, picking the same files again raises no `change`.
    input.set_value("");
    picked
}

fn files_in(list: &FileList) -> Vec<File> {
    (0..list.length())
        .filter_map(|index| list.get(index))
        .collect()
}
