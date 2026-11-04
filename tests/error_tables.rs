//! Every documented error table agrees with the code that routes errors.
//!
//! Four surfaces tell a reader which Python exception a failure raises:
//! `site/content/reference/errors.md`, `site/content/guides/python.md`, the
//! agent skill's `skills/matra/references/errors.md`, and the typed stub
//! `python/matra/_core.pyi`. Nothing tied any of them to the routing in
//! `src/lib.rs`, and the docsite floor cannot help: gate 3 resolves
//! backticked names against `src/`, and `OSError` and `RuntimeError` are
//! Python names that resolve nowhere, so they pass unexamined. The
//! reclassification of transport failures from `ModelInvalid` to `Io` in
//! 0.2.0 changed which `except` clause fires and left three of the four
//! surfaces saying the old thing.
//!
//! So this test reads the routing out of the source and holds the three
//! tables to it: which class each variant raises (`Class::of`), and which
//! builtin each class also derives from (`Class::spec`). The stub is the
//! one surface it cannot reach as a table; `mypy.stubtest` holds its class
//! names to the built module, and the Python suite asserts every class's
//! bases at runtime.
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

/// How one Python class is raised: the `domain::Error` variant behind
/// it, the I/O kind that selects it when the variant is refined by one,
/// and the builtin it also derives from.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Raised {
    variant: String,
    io_kind: Option<String>,
    builtin: String,
}

/// Class name to how it is raised, read out of `Class::of` and
/// `Class::spec` in the Python module of `src/lib.rs`.
///
/// `Class::of` has no wildcard over the variants by design, so what this
/// returns is the whole routing rather than a sample of it. Its one
/// wildcard is inside the `Io` arm, over the I/O kinds, and selects the
/// unrefined `Io` class.
fn routing() -> BTreeMap<String, Raised> {
    let lib = read("src/lib.rs");

    let mut variant_of_class: BTreeMap<String, (String, Option<String>)> = BTreeMap::new();
    let mut outer: Option<String> = None;
    for line in between(
        &lib,
        "fn of(error: &domain::Error) -> Class {",
        "fn spec(self)",
    )
    .lines()
    {
        let Some((pattern, body)) = line.split_once("=>") else {
            continue;
        };
        let pattern = pattern.trim();
        if body.trim().starts_with("match") {
            outer = Some(variant_of(pattern));
            continue;
        }
        let Some(class) = body.trim().strip_prefix("Class::") else {
            continue;
        };
        let class = class.trim_end_matches(',').trim().to_string();
        let raised = match pattern.strip_prefix("std::io::ErrorKind::") {
            Some(io_kind) => (
                outer
                    .clone()
                    .expect("an I/O kind arm inside a variant's arm"),
                Some(io_kind.to_string()),
            ),
            None if pattern == "_" => (
                outer.clone().expect("a wildcard inside a variant's arm"),
                None,
            ),
            None => (variant_of(pattern), None),
        };
        variant_of_class.insert(class, raised);
    }

    let spec = flatten(between(
        &lib,
        "fn spec(self) -> (&'static str, &'static str, Option<Class>) {",
        "fn kind(self)",
    ));
    let mut out = BTreeMap::new();
    for arm in spec.split("Class::").skip(1) {
        let Some((class, rest)) = arm.split_once("=>") else {
            continue;
        };
        let strings: Vec<&str> = rest.split('"').collect();
        let (name, builtin) = (strings[1], strings[3]);
        assert_eq!(class.trim(), name, "a class is named as its arm is");
        let (variant, io_kind) = variant_of_class
            .get(name)
            .unwrap_or_else(|| panic!("{name} has a spec but nothing raises it"))
            .clone();
        out.insert(
            name.to_string(),
            Raised {
                variant,
                io_kind,
                builtin: builtin.to_string(),
            },
        );
    }
    assert_eq!(
        out.len(),
        variant_of_class.len(),
        "every class raised has a spec: {variant_of_class:?}"
    );
    assert!(out.len() >= 9, "the routing parsed as {out:?}");
    out
}

/// The class a variant raises when nothing refines it: the one named as
/// the variant is.
fn base_class(routing: &BTreeMap<String, Raised>, variant: &str) -> Raised {
    let raised = routing
        .get(variant)
        .unwrap_or_else(|| panic!("{variant} raises no class of its own name"))
        .clone();
    assert_eq!(raised.variant, variant);
    raised
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

/// The reference page's class table is the routing: every class, the
/// variant (and I/O kind) that raises it, and the builtin it derives from.
#[test]
fn the_reference_page_agrees_with_the_pyerr_routing() {
    let page = read("site/content/reference/errors.md");
    let documented: BTreeMap<String, Raised> = rows(&page, "## Python exception mapping")
        .into_iter()
        .filter(|r| r.len() == 3)
        .map(|r| {
            // `Io`, or `Io`, kind `NotFound`, or `Io`, any other kind.
            let (variant, refinement) = match r[0].split_once(',') {
                Some((variant, refinement)) => (variant, Some(refinement)),
                None => (r[0].as_str(), None),
            };
            let io_kind = refinement.and_then(|text| {
                text.split('`')
                    .nth(1)
                    .map(str::to_string)
                    .filter(|_| text.contains("kind `"))
            });
            (
                r[1].clone(),
                Raised {
                    variant: variant.trim().trim_matches('`').to_string(),
                    io_kind,
                    builtin: r[2].clone(),
                },
            )
        })
        .collect();

    assert_eq!(
        documented,
        routing(),
        "site/content/reference/errors.md disagrees with Class::of and Class::spec in src/lib.rs"
    );
}

/// The reference page's variant-to-kind table is `Error::kind`.
#[test]
fn the_reference_page_agrees_with_error_kind() {
    let page = read("site/content/reference/errors.md");
    let documented: BTreeMap<String, String> = rows(&page, "## Display strings and kinds")
        .into_iter()
        .filter(|r| r.len() == 3)
        .map(|r| (variant_of(&r[0]), r[2].clone()))
        .collect();

    assert_eq!(
        documented,
        kinds(),
        "site/content/reference/errors.md disagrees with Error::kind in src/domain.rs"
    );
}

/// The Python guide's table names a class and the builtin it also is,
/// per row, so every row it carries has to agree with the routing. It is
/// keyed by situation rather than by class, so more than one row may name
/// a class; none may name the wrong builtin, and every class appears.
#[test]
fn the_python_guide_agrees_with_the_pyerr_routing() {
    let page = read("site/content/guides/python.md");
    let routing = routing();
    let mut seen = std::collections::BTreeSet::new();
    for row in rows(&page, "## Exceptions") {
        if row.len() != 3 {
            continue;
        }
        let (class, builtin) = (&row[1], &row[2]);
        let raised = routing.get(class.as_str()).unwrap_or_else(|| {
            panic!("site/content/guides/python.md names {class}, which matra does not raise")
        });
        assert_eq!(
            builtin, &raised.builtin,
            "site/content/guides/python.md says {class} is a {builtin}, the routing says {}",
            raised.builtin
        );
        seen.insert(class.clone());
    }
    assert_eq!(
        seen.len(),
        routing.len(),
        "the guide's table does not cover every class"
    );
}

/// The agent skill ships inside the binary, so an agent reading a stale
/// table cannot tell. Its rows are keyed by kind rather than by variant,
/// which is the composition of the two source-of-truth maps, and name the
/// class the variant raises when no I/O kind refines it.
#[test]
fn the_skill_reference_agrees_with_the_pyerr_routing() {
    let page = read("skills/matra/references/errors.md");
    let (kinds, routing) = (kinds(), routing());

    let expected: BTreeMap<String, (String, String)> = kinds
        .iter()
        .map(|(variant, kind)| {
            let raised = base_class(&routing, variant);
            (kind.clone(), (variant.clone(), raised.builtin))
        })
        .collect();

    let documented: BTreeMap<String, (String, String)> = page
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('|'))
        .map(|l| {
            l.trim_matches('|')
                .split('|')
                .map(|c| c.trim().trim_matches('`').to_string())
                .collect::<Vec<_>>()
        })
        .filter(|cells| cells.len() == 5 && expected.contains_key(&cells[0]))
        .map(|cells| (cells[0].clone(), (cells[2].clone(), cells[3].clone())))
        .collect();

    assert_eq!(
        documented, expected,
        "skills/matra/references/errors.md disagrees with the routing in src/lib.rs"
    );
}
