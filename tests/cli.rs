//! Tests for the matra command line.
//!
//! The command is a library module now, so these drive [`matra::cli::run`]
//! in process with captured buffers. Nothing here needs a built binary,
//! and a failure points at a function rather than at a subprocess.
//!
//! Three tests are the exception and spawn the binary, for a reason worth
//! stating: `config show`, `config init`, and `-` for stdin read the
//! process's environment and the process's stdin. Setting either from
//! inside a test would mutate state every other test in this file shares,
//! which is precisely the flakiness the plan's risk list names. A
//! subprocess gets its own environment and its own stdin, so those three
//! are isolated instead of racing. The hermetic half of the same ground
//! (atomic write, refuse-without-force, every resolved key renderable) is
//! covered by unit tests in `src/cli/config_cmd.rs`.
//!
//! Tests that need a parse are `#[ignore]` because they require the
//! UDPipe model:
//!
//!     cargo test --features cli --test cli -- --ignored

#![cfg(feature = "cli")]

use std::ffi::OsString;
use std::io::Write;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;

const PROSE: &str = "The committee approved the proposal. Three amendments were submitted \
                     by the working group. The chair adjourned the meeting.";

/// Drive the CLI and collect everything it produced.
fn run(args: &[&str]) -> (u8, String, String) {
    let argv = std::iter::once(OsString::from("matra")).chain(args.iter().map(OsString::from));
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = matra::cli::run(argv, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).expect("stdout is utf8"),
        String::from_utf8(err).expect("stderr is utf8"),
    )
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

fn path_of(f: &NamedTempFile) -> &str {
    f.path().to_str().expect("utf8 path")
}

/// A binary invocation with the environment scrubbed down to a temporary
/// home, so no test reads or writes the developer's real config.
fn scoped(home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("matra").expect("binary built with --features cli");
    cmd.env_clear()
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_DATA_HOME", home.join("data"))
        .env("PATH", std::env::var("PATH").unwrap_or_default());
    cmd
}

// ---------------------------------------------------------------------------
// Argument handling. These need no model.
// ---------------------------------------------------------------------------

#[test]
fn help_lists_every_command() {
    let (code, out, _) = run(&["--help"]);
    assert_eq!(code, 0);
    for command in [
        "analyze",
        "summarize",
        "keyphrases",
        "config",
        "completions",
    ] {
        assert!(out.contains(command), "missing `{command}` in:\n{out}");
    }
}

/// `--help` must read the same from either launcher, which is why the
/// program name is set explicitly rather than taken from `argv[0]`.
#[test]
fn help_names_the_program_whatever_argv_zero_says() {
    let argv = ["/opt/weird/path/to/matra-0.1.0", "--help"]
        .into_iter()
        .map(OsString::from);
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    matra::cli::run(argv, &mut out, &mut err);
    let out = String::from_utf8(out).expect("utf8");
    assert!(out.contains("Usage: matra"), "{out}");
    assert!(!out.contains("matra-0.1.0"), "{out}");
}

#[test]
fn version_names_the_version_then_the_compiled_features() {
    let (code, out, _) = run(&["--version"]);
    assert_eq!(code, 0);
    let mut lines = out.lines();
    assert_eq!(
        lines.next(),
        Some(format!("matra {}", env!("CARGO_PKG_VERSION")).as_str())
    );
    let features = lines.next().expect("a features line");
    assert!(features.starts_with("features:"), "{features}");
    // This test only compiles under `cli`, which implies `udpipe`.
    assert!(features.contains("udpipe"), "{features}");
    assert!(features.contains("cli"), "{features}");
}

#[test]
fn unknown_command_is_rejected_on_stderr() {
    let (code, out, err) = run(&["frobnicate"]);
    assert_eq!(code, 2);
    assert!(out.is_empty(), "usage errors do not go to stdout: {out}");
    assert!(!err.is_empty());
}

#[test]
fn missing_path_argument_is_rejected() {
    let (code, _, err) = run(&["analyze"]);
    assert_eq!(code, 2);
    assert!(!err.is_empty());
}

/// A missing input must fail before the model is touched. Downloading 16 MB
/// only to report that the file does not exist is the wrong order.
#[test]
fn missing_input_file_exits_two_without_loading_the_model() {
    let (code, _, err) = run(&["analyze", "/nonexistent/path/to/file.md"]);
    assert_eq!(code, 2);
    assert!(err.contains("no such file"), "{err}");
}

#[test]
fn completions_generate_for_every_supported_shell() {
    for shell in ["bash", "zsh", "fish"] {
        let (code, out, err) = run(&["completions", shell]);
        assert_eq!(code, 0, "{shell}: {err}");
        assert!(out.contains("matra"), "{shell} script names the command");
        assert!(out.len() > 200, "{shell} script looks truncated");
    }
}

#[test]
fn an_unknown_shell_is_rejected() {
    let (code, _, _) = run(&["completions", "powershell"]);
    assert_eq!(code, 2);
}

// ---------------------------------------------------------------------------
// Behaviour. These parse, so they need the model.
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires the UDPipe model"]
fn analyze_reports_metrics_and_exits_zero() {
    let f = fixture(PROSE, ".txt");
    let (code, out, err) = run(&["analyze", path_of(&f)]);
    assert_eq!(code, 0, "{err}");
    for row in ["sentences", "words", "passive ratio"] {
        assert!(out.contains(row), "missing `{row}` in:\n{out}");
    }
}

/// `--sections` was the one thing the Python CLI could do and the Rust
/// binary could not. It is here now.
#[test]
#[ignore = "requires the UDPipe model"]
fn sections_adds_the_per_section_breakdown() {
    let md = format!("# A Heading\n\n{PROSE}\n\n## Another\n\n{PROSE}\n");
    let f = fixture(&md, ".md");

    let (code, plain, _) = run(&["analyze", path_of(&f)]);
    assert_eq!(code, 0);
    assert!(
        !plain.contains("A Heading"),
        "no breakdown without the flag"
    );

    let (code, out, err) = run(&["analyze", path_of(&f), "--sections"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("paragraphs"), "{out}");
    assert!(out.contains("A Heading"), "{out}");
    assert!(out.contains("Another"), "{out}");
    assert!(out.contains("h1"), "{out}");
    assert!(out.contains("h2"), "{out}");
}

#[test]
#[ignore = "requires the UDPipe model"]
fn quiet_suppresses_the_table_and_keeps_the_exit_code() {
    let f = fixture(PROSE, ".txt");
    let (code, out, _) = run(&["analyze", path_of(&f), "--quiet"]);
    assert_eq!(code, 0);
    assert!(out.is_empty(), "{out}");

    let empty = fixture("", ".txt");
    let (code, out, _) = run(&["analyze", path_of(&empty), "--quiet"]);
    assert_eq!(code, 1, "quiet does not change what was found");
    assert!(out.is_empty(), "{out}");
}

#[test]
#[ignore = "requires the UDPipe model"]
fn quiet_does_not_affect_json() {
    let f = fixture(PROSE, ".txt");
    let (code, quiet, _) = run(&["analyze", path_of(&f), "--json", "--quiet"]);
    let (_, loud, _) = run(&["analyze", path_of(&f), "--json"]);
    assert_eq!(code, 0);
    assert_eq!(quiet, loud);
    assert!(!quiet.is_empty());
}

#[test]
#[ignore = "requires the UDPipe model"]
fn color_is_explicit_when_asked_and_absent_when_refused() {
    let f = fixture(PROSE, ".txt");
    let (_, colored, _) = run(&["analyze", path_of(&f), "--color", "always"]);
    let (_, plain, _) = run(&["analyze", path_of(&f), "--color", "never"]);
    assert!(colored.contains('\x1b'), "no escapes under --color always");
    assert!(!plain.contains('\x1b'), "escapes under --color never");
}

/// Nothing found is not an error. Empty input parses fine and yields nothing,
/// which is exit 1, distinct from exit 2 for a genuine failure.
#[test]
#[ignore = "requires the UDPipe model"]
fn empty_input_exits_one_not_two() {
    let f = fixture("", ".txt");
    let (code, _, _) = run(&["analyze", path_of(&f)]);
    assert_eq!(code, 1);
}

/// Regression: `summarize` and `keyphrases` once read the file raw and parsed
/// it as plain text, so markdown headings and fenced code were ranked as
/// prose. Both now go through the same format detection `analyze` uses.
#[test]
#[ignore = "requires the UDPipe model"]
fn markdown_structure_is_not_ranked_as_prose() {
    let md = format!("# A Heading\n\n```bash\ncd somewhere && make install\n```\n\n{PROSE}\n");
    let f = fixture(&md, ".md");

    let (_, summary, _) = run(&["summarize", path_of(&f), "-n", "3"]);
    assert!(
        !summary.contains("# A Heading"),
        "heading markup ranked as a sentence: {summary}"
    );
    assert!(
        !summary.contains("make install"),
        "fenced code ranked as a sentence: {summary}"
    );

    let (_, phrases, _) = run(&["keyphrases", path_of(&f), "-n", "5"]);
    assert!(
        !phrases.contains("make install"),
        "fenced code produced a keyphrase: {phrases}"
    );
}

#[test]
#[ignore = "requires the UDPipe model"]
fn summarize_honours_the_sentence_count() {
    let f = fixture(PROSE, ".txt");
    let (code, out, err) = run(&["summarize", path_of(&f), "-n", "2"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out.lines().count(), 2, "one line per requested sentence");
}

#[test]
#[ignore = "requires the UDPipe model"]
fn both_summary_methods_run() {
    let f = fixture(PROSE, ".txt");
    for method in ["tfidf", "textrank"] {
        let (code, _, err) = run(&["summarize", path_of(&f), "--method", method]);
        assert_eq!(code, 0, "{method}: {err}");
    }
}

#[test]
#[ignore = "requires the UDPipe model"]
fn both_keyphrase_methods_run() {
    let f = fixture(PROSE, ".txt");
    for method in ["rake", "yake"] {
        let (code, _, err) = run(&["keyphrases", path_of(&f), "--method", method]);
        assert_eq!(code, 0, "{method}: {err}");
    }
}

// ---------------------------------------------------------------------------
// The JSON envelope
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires the UDPipe model"]
fn every_command_emits_the_same_envelope() {
    let f = fixture(PROSE, ".txt");
    let path = path_of(&f).to_string();

    for (args, command) in [
        (vec!["analyze", &path, "--json"], "analyze"),
        (vec!["summarize", &path, "--json"], "summarize"),
        (vec!["keyphrases", &path, "--json"], "keyphrases"),
        (vec!["config", "show", "--json"], "config"),
    ] {
        let (code, out, err) = run(&args);
        assert_eq!(code, 0, "{command}: {err}");
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        let object = parsed.as_object().expect("an object");
        let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["command", "format_version", "input", "result"]);
        assert_eq!(parsed["format_version"], 1);
        assert_eq!(parsed["command"], command);
        assert!(parsed["input"].is_string());
    }
}

/// The `result` value is the serde form of the domain type, unchanged.
/// A field that moved would show up here rather than in a consumer.
#[test]
#[ignore = "requires the UDPipe model"]
fn the_result_is_the_domain_value_unchanged() {
    let f = fixture(PROSE, ".txt");
    let (_, out, _) = run(&["analyze", path_of(&f), "--json"]);
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    let result = &parsed["result"];
    assert!(result["sections"].is_array());
    assert!(
        result["sections"][0]["paragraphs"][0]["sentences"][0]["tokens"][0]["lemma"].is_string(),
        "token lemma reachable at the documented path"
    );
}

/// The shared CLI fixture, run through the Rust launcher. The Python
/// runner in `python/tests/test_cli.py` asserts the same file, which is
/// what makes the two launchers checkable against one contract rather
/// than against each other's output.
#[test]
#[ignore = "requires the UDPipe model"]
fn envelope_matches_the_conformance_fixture() {
    let spec: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("spec/tests/cli/envelope.json"),
        )
        .expect("read fixture"),
    )
    .expect("fixture parses");

    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir
        .path()
        .join(spec["filename"].as_str().expect("filename"));
    std::fs::write(&path, spec["input"].as_str().expect("input")).expect("write input");
    let path = path.to_str().expect("utf8 path").to_string();

    let mut args: Vec<&str> = vec![spec["args"][0].as_str().expect("subcommand"), &path];
    args.push(spec["args"][1].as_str().expect("flag"));
    let (code, out, err) = run(&args);
    assert_eq!(code, 0, "{err}");

    let got: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    let expect = &spec["expect"];

    let mut keys: Vec<&str> = got
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(serde_json::json!(keys), expect["envelope_keys"]);
    assert_eq!(got["format_version"], expect["format_version"]);
    assert_eq!(got["command"], expect["command"]);
    assert_eq!(got["input"], serde_json::json!(path));

    let mut result_keys: Vec<&str> = got["result"]
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    result_keys.sort_unstable();
    assert_eq!(serde_json::json!(result_keys), expect["result_keys"]);

    let sections = got["result"]["sections"].as_array().expect("sections");
    assert_eq!(serde_json::json!(sections.len()), expect["result_sections"]);
    let sentences: usize = sections
        .iter()
        .flat_map(|s| s["paragraphs"].as_array().expect("paragraphs"))
        .map(|p| p["sentences"].as_array().expect("sentences").len())
        .sum();
    assert_eq!(serde_json::json!(sentences), expect["result_sentences"]);
}

// ---------------------------------------------------------------------------
// The environment-scoped commands. These spawn the binary; see the module
// header for why.
// ---------------------------------------------------------------------------

#[test]
fn config_show_reports_where_every_value_came_from() {
    let home = tempfile::tempdir().expect("temp dir");
    let out = scoped(home.path())
        .args(["config", "show"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8(out).expect("utf8");

    for key in [
        "data_dir",
        "model_dir",
        "models.udpipe",
        "models.embedding",
        "semantic.threshold",
        "summarize.n",
        "summarize.algorithm",
        "keyphrases.n",
        "keyphrases.algorithm",
    ] {
        assert!(out.contains(key), "missing `{key}` in:\n{out}");
    }
    // Every line carries its origin.
    for line in out.lines() {
        assert!(line.contains(" # "), "no origin on: {line}");
    }
    assert!(out.contains("default"), "{out}");

    // An environment override is reported as one, not as a default.
    let out = scoped(home.path())
        .env("MATRA_DATA_DIR", home.path().join("elsewhere"))
        .args(["config", "show"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8(out).expect("utf8");
    assert!(out.contains("MATRA_DATA_DIR"), "{out}");
}

#[test]
fn config_init_writes_once_and_refuses_without_force() {
    let home = tempfile::tempdir().expect("temp dir");
    let target = home.path().join("config").join("matra").join("config.toml");

    scoped(home.path())
        .args(["config", "init"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("config.toml"));
    let written = std::fs::read_to_string(&target).expect("the file exists");
    assert!(written.contains("[models]"), "{written}");
    assert!(written.contains("threshold"), "{written}");

    scoped(home.path())
        .args(["config", "init"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--force"));

    scoped(home.path())
        .args(["config", "init", "--force"])
        .assert()
        .code(0);
    assert_eq!(
        std::fs::read_to_string(&target).expect("still there"),
        written
    );
}

/// A file written by `config init` is a file `config show` reads back,
/// and every value in it is then attributed to the file rather than to
/// the built-in defaults.
#[test]
fn a_written_config_becomes_the_source() {
    let home = tempfile::tempdir().expect("temp dir");
    scoped(home.path())
        .args(["config", "init"])
        .assert()
        .code(0);

    let out = scoped(home.path())
        .args(["config", "show"])
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let out = String::from_utf8(out).expect("utf8");
    assert!(out.contains("config.toml"), "{out}");
}

#[test]
#[ignore = "requires the UDPipe model"]
fn stdin_is_read_from_dash_and_labelled_by_stdin_filename() {
    let md = format!("# A Heading\n\n{PROSE}\n");

    // The real environment here, not a scrubbed one: this test needs the
    // model cache the other model-gated tests use. It spawns the binary
    // only because `-` reads the process's own stdin.
    let out = Command::cargo_bin("matra")
        .expect("binary built with --features cli")
        .args(["analyze", "-", "--stdin-filename", "notes.md", "--json"])
        .write_stdin(md)
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let parsed: serde_json::Value =
        serde_json::from_slice(&out).expect("stdout is the JSON envelope");
    assert_eq!(parsed["input"], "notes.md");
    // `.md` selected the markdown decomposer, so the heading became a
    // section rather than a sentence.
    assert_eq!(parsed["result"]["sections"][0]["heading"], "A Heading");
}
