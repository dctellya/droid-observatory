//! The live output of a hosted agent (FR-A3): the latest lines of its spool, masked, and a
//! count of every line. The shell sends the new lines of a spool as `OutputLines`; the UI folds
//! them into a `LiveOutput` and renders it.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use super::scrub::{ScrubbedLine, scrub};

/// New lines of the spool of `launch_id`, already masked.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputLines {
    pub launch_id: String,
    /// Lines the spool holds after these.
    pub total: u64,
    pub lines: Vec<ScrubbedLine>,
}

impl OutputLines {
    /// `raw` are the lines that follow the first `before` lines of the spool.
    pub fn of(launch_id: &str, before: u64, raw: &[Vec<u8>]) -> Self {
        Self {
            launch_id: launch_id.to_owned(),
            total: before + raw.len() as u64,
            lines: raw.iter().map(|line| scrub(line)).collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct LiveOutput {
    capacity: usize,
    launch_id: Option<String>,
    total: u64,
    tail: VecDeque<ScrubbedLine>,
}

impl LiveOutput {
    /// Keeps the latest `capacity` lines (`[ui].live_output_lines`).
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            launch_id: None,
            total: 0,
            tail: VecDeque::new(),
        }
    }

    pub fn push(&mut self, batch: OutputLines) {
        if self.launch_id.as_deref() != Some(batch.launch_id.as_str()) {
            self.tail.clear();
            self.launch_id = Some(batch.launch_id);
        }
        self.total = batch.total;
        self.tail.extend(batch.lines);
        while self.tail.len() > self.capacity {
            self.tail.pop_front();
        }
    }

    pub fn launch_id(&self) -> Option<&str> {
        self.launch_id.as_deref()
    }

    /// Every line the spool holds, shown or not.
    pub fn total(&self) -> u64 {
        self.total
    }

    pub fn lines(&self) -> Vec<ScrubbedLine> {
        self.tail.iter().cloned().collect()
    }
}
