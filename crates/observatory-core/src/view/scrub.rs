//! B-006: mask known credential shapes in a raw line at render time. The store and the session
//! file are never touched; only the text handed to the UI changes.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

/// A raw line as the UI may show it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScrubbedLine {
    pub text: String,
    /// How many token shapes were replaced.
    pub masked: u16,
}

/// `(kind, pattern)` in application order. Patterns are ASCII-only (`(?-u)`), so the regex
/// crate needs no Unicode tables, which keeps the wasm build small. `sk-` needs a word
/// boundary: without it `task-<20 more characters>` is taken for a key.
const SHAPES: [(&str, &str); 8] = [
    (
        "private_key",
        r"-----BEGIN [A-Z ]*PRIVATE KEY-----(?s:.)*?-----END [A-Z ]*PRIVATE KEY-----",
    ),
    ("bearer_token", r"(?i-u)bearer [A-Za-z0-9._~+/=-]{16,}"),
    ("basic_auth", r"(?i-u)basic [A-Za-z0-9+/=]{16,}"),
    (
        "github_token",
        r"(?:gh[pousr]_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{22,})",
    ),
    ("api_key", r"(?-u:\b)sk-[A-Za-z0-9_-]{20,}"),
    ("aws_key", r"AKIA[0-9A-Z]{16}"),
    ("slack_token", r"xox[baprs]-[A-Za-z0-9-]{10,}"),
    (
        "jwt",
        r"eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}",
    ),
];

static COMPILED: LazyLock<Vec<(&'static str, Regex)>> = LazyLock::new(|| {
    SHAPES
        .iter()
        .map(|&(kind, pattern)| {
            let regex = Regex::new(pattern).expect("the token shapes are valid patterns");
            (kind, regex)
        })
        .collect()
});

/// Replaces every known token shape in `raw` with `[redacted:<kind>]`. Bytes that are not
/// UTF-8 are shown lossy, not rejected.
pub fn scrub(raw: &[u8]) -> ScrubbedLine {
    let mut text = String::from_utf8_lossy(raw).into_owned();
    let mut masked: usize = 0;
    for (kind, regex) in COMPILED.iter() {
        let mut found = 0_usize;
        let replaced = regex
            .replace_all(&text, |_: &regex::Captures<'_>| {
                found += 1;
                format!("[redacted:{kind}]")
            })
            .into_owned();
        if found > 0 {
            text = replaced;
            masked += found;
        }
    }
    ScrubbedLine {
        text,
        masked: u16::try_from(masked).unwrap_or(u16::MAX),
    }
}
