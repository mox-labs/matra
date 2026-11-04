//! The cross-surface parity lane (EPR-0006): the same files through Rust,
//! Python and the command line give the same `Document`, the same summary,
//! the same keyphrases and the same error, as `spec/tests/parity/parity.json`
//! pins them.
//!
//! This file runs two of the three surfaces: the library, and the command
//! line as its binary, in a subprocess with no configuration file, so the
//! defaults it applies are the shipped ones. `python/tests/test_parity.py`
//! runs the third against the same fixture. A drift on any one surface
//! fails that surface's runner.
//!
//! Every test here needs the UDPipe model, so all are ignored by default:
//!
//!     cargo test --features cli --test parity -- --ignored
//!
//! With `MATRA_REGENERATE_PARITY=1` the library test rewrites every
//! expectation from the library instead. Read what changed before
//! committing it.

#![cfg(feature = "udpipe")]

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Serialize)]
struct Fixture {
    name: String,
    description: String,
    documents: Vec<DocumentCase>,
    errors: Vec<ErrorCase>,
}

#[derive(Deserialize, Serialize)]
struct DocumentCase {
    file: String,
    input: String,
    summaries: Vec<Call>,
    keyphrases: Vec<Call>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    expect: Option<DocumentExpect>,
}

/// One outcome call. An absent `n` or `method` is the surface's default.
#[derive(Deserialize, Serialize, Clone)]
struct Call {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    n: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    method: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct DocumentExpect {
    document: Value,
    summaries: Vec<Value>,
    keyphrases: Vec<Value>,
}

#[derive(Deserialize, Serialize)]
struct ErrorCase {
    file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bytes: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fill: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    expect: Option<Value>,
}

impl ErrorCase {
    fn contents(&self) -> Vec<u8> {
        match (&self.bytes, &self.fill, self.count) {
            (Some(bytes), None, None) => bytes.clone(),
            (None, Some(fill), Some(count)) => fill.repeat(count).into_bytes(),
            _ => panic!(
                "{}: an error case is `bytes`, or `fill` and `count`",
                self.file
            ),
        }
    }
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("spec/tests/parity/parity.json")
}

fn fixture() -> Fixture {
    let path = fixture_path();
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("malformed fixture {}: {e}", path.display()))
}

/// The JSON of `value` as a reader of the fixture sees it: written, then
/// read back, so a float compares as the text the fixture holds.
fn json_of<T: Serialize>(value: &T) -> Value {
    serde_json::from_str(&serde_json::to_string(value).unwrap()).unwrap()
}

/// A result's JSON with what legitimately differs between runs set aside:
/// the matra version, which moves with every release, and the path, which
/// is a different temporary directory on every run and is checked to be
/// the one passed before it is cleared.
fn normalized(mut value: Value, passed: &Path) -> Value {
    let provenance = &mut value["provenance"];
    assert_eq!(
        provenance["source"]["path"].as_str(),
        Some(passed.to_str().unwrap()),
        "the result names the path it was given"
    );
    provenance["source"]["path"] = Value::Null;
    provenance["matra"] = Value::String("fixture".to_string());
    value
}

/// The default `n` the command line applies: the shipped configuration's.
fn default_n(command: &str) -> usize {
    let cfg = matra::config::Config::from_sources(
        |key| (key == "HOME").then(|| "/nonexistent-home".to_string()),
        None,
    )
    .unwrap();
    match command {
        "summarize" => cfg.summarize_n(),
        "keyphrases" => cfg.keyphrases_n(),
        other => panic!("no default n for {other}"),
    }
}

fn summary_method(name: &str) -> matra::SummaryMethod {
    match name {
        "tfidf" => matra::SummaryMethod::TfIdf,
        "textrank" => matra::SummaryMethod::TextRank,
        other => panic!("the fixture names a summary method matra lacks: {other}"),
    }
}

fn keyphrase_method(name: &str) -> matra::KeyphraseMethod {
    match name {
        "rake" => matra::KeyphraseMethod::Rake,
        "yake" => matra::KeyphraseMethod::Yake,
        other => panic!("the fixture names a keyphrase method matra lacks: {other}"),
    }
}

/// The Rust surface: one file through the engine and the outcome
/// functions, normalized.
fn through_rust(
    engine: &matra::Engine,
    case: &DocumentCase,
    path: &Path,
) -> (Value, Vec<Value>, Vec<Value>) {
    let doc = engine.analyze(matra::Input::file(path)).unwrap();
    let summaries = case
        .summaries
        .iter()
        .map(|call| {
            let n = call.n.unwrap_or_else(|| default_n("summarize"));
            let got = match &call.method {
                None => matra::summarize(&doc, n),
                Some(m) => matra::summarize_with(&doc, n, summary_method(m)),
            };
            normalized(json_of(&got.unwrap()), path)
        })
        .collect();
    let keyphrases = case
        .keyphrases
        .iter()
        .map(|call| {
            let n = call.n.unwrap_or_else(|| default_n("keyphrases"));
            let got = match &call.method {
                None => matra::keyphrases(&doc, n),
                Some(m) => matra::keyphrases_with(&doc, n, keyphrase_method(m)),
            };
            normalized(json_of(&got.unwrap()), path)
        })
        .collect();
    (normalized(json_of(&doc), path), summaries, keyphrases)
}

/// The error the Rust surface returns for an error case's file, as its
/// serde form.
fn error_through_rust(engine: &matra::Engine, path: &Path) -> Value {
    match engine.analyze(matra::Input::file(path)) {
        Ok(_) => panic!("{} analyzed; the fixture expects a failure", path.display()),
        Err(e) => json_of(&e),
    }
}

#[test]
#[ignore = "requires the UDPipe model"]
fn the_library_matches_the_fixture() {
    let mut fixture = fixture();
    let engine = matra::Engine::new().expect("the UDPipe model loads");
    let dir = tempfile::tempdir().unwrap();
    let regenerate = std::env::var_os("MATRA_REGENERATE_PARITY").is_some();

    for case in &mut fixture.documents {
        let path = dir.path().join(&case.file);
        std::fs::write(&path, &case.input).unwrap();
        let (document, summaries, keyphrases) = through_rust(&engine, case, &path);
        if regenerate {
            case.expect = Some(DocumentExpect {
                document,
                summaries,
                keyphrases,
            });
            continue;
        }
        let expect = case.expect.as_ref().expect("regenerate the fixture");
        assert_eq!(document, expect.document, "{}: the document", case.file);
        assert_eq!(summaries, expect.summaries, "{}: the summaries", case.file);
        assert_eq!(
            keyphrases, expect.keyphrases,
            "{}: the keyphrases",
            case.file
        );
    }
    for case in &mut fixture.errors {
        let path = dir.path().join(&case.file);
        std::fs::write(&path, case.contents()).unwrap();
        let got = error_through_rust(&engine, &path);
        if regenerate {
            case.expect = Some(got);
            continue;
        }
        assert_eq!(Some(&got), case.expect.as_ref(), "{}: the error", case.file);
    }

    if regenerate {
        let written = serde_json::to_string_pretty(&fixture).unwrap() + "\n";
        std::fs::write(fixture_path(), written).unwrap();
    }
}

/// Run the binary on `args` with no configuration file, returning the exit
/// code, stdout and stderr.
#[cfg(feature = "cli")]
fn matra_cli(args: &[&str], home: &Path) -> (i32, String, String) {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_matra"))
        .args(args)
        .env("MATRA_CONFIG_FILE", home.join("no-config.toml"))
        .output()
        .expect("the binary runs");
    (
        output.status.code().expect("an exit code"),
        String::from_utf8(output.stdout).expect("stdout is UTF-8"),
        String::from_utf8(output.stderr).expect("stderr is UTF-8"),
    )
}

/// The `result` of a successful `--json` run.
#[cfg(feature = "cli")]
fn cli_result(args: &[&str], home: &Path) -> Value {
    let (code, out, err) = matra_cli(args, home);
    assert_eq!(
        code,
        0,
        "matra {}: exit {code}, stderr {err}",
        args.join(" ")
    );
    let envelope: Value = serde_json::from_str(&out).expect("the envelope is JSON");
    envelope["result"].clone()
}

/// A call's command-line flags: `-n` and `--method` only when the case
/// names them, so an empty case runs the command line's defaults.
#[cfg(feature = "cli")]
fn flags(call: &Call) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(n) = call.n {
        out.extend(["-n".to_string(), n.to_string()]);
    }
    if let Some(method) = &call.method {
        out.extend(["--method".to_string(), method.clone()]);
    }
    out
}

#[cfg(feature = "cli")]
#[test]
#[ignore = "requires the UDPipe model"]
fn the_command_line_matches_the_fixture() {
    let fixture = fixture();
    let dir = tempfile::tempdir().unwrap();

    for case in &fixture.documents {
        let path = dir.path().join(&case.file);
        std::fs::write(&path, &case.input).unwrap();
        let file = path.to_str().unwrap();
        let expect = case.expect.as_ref().expect("regenerate the fixture");

        let document = normalized(cli_result(&["analyze", file, "--json"], dir.path()), &path);
        assert_eq!(document, expect.document, "{}: the document", case.file);

        for (command, calls, expected) in [
            ("summarize", &case.summaries, &expect.summaries),
            ("keyphrases", &case.keyphrases, &expect.keyphrases),
        ] {
            for (call, want) in calls.iter().zip(expected) {
                let extra = flags(call);
                let mut args = vec![command, file, "--json"];
                args.extend(extra.iter().map(String::as_str));
                let got = normalized(cli_result(&args, dir.path()), &path);
                assert_eq!(&got, want, "{}: matra {}", case.file, args.join(" "));
            }
        }
    }

    for case in &fixture.errors {
        let path = dir.path().join(&case.file);
        std::fs::write(&path, case.contents()).unwrap();
        let expect = case.expect.as_ref().expect("regenerate the fixture");
        let message = expect["message"].as_str().unwrap();

        let (code, out, err) =
            matra_cli(&["analyze", path.to_str().unwrap(), "--json"], dir.path());
        assert_eq!(code, 2, "{}: a failure exits 2", case.file);
        assert_eq!(out, "", "{}: nothing on stdout", case.file);
        assert_eq!(
            err,
            format!("matra: {}: {message}\n", path.display()),
            "{}: the message on stderr",
            case.file
        );
    }
}
