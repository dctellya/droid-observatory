//! The oracle: the engine's CSV against a reference CSV keyed by (`scope`, `id`), within a
//! tolerance per column kind (SPEC §3 M5: 1 s, 1 token, 0.01 USD; `throttled_minutes` 1 minute,
//! `rework_windows` and `price_table` exact).

use std::collections::BTreeMap;

use thiserror::Error;

use super::csv::CSV_HEADER;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tolerance {
    pub seconds: f64,
    pub tokens: f64,
    pub usd: f64,
    pub minutes: f64,
}

impl Default for Tolerance {
    fn default() -> Self {
        Self {
            seconds: 1.0,
            tokens: 1.0,
            usd: 0.01,
            minutes: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviationKind {
    /// A cell differs by more than its tolerance.
    Outside,
    /// A reference row the engine does not produce.
    Missing,
    /// An engine row the reference does not have.
    Extra,
}

impl DeviationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Outside => "outside",
            Self::Missing => "missing",
            Self::Extra => "extra",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Deviation {
    pub scope: String,
    pub id: String,
    /// The column; empty for a missing or extra row.
    pub column: String,
    pub reference: String,
    pub produced: String,
    pub kind: DeviationKind,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OracleDiff {
    pub rows_compared: usize,
    pub deviations: Vec<Deviation>,
    /// The largest absolute difference per numeric column over the rows compared (the
    /// tolerance's unit: seconds, tokens, USD, minutes, windows).
    pub max_by_column: BTreeMap<String, f64>,
}

#[derive(Debug, Error)]
pub enum OracleError {
    #[error("the {which} CSV header is not the 22-name contract: {found}")]
    Header { which: &'static str, found: String },
    #[error("the {which} CSV line {line} has {found} fields, the contract has {wanted}")]
    Row {
        which: &'static str,
        line: usize,
        found: usize,
        wanted: usize,
    },
    #[error("the {which} CSV row {scope} {id}: {column} `{cell}` is not a number")]
    Cell {
        which: &'static str,
        scope: String,
        id: String,
        column: &'static str,
        cell: String,
    },
}

/// RFC 4180: quoted cells with doubled quotes, `\n` or `\r\n` line ends, blank lines skipped.
fn parse(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match (quoted, c) {
            (true, '"') if chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            (true, '"') => quoted = false,
            (true, _) => cell.push(c),
            (false, '"') => quoted = true,
            (false, ',') => row.push(std::mem::take(&mut cell)),
            (false, '\r') => {}
            (false, '\n') => {
                row.push(std::mem::take(&mut cell));
                rows.push(std::mem::take(&mut row));
            }
            (false, _) => cell.push(c),
        }
    }
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        rows.push(row);
    }
    rows.retain(|row| !(row.len() == 1 && row[0].is_empty()));
    rows
}

type Table = BTreeMap<(String, String), Vec<String>>;

fn table(text: &str, which: &'static str) -> Result<Table, OracleError> {
    let rows = parse(text);
    let header = rows.first().cloned().unwrap_or_default();
    if header != CSV_HEADER {
        return Err(OracleError::Header {
            which,
            found: header.join(","),
        });
    }
    let mut found = Table::new();
    for (index, row) in rows.into_iter().enumerate().skip(1) {
        if row.len() != CSV_HEADER.len() {
            return Err(OracleError::Row {
                which,
                line: index + 1,
                found: row.len(),
                wanted: CSV_HEADER.len(),
            });
        }
        found.insert((row[0].clone(), row[1].clone()), row);
    }
    Ok(found)
}

fn number(
    cell: &str,
    which: &'static str,
    key: &(String, String),
    column: &'static str,
) -> Result<f64, OracleError> {
    cell.trim().parse().map_err(|_| OracleError::Cell {
        which,
        scope: key.0.clone(),
        id: key.1.clone(),
        column,
        cell: cell.to_owned(),
    })
}

fn tolerance_of(column: &str, tolerance: &Tolerance) -> f64 {
    match column {
        "rework_windows" => 0.0,
        "throttled_minutes" => tolerance.minutes,
        "cost_usd" => tolerance.usd,
        name if name.starts_with("tokens_") => tolerance.tokens,
        _ => tolerance.seconds,
    }
}

/// Compares `produced` (the engine's CSV) with `reference`. A row in one and not the other is a
/// deviation; the `name` column is a label and is not compared.
pub fn compare_csv(
    reference: &str,
    produced: &str,
    tolerance: &Tolerance,
) -> Result<OracleDiff, OracleError> {
    let reference = table(reference, "reference")?;
    let produced = table(produced, "engine")?;
    let mut diff = OracleDiff::default();
    let deviation =
        |key: &(String, String), column: &str, reference: &str, produced: &str, kind| Deviation {
            scope: key.0.clone(),
            id: key.1.clone(),
            column: column.to_owned(),
            reference: reference.to_owned(),
            produced: produced.to_owned(),
            kind,
        };
    for (key, wanted) in &reference {
        let Some(have) = produced.get(key) else {
            diff.deviations.push(deviation(
                key,
                "",
                "present",
                "absent",
                DeviationKind::Missing,
            ));
            continue;
        };
        diff.rows_compared += 1;
        for (index, column) in CSV_HEADER.iter().enumerate().skip(3) {
            if *column == "price_table" {
                if wanted[index] != have[index] {
                    diff.deviations.push(deviation(
                        key,
                        column,
                        &wanted[index],
                        &have[index],
                        DeviationKind::Outside,
                    ));
                }
                continue;
            }
            let a = number(&wanted[index], "reference", key, column)?;
            let b = number(&have[index], "engine", key, column)?;
            let gap = (a - b).abs();
            let max = diff
                .max_by_column
                .entry((*column).to_owned())
                .or_insert(0.0);
            *max = max.max(gap);
            if gap > tolerance_of(column, tolerance) + 1e-9 {
                diff.deviations.push(deviation(
                    key,
                    column,
                    &wanted[index],
                    &have[index],
                    DeviationKind::Outside,
                ));
            }
        }
    }
    for key in produced.keys().filter(|key| !reference.contains_key(*key)) {
        diff.deviations.push(deviation(
            key,
            "",
            "absent",
            "present",
            DeviationKind::Extra,
        ));
    }
    Ok(diff)
}
