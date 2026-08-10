//! Integration tests for the `matra` binary.
//!
//! The library is covered by unit tests and the conformance suite; this file
//! covers the application tier, which has its own failure modes: argument
//! parsing, format detection, output shape, and exit codes.
//!
//! Tests that need a parse are `#[ignore]` because they require the UDPipe
//! model:
//!
//!     cargo test --features cli --test cli -- --ignored

#![cfg(feature = "cli")]

use std::io::Write;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;

fn matra() -> Command {
    Command::cargo_bin("matra").expect("binary built with --features cli")
}

fn fixture(contents: &str, suffix: &str) -> NamedTempFile {
    let mut f = tempfile::Builder::new()
        .suffix(suffix)
        .tempfile()
        .expect("temp file");
    f.write_all(contents.as_bytes()).expect("write fixture");
    f.flush().expect("flush fixture");
    f
}

const PROSE: &str = "The committee approved the proposal. Three amendments were submitted \
                     by the working group. The chair adjourned the meeting.";

// ---------------------------------------------------------------------------
// Argument handling. These need no model.
// ---------------------------------------------------------------------------

#[test]
fn help_lists_every_command() {
    matra()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("analyze"))
        .stdout(predicate::str::contains("summarize"))
        .stdout(predicate::str::contains("keyphrases"));
}

#[test]
fn version_is_reported() {
    matra()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn unknown_command_is_rejected() {
    matra().arg("frobnicate").assert().failure();
}

#[test]
fn missing_path_argument_is_rejected() {
    matra().arg("analyze").assert().failure();
}

/// A missing input must fail before the model is touched. Downloading 16 MB
/// only to report that the file does not exist is the wrong order.
#[test]
fn missing_input_file_exits_two_without_loading_the_model() {
    matra()
        .args(["analyze", "/nonexistent/path/to/file.md"])
        .env("MATRA_MODEL_DIR", "/nonexistent/model/dir")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("no such file"));
}

// ---------------------------------------------------------------------------
// Behaviour. These parse, so they need the model.
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires the UDPipe model"]
fn analyze_reports_metrics_and_exits_zero() {
    let f = fixture(PROSE, ".txt");
    matra()
        .args(["analyze", f.path().to_str().unwrap()])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("sentences"))
        .stdout(predicate::str::contains("words"))
        .stdout(predicate::str::contains("passive ratio"));
}

#[test]
#[ignore = "requires the UDPipe model"]
fn analyze_json_is_valid_and_carries_the_document_shape() {
    let f = fixture(PROSE, ".txt");
    let out = matra()
        .args(["analyze", f.path().to_str().unwrap(), "--json"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let parsed: serde_json::Value = serde_json::from_slice(&out).expect("stdout is valid JSON");
    assert!(parsed.get("sections").is_some(), "sections present");
    assert!(
        parsed["sections"][0]["paragraphs"][0]["sentences"][0]["tokens"][0]["lemma"].is_string(),
        "token lemma reachable at the documented path"
    );
}

/// Regression: `summarize` and `keyphrases` once read the file raw and parsed
/// it as plain text, so markdown headings and fenced code were ranked as
/// prose. Both now go through the same format detection `analyze` uses.
#[test]
#[ignore = "requires the UDPipe model"]
fn markdown_structure_is_not_ranked_as_prose() {
    let md = format!("# A Heading\n\n```bash\ncd somewhere && make install\n```\n\n{PROSE}\n");
    let f = fixture(&md, ".md");

    let out = matra()
        .args(["summarize", f.path().to_str().unwrap(), "-n", "3"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).expect("utf8");

    assert!(
        !text.contains("# A Heading"),
        "heading markup ranked as a sentence: {text}"
    );
    assert!(
        !text.contains("make install"),
        "fenced code ranked as a sentence: {text}"
    );

    let phrases = matra()
        .args(["keyphrases", f.path().to_str().unwrap(), "-n", "5"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let phrases = String::from_utf8(phrases).expect("utf8");
    assert!(
        !phrases.contains("make install"),
        "fenced code produced a keyphrase: {phrases}"
    );
}

#[test]
#[ignore = "requires the UDPipe model"]
fn summarize_honours_the_sentence_count() {
    let f = fixture(PROSE, ".txt");
    let out = matra()
        .args(["summarize", f.path().to_str().unwrap(), "-n", "2"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let lines = String::from_utf8(out).expect("utf8").lines().count();
    assert_eq!(lines, 2, "one line per requested sentence");
}

#[test]
#[ignore = "requires the UDPipe model"]
fn both_summary_methods_run() {
    let f = fixture(PROSE, ".txt");
    for method in ["tfidf", "textrank"] {
        matra()
            .args(["summarize", f.path().to_str().unwrap(), "--method", method])
            .assert()
            .code(0);
    }
}

#[test]
#[ignore = "requires the UDPipe model"]
fn both_keyphrase_methods_run() {
    let f = fixture(PROSE, ".txt");
    for method in ["rake", "yake"] {
        matra()
            .args(["keyphrases", f.path().to_str().unwrap(), "--method", method])
            .assert()
            .code(0);
    }
}

/// Nothing found is not an error. Empty input parses fine and yields nothing,
/// which is exit 1, distinct from exit 2 for a genuine failure.
#[test]
#[ignore = "requires the UDPipe model"]
fn empty_input_exits_one_not_two() {
    let f = fixture("", ".txt");
    matra()
        .args(["analyze", f.path().to_str().unwrap()])
        .assert()
        .code(1);
}
