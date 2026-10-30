//! The events of a capture read off disk with no store: the session tree, then the usage-log
//! extract its header names (what `replay_file` stores, for a reader that only computes).

use std::path::Path;
use std::sync::Arc;

use super::capture::{missing_usage_log_warning, read_capture};
use super::memory::order_key;
use super::{IngestError, JsonlSource, SessionTree, Source, UsageLogSource, select_mapping_of};
use crate::config::IngestConfig;
use crate::mapping::TreeMapper;
use crate::model::{CycleTemplate, Event};

/// Every event of the capture at `root` (the root `.jsonl`), child sessions and `turn.usage`
/// turns included, in replay order, and the warnings a reader should show.
pub fn mission_events(
    root: &Path,
    cfg: &IngestConfig,
    template: impl Into<Arc<CycleTemplate>>,
) -> Result<(Vec<Event>, Vec<String>), IngestError> {
    let (version, warning) = select_mapping_of(root)?;
    let mut warnings: Vec<String> = warning.iter().map(ToString::to_string).collect();
    let tree = SessionTree::discover(root, cfg)?;
    let mut source = JsonlSource::open_tree(&tree, cfg)?;
    let mut mapper = TreeMapper::with_template(&tree, version, template);
    let mut events = Vec::new();
    loop {
        let mut batch = source.next_batch()?;
        let at_eof = source.at_eof();
        if at_eof {
            batch.extend(source.drain_partial());
        }
        events.extend(batch.iter().flat_map(|record| mapper.map(record)));
        if at_eof {
            events.extend(mapper.finish());
            break;
        }
    }
    if let Some(info) = read_capture(root)?.and_then(|header| header.usage_log) {
        let path = root.with_file_name(&info.file);
        if path.is_file() {
            let mut log = UsageLogSource::open_files(&[path], cfg)?;
            let turns = log.read_all()?;
            events.extend(turns.iter().filter_map(|turn| mapper.map_usage(turn)));
        } else {
            warnings.push(missing_usage_log_warning(&info.file));
        }
    }
    events.sort_by_key(order_key);
    Ok((events, warnings))
}
