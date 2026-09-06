//! The pages the documentation's measured figures were taken from.
//!
//! Several pages cite numbers measured against other pages of this book: the
//! similarity scores in the semantic guide come from two guides and the
//! roadmap, and the keyphrase figures come from the errors reference. Those
//! inputs are live documents. Editing one silently invalidates a figure
//! somewhere else, and nothing notices.
//!
//! That is not hypothetical. Three wrong figures shipped across three review
//! rounds of one branch, and the margin that carries the whole lesson of the
//! semantic example is 0.0166: a near-duplicate pair scores 0.8334 against a
//! cutoff of 0.85, and the example exists to show that the cutoff calls them
//! unrelated. An ordinary edit to either guide can push that pair over the
//! line, at which point two pages teach the opposite of what they say, in
//! prose that still reads correctly.
//!
//! Recomputing the figures here would need the embedding model and the
//! parser, so it would run only in the model-gated lane and would not fire on
//! the edit that caused the drift. This does the cheap thing instead: it pins
//! the content of each source page. Change one and this test fails, naming
//! the figures to measure again. It is the same shape as the count law in
//! `tests/skill.rs`, which fails when a command escapes its runner rather
//! than trying to prove the command still works.
//!
//! When a source page legitimately changes: re-measure the figures the
//! failure names, update them wherever they are cited, then update the digest
//! here. Updating the digest alone is the one thing that defeats the point.

use std::collections::BTreeMap;

/// Each source page, its digest, and what depends on it.
fn pinned() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (
            "book/src/guides/cli.md",
            "16743:db8e1ab6918622ef",
            "the 0.8334 pair score in book/src/guides/semantic-clusters.md \
             and skills/matra/references/semantic.md, and the 0.5917 unrelated score",
        ),
        (
            "book/src/guides/rust.md",
            "17823:7fa3d4e32f0eb6a5",
            "the 0.8334 pair score and the 0.6035 unrelated score, in the same two files",
        ),
        (
            "book/src/roadmap.md",
            "28552:401dc1bf5ab761ac",
            "the 0.5917 and 0.6035 unrelated scores, in the same two files",
        ),
        (
            "book/src/reference/errors.md",
            "16349:6bb85e7bcf54de5a",
            "the RAKE and YAKE figures in book/src/guides/cli.md: 342 phrases, \
             9.000, 1.154, 151 at the floor, and YAKE's 1390.768",
        ),
        (
            "book/src/reference/domain-types.md",
            "32875:1db146b5048bd3c2",
            "the 141-word saturation floor and the 54-word window in \
             book/src/guides/semantic-clusters.md and skills/matra/references/semantic.md",
        ),
    ]
}

fn digest(path: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(root.join(path))
        .unwrap_or_else(|e| panic!("cannot read the pinned source page {path}: {e}"));
    // A length and a cheap rolling sum. This is drift detection, not
    // integrity: the threat is an honest edit, not a forged page, so a
    // dependency on a hash crate would buy nothing the test needs.
    let sum = bytes.iter().fold(0u64, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(u64::from(*b))
    });
    format!("{}:{:016x}", bytes.len(), sum)
}

#[test]
fn every_page_a_cited_figure_was_measured_from_is_pinned() {
    let mut drifted: BTreeMap<&str, (String, &str)> = BTreeMap::new();
    for (path, expected, depends) in pinned() {
        let actual = digest(path);
        if actual != expected {
            drifted.insert(path, (actual, depends));
        }
    }
    assert!(
        drifted.is_empty(),
        "a page that documented figures were measured from has changed.\n\n{}\n\
         Re-measure the figures named above against the current pages, update \
         them everywhere they are cited, and only then update the digest in \
         tests/cited_figures.rs. Updating the digest alone is the one move \
         this test exists to prevent.",
        drifted
            .iter()
            .map(|(path, (actual, depends))| format!(
                "  {path}\n    now: {actual}\n    figures depending on it: {depends}\n"
            ))
            .collect::<Vec<_>>()
            .join("")
    );
}

#[test]
fn the_pin_list_names_pages_that_exist() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for (path, _, _) in pinned() {
        assert!(
            root.join(path).is_file(),
            "tests/cited_figures.rs pins {path}, which is not a file. A pinned \
             page that has moved silently stops guarding anything."
        );
    }
}
