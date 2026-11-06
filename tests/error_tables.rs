//! Every documented error table agrees with the code that routes errors.
//!
//! Four surfaces tell a reader which Python exception a failure raises:
//! `site/content/reference/errors.md`, `site/content/guides/python.md`, the
//! agent skill's `skills/matra/references/errors.md`, and the typed stub
//! `python/matra/_core.pyi`. Nothing tied any of them to the routing in
//! `src/python/errors.rs`, and the docsite floor cannot help: gate 3 resolves
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
/// `Class::spec` in `src/python/errors.rs`.
///
/// `Class::of` has no wildcard over the variants by design, so what this
/// returns is the whole routing rather than a sample of it. Its one
/// wildcard is inside the `Io` arm, over the I/O kinds, and selects the
/// unrefined `Io` class.
fn routing() -> BTreeMap<String, Raised> {
    let lib = read("src/python/errors.rs");

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
        // The Python name is the Rust arm's with PEP 8's suffix, except
        // that `IoPermissionDenied` shortens to `IoPermissionError`, as the
        // builtin it derives from is named.
        let arm = class.trim();
        assert!(
            name.ends_with("Error") && name.starts_with(arm.trim_end_matches("Denied")),
            "{arm} raises as {name}, which is not its name with the Error suffix"
        );
        let (variant, io_kind) = variant_of_class
            .get(arm)
            .unwrap_or_else(|| panic!("{arm} has a spec but nothing raises it"))
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

/// The class a variant raises when no I/O kind refines it, by name.
fn base_class(routing: &BTreeMap<String, Raised>, variant: &str) -> (String, Raised) {
    let mut unrefined = routing
        .iter()
        .filter(|(_, raised)| raised.variant == variant && raised.io_kind.is_none());
    let (name, raised) = unrefined
        .next()
        .unwrap_or_else(|| panic!("{variant} raises no unrefined class"));
    assert!(
        unrefined.next().is_none(),
        "{variant} raises two unrefined classes"
    );
    (name.clone(), raised.clone())
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
        "site/content/reference/errors.md disagrees with Class::of and Class::spec in src/python/errors.rs"
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
            let (class, raised) = base_class(&routing, variant);
            (kind.clone(), (class, raised.builtin))
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
        "skills/matra/references/errors.md disagrees with the routing in src/python/errors.rs"
    );
}

/// Every `reason` an `invalid_input` can carry, read out of the library
/// source: the `reason: "..."` of each `Error::InvalidInput` it builds,
/// and the first argument of the two helpers that build one, the command
/// line's `Failure::refused` and the Python binding's `invalid_input`.
/// Test modules are left out, since what they build is not raised.
fn reasons_in_source() -> std::collections::BTreeSet<String> {
    fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).expect("read src") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    walk(&repo("src"), &mut files);
    let mut found = std::collections::BTreeSet::new();
    for file in files {
        let text = fs::read_to_string(&file).expect("read source");
        let code =
            flatten(text.split("#[cfg(test)]\nmod tests").next().unwrap_or("")).replace("( ", "(");
        for marker in [
            "InvalidInput { reason: \"",
            "refused(\"",
            "invalid_input(\"",
        ] {
            for piece in code.split(marker).skip(1) {
                let name: String = piece
                    .chars()
                    .take_while(|c| c.is_ascii_lowercase() || *c == '_' || c.is_ascii_digit())
                    .collect();
                if piece[name.len()..].starts_with('"') && !name.is_empty() {
                    found.insert(name);
                }
            }
        }
    }
    found
}

/// The errors reference lists every `invalid_input` reason the library
/// raises, and nothing it does not: `reason` is the stable name a caller
/// branches on, so a check added without a row, or a row left behind by a
/// check removed, is a contract the page misstates.
#[test]
fn the_reference_page_lists_every_invalid_input_reason() {
    let page = read("site/content/reference/errors.md");
    let section = between(&page, "### InvalidInput", "### UnsupportedFormat");
    let documented: std::collections::BTreeSet<String> = rows(section, "")
        .into_iter()
        .map(|r| r[0].clone())
        .collect();
    let raised = reasons_in_source();
    assert!(
        raised.len() > 20,
        "the source scan found only {raised:?}: the markers no longer match how the source builds the error"
    );
    assert_eq!(
        documented, raised,
        "site/content/reference/errors.md (### InvalidInput) disagrees with the reasons src/ raises"
    );
}
