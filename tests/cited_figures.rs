//! The pages the documentation's measured figures were taken from.
//!
//! Several pages cite numbers measured against other pages of this book: the
//! similarity scores in the semantic guide come from two guides and the
//! roadmap, the keyphrase figures come from the errors and methodology
//! references, and the embedder's word figures come from a sweep of every
//! page. Those inputs are live documents. Editing one silently invalidates a
//! figure somewhere else, and nothing notices.
//!
//! That is not hypothetical. Three wrong figures shipped across three review
//! rounds of one branch, and the margin that carries the whole lesson of the
//! semantic example is 0.0138: a near-duplicate pair scores 0.8362 against a
//! cutoff of 0.85, and the example exists to show that the cutoff calls them
//! unrelated. An ordinary edit to either guide can push that pair over the
//! line, at which point two pages teach the opposite of what they say, in
//! prose that still reads correctly.
//!
//! Recomputing the figures here would need the embedding model and the
//! parser, so it would run only in the model-gated lane and would not fire on
//! the edit that caused the drift. This does the cheap thing instead: it pins
//! the content of the pages the figures were measured from. Change one and
//! this test fails, naming the figures to measure again. It is the same shape
//! as the count law in `tests/skill.rs`, which fails when a command escapes
//! its runner rather than trying to prove the command still works.
//!
//! The coverage is narrower for the figure swept over every page. The 141 to
//! 368 word range pins only the two pages that set its ends, so an edit that
//! carries some other page past either end goes unnoticed.
//!
//! When a source page legitimately changes: re-measure the figures the
//! failure names, update them wherever they are cited, then update the digest
//! here. Updating the digest alone is the one thing that defeats the point.

use std::collections::BTreeMap;

/// Each source page, its digest, and what depends on it.
fn pinned() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (
            "site/content/guides/cli.md",
            "17214:e0136af0618f0dff",
            "the 0.8362 pair score in site/content/guides/semantic-clusters.md \
             and skills/matra/references/semantic.md, and the 0.5907 unrelated score",
        ),
        (
            "site/content/guides/rust.md",
            "18204:82b70f0e40a536ac",
            "the 0.8362 pair score and the 0.6080 unrelated score, in the same two files",
        ),
        (
            "site/content/roadmap.md",
            "29344:3b4175273c83c6d7",
            "the 0.5907 and 0.6080 unrelated scores, in the same two files",
        ),
        (
            "site/content/reference/errors.md",
            "25113:4527dfc9417cedf7",
            "the RAKE and YAKE figures in site/content/guides/cli.md: 465 phrases, \
             the two tied at 9.000, 1.190, 215 at the floor, and YAKE's 80.224",
        ),
        (
            "site/content/reference/domain-types.md",
            "32877:efa9451410f35008",
            "the 141-word saturation floor and the 54-word window in \
             site/content/guides/semantic-clusters.md and skills/matra/references/semantic.md",
        ),
        (
            "site/content/explanation/situation-model.md",
            "3823:a6d990f55a916692",
            "the 368-word saturation ceiling in site/content/guides/semantic-clusters.md \
             and skills/matra/references/semantic.md",
        ),
        (
            "site/content/reference/methodology.md",
            "25156:038958d544bafd9e",
            "the RAKE length example in site/content/guides/cli.md: `lexical density` \
             at 5.667 over `model file name` at 5.167",
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
