//! Findings in validator reports. The format was read off the M0 capture, where the `Result` and
//! heading lines are Markdown the validators wrote by habit, not a schema:
//!
//! * scrutiny reports hold a `**Non-blocking issues**` or `**Blocking issues**` heading (bold
//!   line or `#` heading) followed by numbered or bulleted items, and often a
//!   `**Result: pass.**` line;
//! * user-testing reports hold one verdict per assertion, as a table row
//!   `| VAL-XXX-001 | pass | ... |` or a bold line `**VAL-XXX-001 (pass): ...**`, and name an
//!   `evidence/` folder.

use crate::model::EventKind;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Other,
    Blocking,
    NonBlocking,
}

/// One `Finding*` per item under a (non-)blocking heading, then `ScrutinyPassed` when the text
/// is a report that holds no blocking item and does not say it failed. Text that is neither
/// (a validator thinking aloud) yields nothing.
pub(super) fn scrutiny(text: &str) -> Vec<EventKind> {
    let mut events = Vec::new();
    let mut section = Section::Other;
    let (mut report, mut failed) = (false, false);
    for line in text.lines() {
        let lower = line.to_lowercase();
        if lower.contains("result: pass") {
            report = true;
        }
        if lower.contains("result: fail") {
            report = true;
            failed = true;
        }
        if let Some(title) = heading(line) {
            let title = title.to_lowercase();
            section = if title.contains("non-blocking") {
                Section::NonBlocking
            } else if title.contains("blocking") {
                Section::Blocking
            } else {
                Section::Other
            };
            report |= section != Section::Other;
        } else if is_item(line) {
            match section {
                Section::Blocking => {
                    failed = true;
                    events.push(EventKind::FindingBlocking);
                }
                Section::NonBlocking => events.push(EventKind::FindingNonBlocking),
                Section::Other => {}
            }
        }
    }
    if report && !failed {
        events.push(EventKind::ScrutinyPassed);
    }
    events
}

/// One scenario event per assertion verdict, then `QaEvidence` once if the text names an
/// evidence folder.
pub(super) fn user_testing(text: &str) -> Vec<EventKind> {
    let mut events = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let verdict_of = if line.starts_with('|') {
            let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
            match cells.as_slice() {
                [id, status, ..] if is_assertion_id(clean(id)) => verdict(status),
                _ => None,
            }
        } else {
            line.strip_prefix("**").and_then(|rest| {
                let id_end = rest
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
                    .unwrap_or(rest.len());
                is_assertion_id(&rest[..id_end]).then(|| verdict(&rest[id_end..]))?
            })
        };
        match verdict_of {
            Some(true) => events.push(EventKind::QaScenarioPassed),
            Some(false) => events.push(EventKind::QaScenarioFailed),
            None => {}
        }
    }
    if text.contains("/evidence/") {
        events.push(EventKind::QaEvidence);
    }
    events
}

/// The title of a Markdown heading or of a line that is one bold phrase.
fn heading(line: &str) -> Option<&str> {
    let line = line.trim();
    if line.starts_with('#') {
        return Some(line.trim_start_matches('#').trim());
    }
    let inner = line.strip_prefix("**")?;
    let inner = inner
        .strip_suffix("**")
        .or_else(|| inner.strip_suffix("**:"))?;
    (!inner.contains("**")).then_some(inner.trim_end_matches(':'))
}

/// A numbered or bulleted item starting in the first column.
fn is_item(line: &str) -> bool {
    if line.starts_with("- ") || line.starts_with("* ") {
        return true;
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    digits > 0 && line[digits..].starts_with(". ")
}

fn clean(cell: &str) -> &str {
    cell.trim_matches(|c: char| c == '`' || c == '*' || c.is_whitespace())
}

fn is_assertion_id(word: &str) -> bool {
    word.starts_with("VAL-") && word.ends_with(|c: char| c.is_ascii_digit())
}

/// `Some(true)` for the first of pass / fail / block that `text` mentions when it is `pass`.
fn verdict(text: &str) -> Option<bool> {
    let lower = text.to_lowercase();
    ["pass", "fail", "block"]
        .into_iter()
        .filter_map(|word| lower.find(word).map(|at| (at, word)))
        .min()
        .map(|(_, word)| word == "pass")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_are_hash_lines_or_one_bold_phrase() {
        assert_eq!(heading("## Blocking issues"), Some("Blocking issues"));
        assert_eq!(
            heading("**Non-blocking issues:**"),
            Some("Non-blocking issues")
        );
        assert_eq!(heading("**Result: pass.** more text"), None);
        assert_eq!(heading("plain"), None);
    }

    #[test]
    fn items_start_in_the_first_column() {
        assert!(is_item("1. one"));
        assert!(is_item("- one"));
        assert!(!is_item("  - nested"));
        assert!(!is_item("1.5 not an item"));
    }

    #[test]
    fn the_first_verdict_word_decides() {
        assert_eq!(verdict(": pass, with a failing note"), Some(true));
        assert_eq!(verdict("(FAIL)"), Some(false));
        assert_eq!(verdict("blocked by a bug"), Some(false));
        assert_eq!(verdict("n/a"), None);
    }
}
