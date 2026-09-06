//! Every documented error table agrees with the code that routes errors.
//!
//! Four surfaces tell a reader which Python exception a failure raises:
//! `book/src/reference/errors.md`, `book/src/guides/python.md`, the
//! agent skill's `skills/matra/references/errors.md`, and the typed stub
//! `python/matra/_core.pyi`. Nothing tied any of them to
//! `From<MatraError> for PyErr` in `src/lib.rs`, and the docsite floor
//! cannot help: gate 3 resolves backticked names against `src/`, and
//! `OSError` and `RuntimeError` are Python names that resolve nowhere,
//! so they pass unexamined. The reclassification of transport failures
//! from `ModelInvalid` to `Io` in 0.2.0 changed which `except` clause
//! fires and left three of the four surfaces saying the old thing.
//!
//! So this test reads the routing out of the source and holds the three
//! tables to it. The stub is the one surface it cannot reach: its claims
//! are sentences in docstrings rather than rows, and pinning prose would
//! mean pinning wording. Reviewers own that file.
//!
//! The source is read as text rather than exercised, because the routing
//! lives behind `#[cfg(feature = "python")]` and needs an interpreter to
//! run. Text is enough: what these tables can get wrong is which arm a
//! variant sits in, and that is visible in the arm.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn repo(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn read(relative: &str) -> String {
    fs::read_to_string(repo(relative)).unwrap_or_else(|e| panic!("cannot read {relative}: {e}"))
}

/// The text after `start` and before `end`, or to the end of the input
/// when `end` never comes, which is what a final section looks like.
fn between<'a>(haystack: &'a str, start: &str, end: &str) -> &'a str {
    let from = haystack
        .find(start)
        .unwrap_or_else(|| panic!("marker not found: {start}"))
        + start.len();
    let rest = &haystack[from..];
    &rest[..rest.find(end).unwrap_or(rest.len())]
}

/// Collapse every run of whitespace to one space, so a match arm that
/// wraps across lines reads the same as one that does not.
fn flatten(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A pattern such as `InputTooLarge { .. }` or `Io(_)` reduced to its
/// variant name.
fn variant_of(pattern: &str) -> String {
    pattern
        .trim()
        .trim_start_matches(['{', '}', ',', '|'])
        .trim()
        .split(['(', ' '])
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

/// Variant name to Python exception class, read out of the
/// `From<MatraError> for PyErr` match in `src/lib.rs`.
///
/// The match has no wildcard arm by design, so what this returns is the
/// whole routing rather than a sample of it.
fn routing() -> BTreeMap<String, String> {
    let lib = read("src/lib.rs");
    let arms = flatten(between(
        &lib,
        "impl From<MatraError> for PyErr {",
        "\n    }\n",
    ));
    let arms = &arms[arms.find("match e.0 {").expect("the routing match") + "match e.0 {".len()..];

    let mut out = BTreeMap::new();
    let mut cursor = 0usize;
    while let Some(offset) = arms[cursor..].find("=>") {
        let split = cursor + offset;
        // The cursor is left at the end of the previous arm, so this is
        // exactly this arm's patterns.
        let patterns = &arms[cursor..split];
        let body = &arms[split + 2..];
        let call = body
            .find("::new_err")
            .unwrap_or_else(|| panic!("an arm with no exception class: {body}"));
        let class = body[..call]
            .split_whitespace()
            .next_back()
            .expect("an exception class name")
            .trim_start_matches('{')
            .trim();
        for pattern in patterns.split('|') {
            let variant = variant_of(pattern);
            if !variant.is_empty() {
                out.insert(variant, class.trim_start_matches("Py").to_string());
            }
        }
        // Step over the call's balanced parentheses and whatever closes
        // the arm, so the next pass starts on patterns and nothing else.
        let mut at = split + 2 + call;
        let mut depth = 0usize;
        for (index, ch) in arms[at..].char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
            if depth == 0 && ch == ')' {
                at += index + 1;
                break;
            }
        }
        cursor = at
            + arms[at..]
                .find(|c: char| !c.is_whitespace() && c != '}' && c != ',')
                .unwrap_or(arms.len() - at);
    }
    assert!(out.len() >= 7, "the routing match parsed as {out:?}");
    out
}

/// Variant name to kind string, read out of `Error::kind` in
/// `src/domain.rs`. Also wildcard-free, for the same reason.
fn kinds() -> BTreeMap<String, String> {
    let domain = read("src/domain.rs");
    let body = between(&domain, "pub fn kind(&self) -> &'static str {", "\n    }");
    let mut out = BTreeMap::new();
    for line in body.lines() {
        let Some((pattern, kind)) = line.split_once("=>") else {
            continue;
        };
        let Some(kind) = kind.split('"').nth(1) else {
            continue;
        };
        let variant = variant_of(pattern.trim().trim_start_matches("Error::"));
        if !variant.is_empty() {
            out.insert(variant, kind.to_string());
        }
    }
    out
}

/// Every markdown table row under `heading`, as its cells with the
/// backticks stripped. Stops at the first line that is not a table row.
fn rows(markdown: &str, heading: &str) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    let after = between(markdown, heading, "\n## ");
    for line in after.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            if out.is_empty() {
                continue;
            }
            break;
        }
        let cells: Vec<String> = line
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim().trim_matches('`').to_string())
            .collect();
        // The header and its separator are not rows.
        if cells
            .iter()
            .all(|c| c.chars().all(|ch| ch == '-' || ch == ':'))
        {
            out.clear();
            continue;
        }
        out.push(cells);
    }
    assert!(!out.is_empty(), "no table under {heading}");
    out
}

/// The reference page's variant-to-exception table is the routing.
#[test]
fn the_reference_page_agrees_with_the_pyerr_routing() {
    let page = read("book/src/reference/errors.md");
    let documented: BTreeMap<String, String> = rows(&page, "## Python exception mapping")
        .into_iter()
        .filter(|r| r.len() == 2)
        .map(|r| (r[0].clone(), r[1].clone()))
        .collect();

    assert_eq!(
        documented,
        routing(),
        "book/src/reference/errors.md disagrees with From<MatraError> for PyErr in src/lib.rs"
    );
}

/// The reference page's variant-to-kind table is `Error::kind`.
#[test]
fn the_reference_page_agrees_with_error_kind() {
    let page = read("book/src/reference/errors.md");
    let documented: BTreeMap<String, String> = rows(&page, "## Display strings and kinds")
        .into_iter()
        .filter(|r| r.len() == 3)
        .map(|r| (variant_of(&r[0]), r[2].clone()))
        .collect();

    assert_eq!(
        documented,
        kinds(),
        "book/src/reference/errors.md disagrees with Error::kind in src/domain.rs"
    );
}

/// The Python guide's table names a variant and an exception per row, so
/// every row it carries has to be a row the routing agrees with. It is
/// keyed by situation rather than by variant, so more than one row may
/// name the same variant; none of them may name the wrong exception.
#[test]
fn the_python_guide_agrees_with_the_pyerr_routing() {
    let page = read("book/src/guides/python.md");
    let routing = routing();
    let mut seen = 0usize;
    for row in rows(&page, "## Exceptions") {
        if row.len() != 3 {
            continue;
        }
        let (variant, exception) = (&row[1], &row[2]);
        let Some(expected) = routing.get(variant.as_str()) else {
            continue;
        };
        assert_eq!(
            exception, expected,
            "book/src/guides/python.md maps {variant} to {exception}, the routing says {expected}"
        );
        seen += 1;
    }
    assert_eq!(
        seen,
        routing.len(),
        "the guide's table does not cover every variant"
    );
}

/// The agent skill ships inside the binary, so an agent reading a stale
/// table cannot tell. Its rows are keyed by kind rather than by variant,
/// which is the composition of the two source-of-truth maps.
#[test]
fn the_skill_reference_agrees_with_the_pyerr_routing() {
    let page = read("skills/matra/references/errors.md");
    let (kinds, routing) = (kinds(), routing());

    let expected: BTreeMap<String, String> = kinds
        .iter()
        .map(|(variant, kind)| {
            let exception = routing
                .get(variant)
                .unwrap_or_else(|| panic!("{variant} has a kind but no exception"));
            (kind.clone(), exception.clone())
        })
        .collect();

    let documented: BTreeMap<String, String> = page
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('|'))
        .map(|l| {
            l.trim_matches('|')
                .split('|')
                .map(|c| c.trim().trim_matches('`').to_string())
                .collect::<Vec<_>>()
        })
        .filter(|cells| cells.len() == 4 && expected.contains_key(&cells[0]))
        .map(|cells| (cells[0].clone(), cells[2].clone()))
        .collect();

    assert_eq!(
        documented, expected,
        "skills/matra/references/errors.md disagrees with the routing in src/lib.rs"
    );
}
