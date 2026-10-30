//! Tokens from `turn.usage` (D-029): per role, lane, milestone and session, the tokens burnt in
//! waste bands, and the reconciliation against the sidecar totals.

use std::collections::BTreeMap;
use std::ops::AddAssign;

use serde::Serialize;

use super::cards::CardTime;
use super::span::Span;
use super::waste::WasteCause;
use crate::model::{Event, EventKind, FeatureId, ModelId, Role, SessionId, Timestamp};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct TokenCounts {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
}

impl AddAssign for TokenCounts {
    fn add_assign(&mut self, other: Self) {
        self.input += other.input;
        self.output += other.output;
        self.cache_read += other.cache_read;
    }
}

impl TokenCounts {
    pub fn total(&self) -> u64 {
        self.input + self.output + self.cache_read
    }

    pub fn is_zero(&self) -> bool {
        self.total() == 0
    }
}

/// The payload of one `turn.usage` event.
#[derive(Clone, Copy, Debug)]
pub struct Turn<'a> {
    pub event: &'a Event,
    pub tokens: TokenCounts,
    /// The log's `modelId`, else the model the event was mapped under.
    pub model: Option<&'a ModelId>,
    pub log_ts: Timestamp,
}

pub fn turns(events: &[Event]) -> impl Iterator<Item = Turn<'_>> {
    events.iter().filter_map(|event| match &event.kind {
        EventKind::TurnUsage {
            input,
            output,
            cache_read,
            model,
            log_ts,
            ..
        } => Some(Turn {
            event,
            tokens: TokenCounts {
                input: *input,
                output: *output,
                cache_read: *cache_read,
            },
            model: model.as_ref().or(event.model.as_ref()),
            log_ts: *log_ts,
        }),
        _ => None,
    })
}

/// The tokens of the turns inside waste bands, per cause, and apart, inside a `throttled`
/// interval. Each turn is counted once: the stop first, then the cause whose band holds it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct TokensInWaste {
    #[serde(flatten)]
    pub by_cause: BTreeMap<WasteCause, TokenCounts>,
    pub throttled: TokenCounts,
}

pub fn tokens_in_waste(
    events: &[Event],
    cards: &BTreeMap<Option<FeatureId>, CardTime>,
    throttled: &[Span],
) -> TokensInWaste {
    let mut found = TokensInWaste {
        by_cause: WasteCause::ALL
            .into_iter()
            .map(|cause| (cause, TokenCounts::default()))
            .collect(),
        throttled: TokenCounts::default(),
    };
    for turn in turns(events) {
        if throttled
            .iter()
            .any(|span| span.from <= turn.log_ts && turn.log_ts < span.to)
        {
            found.throttled += turn.tokens;
        } else if let Some(cause) = cards
            .get(&turn.event.feature_id)
            .and_then(|card| card.waste.cause_at(turn.log_ts))
        {
            *found.by_cause.entry(cause).or_default() += turn.tokens;
        }
    }
    found
}

/// `turns - reported`, per class.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SignedCounts {
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
}

fn signed(turns: u64, reported: u64) -> i64 {
    i64::try_from(turns).unwrap_or(i64::MAX) - i64::try_from(reported).unwrap_or(i64::MAX)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SessionReconciliation {
    pub session: SessionId,
    pub role: Role,
    pub turns: TokenCounts,
    /// The sidecar's totals; zero when the session has no `usage.reported`.
    pub reported: TokenCounts,
    pub has_sidecar: bool,
    pub difference: SignedCounts,
}

/// The per-session sums of the turns next to the sidecar totals. Equality is not required: the
/// sidecar may count turns outside the log window, so the extract's first and last timestamps
/// are carried with the difference.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Reconciliation {
    pub first_ts: Option<Timestamp>,
    pub last_ts: Option<Timestamp>,
    pub sessions: Vec<SessionReconciliation>,
}

/// The id under which turns the log could not join to a session are listed.
pub const UNJOINED_SESSION: &str = "unjoined";

pub fn reconcile(events: &[Event]) -> Reconciliation {
    struct Row {
        role: Role,
        turns: TokenCounts,
        reported: Option<TokenCounts>,
    }
    let mut rows: BTreeMap<String, Row> = BTreeMap::new();
    let session_of = |event: &Event| {
        event
            .session_id
            .as_ref()
            .map_or_else(|| UNJOINED_SESSION.to_owned(), |id| id.0.clone())
    };
    let mut found = Reconciliation::default();
    for turn in turns(events) {
        let row = rows.entry(session_of(turn.event)).or_insert(Row {
            role: turn.event.role,
            turns: TokenCounts::default(),
            reported: None,
        });
        row.turns += turn.tokens;
        found.first_ts = found
            .first_ts
            .map_or(Some(turn.log_ts), |t| Some(t.min(turn.log_ts)));
        found.last_ts = found
            .last_ts
            .map_or(Some(turn.log_ts), |t| Some(t.max(turn.log_ts)));
    }
    for event in events.iter().filter(|e| e.kind == EventKind::UsageReported) {
        let Some(usage) = event.usage else { continue };
        let row = rows.entry(session_of(event)).or_insert(Row {
            role: event.role,
            turns: TokenCounts::default(),
            reported: None,
        });
        let total = row.reported.get_or_insert_with(TokenCounts::default);
        *total += TokenCounts {
            input: usage.input,
            output: usage.output,
            cache_read: usage.cache_read,
        };
    }
    found.sessions = rows
        .into_iter()
        .map(|(session, row)| {
            let reported = row.reported.unwrap_or_default();
            SessionReconciliation {
                session: SessionId(session),
                role: row.role,
                turns: row.turns,
                reported,
                has_sidecar: row.reported.is_some(),
                difference: SignedCounts {
                    input: signed(row.turns.input, reported.input),
                    output: signed(row.turns.output, reported.output),
                    cache_read: signed(row.turns.cache_read, reported.cache_read),
                },
            }
        })
        .collect();
    found
}
