//! What the mapped events do not carry and the engine needs to pair or name them: the scenario or
//! check an event is about, the reason on a pause, the question an app answer closes. The
//! callers read these from where they live (the store's `rework_windows.check_name`, the
//! `human_actions` row, the runner's progress log); without a note an event pairs by kind.

use std::collections::{HashMap, HashSet};

use crate::model::{Event, FeatureId};

#[derive(Clone, Debug, Default)]
pub struct Notes {
    keys: HashMap<String, String>,
    pause_reasons: HashMap<String, String>,
    answers: HashMap<String, String>,
}

impl Notes {
    /// `key` names the scenario (`VAL-A-001`) or the check (`demo-gate`) the event is about.
    #[must_use]
    pub fn with_key(mut self, event: &str, key: &str) -> Self {
        self.keys.insert(event.to_owned(), key.to_owned());
        self
    }

    /// The `pauseReason` recorded on a pause marker (`unrecoverable_usage_402`).
    #[must_use]
    pub fn with_pause_reason(mut self, event: &str, reason: &str) -> Self {
        self.pause_reasons
            .insert(event.to_owned(), reason.to_owned());
        self
    }

    /// `answer` is the event of an app answer, `question` the event it answers
    /// (`human_actions.payload_ptr`).
    #[must_use]
    pub fn with_answer(mut self, answer: &str, question: &str) -> Self {
        self.answers.insert(answer.to_owned(), question.to_owned());
        self
    }

    /// The scenario or check an event is about, when the notes name it.
    pub fn key(&self, event: &Event) -> Option<&str> {
        self.keys.get(&event.id.0).map(String::as_str)
    }

    pub(super) fn pause_reason(&self, event: &Event) -> Option<&str> {
        self.pause_reasons.get(&event.id.0).map(String::as_str)
    }

    pub(super) fn question_of(&self, answer: &Event) -> Option<&str> {
        self.answers.get(&answer.id.0).map(String::as_str)
    }
}

/// The events of one feature: those that name it, and the app answers to its questions (an
/// answer event names no feature of its own). Oldest first, without `turn.usage`.
pub fn events_of_feature(events: &[Event], feature: &FeatureId, notes: &Notes) -> Vec<Event> {
    let own: HashSet<&str> = events
        .iter()
        .filter(|event| event.feature_id.as_ref() == Some(feature))
        .map(|event| event.id.0.as_str())
        .collect();
    let mut mine: Vec<Event> = events
        .iter()
        .filter(|event| !event.kind.is_panel_only())
        .filter(|event| {
            event.feature_id.as_ref() == Some(feature)
                || (event.feature_id.is_none()
                    && notes
                        .question_of(event)
                        .is_some_and(|question| own.contains(question)))
        })
        .cloned()
        .collect();
    mine.sort_by_key(|event| event.at);
    mine
}
