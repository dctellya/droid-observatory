//! The rule for a watched session's name once it was announced: it may change once, and never
//! to a runner tag (D-036: a feature id arrives after the first line; nothing later is a
//! rename).

use std::collections::HashMap;

use crate::model::{SessionId, Timestamp};

/// A name change that was accepted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionRename {
    pub session_id: SessionId,
    pub old: String,
    pub new: String,
    /// When the change was seen, RFC 3339 UTC.
    pub at: String,
}

/// A name change that was not accepted: the session keeps `kept`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenameRefused {
    pub session_id: SessionId,
    pub kept: String,
    pub attempted: String,
    pub at: String,
}

/// What `SessionNames::observe` decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NameChange {
    Unchanged,
    /// The first name seen for the session.
    First,
    Renamed(SessionRename),
    Refused(RenameRefused),
}

struct Entry {
    name: String,
    renamed: bool,
    /// The refused name already reported, so a poll that sees it again says nothing.
    refused: Option<String>,
}

pub struct SessionNames {
    runner_tags: Vec<String>,
    entries: HashMap<SessionId, Entry>,
}

fn now() -> String {
    Timestamp(time::OffsetDateTime::now_utc()).to_rfc3339_millis()
}

impl SessionNames {
    pub fn new(runner_tags: &[String]) -> Self {
        Self {
            runner_tags: runner_tags.to_vec(),
            entries: HashMap::new(),
        }
    }

    /// The name accepted for `id` so far.
    pub fn current(&self, id: &SessionId) -> Option<&str> {
        self.entries.get(id).map(|entry| entry.name.as_str())
    }

    /// Takes the name the files give `id` now.
    pub fn observe(&mut self, id: &SessionId, name: &str) -> NameChange {
        let Some(entry) = self.entries.get_mut(id) else {
            self.entries.insert(
                id.clone(),
                Entry {
                    name: name.to_owned(),
                    renamed: false,
                    refused: None,
                },
            );
            return NameChange::First;
        };
        if entry.name == name {
            return NameChange::Unchanged;
        }
        if entry.renamed || self.runner_tags.iter().any(|tag| tag == name) {
            if entry.refused.as_deref() == Some(name) {
                return NameChange::Unchanged;
            }
            entry.refused = Some(name.to_owned());
            return NameChange::Refused(RenameRefused {
                session_id: id.clone(),
                kept: entry.name.clone(),
                attempted: name.to_owned(),
                at: now(),
            });
        }
        let old = std::mem::replace(&mut entry.name, name.to_owned());
        entry.renamed = true;
        NameChange::Renamed(SessionRename {
            session_id: id.clone(),
            old,
            new: name.to_owned(),
            at: now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> SessionNames {
        SessionNames::new(&["exec".to_owned(), "mission-worker".to_owned()])
    }

    fn id() -> SessionId {
        SessionId("s1".to_owned())
    }

    #[test]
    fn the_name_changes_once_and_never_back() {
        let mut names = names();
        assert_eq!(names.observe(&id(), "s1"), NameChange::First);
        assert!(
            matches!(names.observe(&id(), "f-1"), NameChange::Renamed(r) if r.old == "s1" && r.new == "f-1")
        );
        assert!(matches!(names.observe(&id(), "s1"), NameChange::Refused(r) if r.kept == "f-1"));
        assert_eq!(names.observe(&id(), "s1"), NameChange::Unchanged);
        assert_eq!(names.current(&id()), Some("f-1"));
    }

    #[test]
    fn a_runner_tag_is_refused_before_the_one_rename_is_used() {
        let mut names = names();
        names.observe(&id(), "s1");
        assert!(matches!(
            names.observe(&id(), "exec"),
            NameChange::Refused(_)
        ));
        assert!(matches!(
            names.observe(&id(), "f-1"),
            NameChange::Renamed(_)
        ));
    }
}
