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

mod document_schema;

use std::ffi::OsString;
use std::io::Write;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;

const PROSE: &str = "The committee approved the proposal. Three amendments were submitted \
                     by the working group. The chair adjourned the meeting.";

/// The commands that read a document, as this build can run them. Without
/// `model2vec`, `clusters` refuses before it looks at its path, because
/// the build ships no embedder; that refusal has its own test.
#[cfg(feature = "model2vec")]
const DOCUMENT_COMMANDS: &[&str] = &["analyze", "summarize", "keyphrases", "clusters"];
#[cfg(not(feature = "model2vec"))]
const DOCUMENT_COMMANDS: &[&str] = &["analyze", "summarize", "keyphrases"];

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
        "clusters",
        "config",
        "completions",
    ] {
        assert!(out.contains(command), "missing `{command}` in:\n{out}");
    }
}

/// The agent flags are on the help, because a flag nobody can find is a
/// flag nobody runs.
#[test]
fn help_lists_the_agent_flags() {
    let (code, out, _) = run(&["--help"]);
    assert_eq!(code, 0);
    assert!(out.contains("--skill"), "{out}");
    assert!(out.contains("--reference"), "{out}");
    assert!(out.contains("--schema"), "{out}");
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
/// only to report that the file does not exist is the wrong order. The
/// failure is the one the library gives for the path, and every command
/// that reads a document gives it.
#[test]
fn missing_input_file_exits_two_without_loading_the_model() {
    let missing = "/nonexistent/path/to/file.md";
    for command in DOCUMENT_COMMANDS {
        let (code, out, err) = run(&[command, missing]);
        assert_eq!(code, 2, "{command}");
        assert!(out.is_empty(), "{command} printed: {out}");
        assert!(err.contains("cannot read"), "{command}: {err}");
        assert!(err.contains(missing), "{command} names the path: {err}");
    }
}

/// `summarize`, `keyphrases` and `clusters` report on one document, so a
/// directory is refused, before the engine is built, by name, and with
/// the command that does read one. A directory used to be ingested and
/// silently reduced to whatever file the walk yielded first.
#[test]
fn a_directory_is_refused_by_every_command_that_reads_one_document() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::write(dir.path().join("a.txt"), PROSE).expect("write");
    std::fs::write(dir.path().join("b.txt"), PROSE).expect("write");
    let path = dir.path().to_str().expect("utf8 path");

    for command in DOCUMENT_COMMANDS.iter().filter(|c| **c != "analyze") {
        let (code, out, err) = run(&[command, path]);
        assert_eq!(code, 2, "{command}: {err}");
        assert!(out.is_empty(), "{command} printed: {out}");
        assert!(err.contains(path), "{command} names the path: {err}");
        assert!(err.contains("is a directory"), "{command}: {err}");
        assert!(err.contains("matra analyze"), "{command}: {err}");
    }
}

/// `--format` names the format of one file or of stdin. A directory's
/// files are each read as their own name says, so the pair is refused,
/// before the engine is built.
#[test]
fn format_with_a_directory_is_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().to_str().expect("utf8 path");
    let (code, out, err) = run(&["analyze", path, "--format", "markdown"]);
    assert_eq!(code, 2, "{err}");
    assert!(out.is_empty(), "{out}");
    assert!(err.contains("--format"), "{err}");
}

/// `--depth` shapes the JSON, so asking for it without `--json` is a
/// usage failure rather than a flag silently ignored.
#[test]
fn depth_needs_json() {
    let (code, out, err) = run(&["analyze", "notes.md", "--depth", "sentence"]);
    assert_eq!(code, 2);
    assert!(out.is_empty(), "{out}");
    assert!(err.contains("--json"), "{err}");
}

/// The format vocabulary is the one the JSON and Python use, `plain_text`
/// included, and nothing else.
#[test]
fn format_takes_the_shared_vocabulary() {
    let (code, _, err) = run(&["analyze", "x", "--format", "plain-text"]);
    assert_eq!(code, 2, "the kebab spelling is not the vocabulary");
    assert!(
        err.contains("plain_text"),
        "the refusal names the word: {err}"
    );
    for format in ["markdown", "plain_text", "blocks"] {
        let (code, _, err) = run(&["analyze", "/nonexistent/x", "--format", format]);
        assert_eq!(code, 2, "{format}");
        assert!(
            err.contains("cannot read"),
            "{format} parsed, then the path failed: {err}"
        );
    }
}

// ---------------------------------------------------------------------------
// Failures under --json. None of these needs a model: each fails before
// the engine is built.
// ---------------------------------------------------------------------------

/// The envelope of a failure: five keys, `result` null, and `error` the
/// serde form every surface carries.
fn failure_envelope(out: &str) -> serde_json::Value {
    let parsed: serde_json::Value = serde_json::from_str(out).expect("stdout is one JSON object");
    let mut keys: Vec<&str> = parsed
        .as_object()
        .expect("an object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["command", "error", "format_version", "input", "result"]
    );
    assert_eq!(parsed["format_version"], 2);
    assert!(parsed["result"].is_null(), "a failure has no result");
    document_schema::assert_valid("error", &parsed["error"], "the failure envelope");
    parsed
}

/// Defect 5 of EPR-0006: a failure under `--json` wrote nothing to
/// stdout, so an agent read prose. It writes the envelope, with the error
/// in it, and still writes the message to stderr and exits 2.
#[test]
fn a_failure_under_json_is_an_envelope_on_stdout() {
    let missing = "/nonexistent/path/to/file.md";
    let (code, out, err) = run(&["analyze", missing, "--json"]);
    assert_eq!(code, 2);
    assert!(err.starts_with("matra: "), "{err}");
    let envelope = failure_envelope(&out);
    assert_eq!(envelope["command"], "analyze");
    assert_eq!(envelope["input"], missing);
    let error = &envelope["error"];
    assert_eq!(error["kind"], "io");
    assert_eq!(error["details"]["io_kind"], "not_found");
    assert_eq!(error["details"]["path"], missing);
    assert!(
        err.contains(error["message"].as_str().expect("a message")),
        "stderr carries the same message: {err}"
    );
}

/// The command line's own refusals carry the kind whose remedy matches:
/// each of these is a call to change, which is `invalid_input`.
#[test]
fn the_command_line_s_own_refusals_are_invalid_input() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().to_str().expect("utf8 path");
    for (args, command) in [
        (vec!["summarize", path, "--json"], "summarize"),
        (
            vec!["analyze", path, "--format", "blocks", "--json"],
            "analyze",
        ),
        (vec!["-r", "json", "--json"], "skill"),
        (vec!["--skill", "-r", "jsn", "--json"], "skill"),
        (vec!["--schema", "documnet", "--json"], "schema"),
    ] {
        let (code, out, err) = run(&args);
        assert_eq!(code, 2, "{args:?}: {err}");
        let envelope = failure_envelope(&out);
        assert_eq!(envelope["command"], command, "{args:?}");
        assert_eq!(envelope["error"]["kind"], "invalid_input", "{args:?}");
    }
}

/// A usage failure under `--json` is a failure like any other: clap's
/// message on stderr, and the envelope on stdout naming the command its
/// arguments asked for, or null when they named none.
#[test]
fn a_usage_failure_under_json_is_an_envelope() {
    let (code, out, err) = run(&["analyze", "--json"]);
    assert_eq!(code, 2);
    assert!(err.contains("Usage:"), "clap's own text on stderr: {err}");
    let envelope = failure_envelope(&out);
    assert_eq!(envelope["command"], "analyze");
    assert_eq!(envelope["error"]["kind"], "invalid_input");
    assert!(
        envelope["error"]["message"]
            .as_str()
            .is_some_and(|m| m.contains("<PATH>")),
        "{envelope}"
    );

    let (code, out, _) = run(&["--json", "--bogus"]);
    assert_eq!(code, 2);
    let envelope = failure_envelope(&out);
    assert!(envelope["command"].is_null(), "{envelope}");

    // Without `--json` a usage failure writes nothing to stdout, as it
    // always has.
    let (code, out, _) = run(&["--bogus"]);
    assert_eq!(code, 2);
    assert!(out.is_empty(), "{out}");
}

// ---------------------------------------------------------------------------
// The schemas. `--schema` reads no document, so none of this needs a model.
// ---------------------------------------------------------------------------

fn schema_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("spec/schema/v2")
}

/// Every published schema prints byte for byte, by the name its file
/// carries.
#[test]
fn every_schema_prints_its_file_verbatim() {
    for name in [
        "blocks",
        "clusters",
        "document",
        "error",
        "keyphrases",
        "summary",
    ] {
        let on_disk = std::fs::read_to_string(schema_dir().join(format!("{name}.schema.json")))
            .expect("read the schema");
        let (code, out, err) = run(&["--schema", name]);
        assert_eq!(code, 0, "{name}: {err}");
        assert_eq!(out, on_disk, "{name}");
    }
}

/// `--schema` alone lists every schema with its title; under `--json` the
/// list and each schema come in the envelope.
#[test]
fn the_schema_list_and_the_schema_json() {
    let (code, out, err) = run(&["--schema"]);
    assert_eq!(code, 0, "{err}");
    let names: Vec<&str> = out
        .lines()
        .map(|line| line.split_whitespace().next().expect("a name"))
        .collect();
    assert_eq!(
        names,
        [
            "blocks",
            "clusters",
            "document",
            "error",
            "keyphrases",
            "summary"
        ]
    );

    let (code, out, _) = run(&["--schema", "--json"]);
    assert_eq!(code, 0);
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("JSON");
    assert_eq!(parsed["command"], "schema");
    assert!(parsed["error"].is_null());
    assert_eq!(
        parsed["result"]["schemas"].as_array().map(Vec::len),
        Some(6)
    );

    let (code, out, _) = run(&["--schema", "document", "--json"]);
    assert_eq!(code, 0);
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("JSON");
    assert_eq!(parsed["result"]["name"], "document");
    assert_eq!(
        parsed["result"]["schema"]["title"],
        "matra Document, schema 2.x"
    );
}

/// `--schema` is a property of the program, like `--skill`, so it
/// outranks a subcommand, and the two cannot be asked for at once.
#[test]
fn the_schema_outranks_a_subcommand_and_conflicts_with_the_skill() {
    let (code, out, err) = run(&["analyze", "/nonexistent/x.md", "--schema", "error"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("matra error"), "{out}");

    let (code, _, _) = run(&["--schema", "--skill"]);
    assert_eq!(code, 2);
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
// The agent skill. `--skill` reads no document, so none of this needs a
// model.
// ---------------------------------------------------------------------------

/// Where the skill files live, read at test time so the assertions are
/// against the files rather than against a second copy of them.
fn skill_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("skills/matra")
}

/// Every reference name, alphabetical, which is the order `--skill -r`
/// promises.
fn reference_names() -> Vec<String> {
    let dir = skill_dir().join("references");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|entry| entry.expect("dir entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "md"))
        .map(|path| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .expect("utf8 name")
                .to_string()
        })
        .collect();
    names.sort();
    names
}

/// The verbatim promise, from the Rust launcher. `python/tests/test_cli.py`
/// asserts the same file through `cli_main`, and the two together are what
/// make "the binary prints its own skill" a statement about both launchers.
#[test]
fn the_skill_is_the_file_verbatim() {
    let on_disk = std::fs::read_to_string(skill_dir().join("SKILL.md")).expect("read SKILL.md");
    let (code, out, err) = run(&["--skill"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, on_disk);
}

#[test]
fn every_reference_prints_its_file_verbatim() {
    for name in reference_names() {
        let path = skill_dir().join("references").join(format!("{name}.md"));
        let on_disk = std::fs::read_to_string(&path).expect("read the reference");
        let (code, out, err) = run(&["--skill", "-r", &name]);
        assert_eq!(code, 0, "{name}: {err}");
        assert_eq!(out, on_disk, "{name} did not print its own file");
    }
}

/// The list is every file under `references/`, in file-name order, with
/// the summary each file declares. A reference added to the directory and
/// not to the binary fails here as well as in the unit test that reads the
/// same directory.
#[test]
fn the_reference_list_names_every_file_in_order() {
    let (code, out, err) = run(&["--skill", "-r"]);
    assert_eq!(code, 0, "{err}");

    let listed: Vec<&str> = out
        .lines()
        .map(|line| line.split_whitespace().next().expect("a name"))
        .collect();
    assert_eq!(listed, reference_names());

    for line in out.lines() {
        let (_, summary) = line.split_once("  ").expect("a name then its summary");
        assert!(!summary.trim().is_empty(), "no summary on: {line}");
    }
}

#[test]
fn an_unknown_reference_exits_two_and_names_the_known_ones() {
    let (code, out, err) = run(&["--skill", "-r", "jsn"]);
    assert_eq!(code, 2);
    assert!(out.is_empty(), "{out}");
    for name in reference_names() {
        assert!(err.contains(&name), "{err} omits `{name}`");
    }
}

/// `-r` alone is half an incantation, and the message says which half is
/// missing rather than reporting the absent subcommand.
#[test]
fn a_reference_without_the_skill_flag_is_refused_by_name() {
    for args in [vec!["-r"], vec!["-r", "json"], vec!["--reference", "json"]] {
        let (code, out, err) = run(&args);
        assert_eq!(code, 2, "{args:?}");
        assert!(out.is_empty(), "{out}");
        assert!(err.contains("--skill"), "{err}");
    }
}

/// `--skill` is a property of the program, not an action on a document,
/// so it outranks a subcommand. The path here does not exist and is never
/// looked at.
#[test]
fn the_skill_outranks_a_subcommand() {
    let on_disk = std::fs::read_to_string(skill_dir().join("SKILL.md")).expect("read SKILL.md");
    let (code, out, err) = run(&["analyze", "/nonexistent/path/to/file.md", "--skill"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(out, on_disk);
}

/// The three `--json` shapes, against the one envelope every command
/// emits. `input` is null because the command reads no document, and the
/// key is present rather than absent so a consumer reads five keys
/// whatever it ran.
#[test]
fn the_skill_json_is_the_envelope() {
    for (args, expect_list) in [
        (vec!["--skill", "--json"], false),
        (vec!["--skill", "-r", "--json"], true),
        (vec!["--skill", "-r", "json", "--json"], false),
    ] {
        let (code, out, err) = run(&args);
        assert_eq!(code, 0, "{args:?}: {err}");

        let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        let mut keys: Vec<&str> = parsed
            .as_object()
            .expect("an object")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["command", "error", "format_version", "input", "result"]
        );
        assert_eq!(parsed["format_version"], 2);
        assert_eq!(parsed["command"], "skill");
        assert!(parsed["input"].is_null(), "{args:?}: input is not null");
        assert!(parsed["error"].is_null(), "{args:?}: error is not null");

        let result = &parsed["result"];
        if expect_list {
            let references = result["references"].as_array().expect("a list");
            let listed: Vec<&str> = references
                .iter()
                .map(|entry| entry["name"].as_str().expect("a name"))
                .collect();
            assert_eq!(listed, reference_names());
            for entry in references {
                assert!(
                    !entry["summary"].as_str().expect("a summary").is_empty(),
                    "an empty summary in the list"
                );
            }
        } else {
            assert!(result["name"].is_string(), "{args:?}");
            assert!(result["body"].is_string(), "{args:?}");
        }
    }
    // The name reported for a reference is the reference, not the top
    // level.
    let (_, out, _) = run(&["--skill", "-r", "json", "--json"]);
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    assert_eq!(parsed["result"]["name"], "json");
}

/// The subcommand became optional so `--skill` could stand alone. A bare
/// invocation must still be the usage failure it has always been.
#[test]
fn a_bare_invocation_is_still_a_usage_failure() {
    let (code, out, err) = run(&[]);
    assert_eq!(code, 2);
    assert!(out.is_empty(), "usage errors do not go to stdout: {out}");
    assert!(err.contains("Usage: matra"), "{err}");
    assert!(err.contains("analyze"), "{err}");
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

/// Every command that reads a document names it in `input`, a string, and
/// carries `error: null` beside its result. `config show` is held to the
/// same five keys in `config_show_names_the_file_it_read_or_null`, in a
/// scrubbed environment, because what its `input` is depends on whether a
/// config file exists (#166).
#[test]
#[ignore = "requires the UDPipe model"]
fn every_command_emits_the_same_envelope() {
    let f = fixture(PROSE, ".txt");
    let path = path_of(&f).to_string();

    for (args, command) in [
        (vec!["analyze", &path, "--json"], "analyze"),
        (vec!["summarize", &path, "--json"], "summarize"),
        (vec!["keyphrases", &path, "--json"], "keyphrases"),
    ] {
        let (code, out, err) = run(&args);
        assert_eq!(code, 0, "{command}: {err}");
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        let object = parsed.as_object().expect("an object");
        let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["command", "error", "format_version", "input", "result"]
        );
        assert_eq!(parsed["format_version"], 2);
        assert_eq!(parsed["command"], command);
        assert_eq!(parsed["input"], path.as_str());
        assert!(parsed["error"].is_null(), "{command}");
        assert!(parsed["result"].is_object(), "{command}");
    }
}

/// `matra analyze DIR`: one result per file, in name order. Under
/// `--json` that is one envelope per line; a file that fails is a line
/// with its `error`, the walk goes on past it, and the exit code is 2
/// because one failed.
#[test]
#[ignore = "requires the UDPipe model"]
fn a_directory_is_one_envelope_per_line_and_a_failure_does_not_stop_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::write(dir.path().join("a.md"), format!("# Notes\n\n{PROSE}\n")).expect("write");
    std::fs::write(dir.path().join("b.bin"), [0xff, 0xfe, 0x00]).expect("write");
    std::fs::write(dir.path().join("c.txt"), PROSE).expect("write");
    let path = dir.path().to_str().expect("utf8 path");

    let (code, out, err) = run(&["analyze", path, "--json"]);
    assert_eq!(code, 2, "one file failed: {err}");
    let lines: Vec<serde_json::Value> = out
        .lines()
        .map(|line| serde_json::from_str(line).expect("each line is one envelope"))
        .collect();
    assert_eq!(lines.len(), 3, "{out}");
    let inputs: Vec<&str> = lines
        .iter()
        .map(|l| l["input"].as_str().expect("each names its file"))
        .collect();
    for (input, name) in inputs.iter().zip(["a.md", "b.bin", "c.txt"]) {
        assert!(input.ends_with(name), "{inputs:?}");
    }
    assert!(lines[0]["error"].is_null());
    document_schema::assert_valid("document", &lines[0]["result"], "a.md");
    assert!(lines[1]["result"].is_null());
    assert_eq!(lines[1]["error"]["kind"], "unsupported_format");
    assert!(
        lines[2]["error"].is_null(),
        "the walk went on past the failure"
    );
    assert!(err.contains("b.bin"), "stderr names the failed file: {err}");

    // The table, one block per file, and the same exit code.
    let (code, out, _) = run(&["analyze", path]);
    assert_eq!(code, 2);
    assert!(out.contains("a.md") && out.contains("c.txt"), "{out}");

    // Every file read, nothing found: exit 1. Nothing failed: never 2.
    let empty = tempfile::tempdir().expect("temp dir");
    std::fs::write(empty.path().join("a.txt"), "").expect("write");
    let (code, _, err) = run(&["analyze", empty.path().to_str().expect("utf8"), "--json"]);
    assert_eq!(code, 1, "{err}");
}

/// `--depth` removes what lies below the level asked for, as keys, and
/// what remains is valid against the published document schema.
#[test]
#[ignore = "requires the UDPipe model"]
fn depth_cuts_the_document_at_the_level_asked_for() {
    let f = fixture(&format!("# Notes\n\n{PROSE}\n"), ".md");
    let result = |depth: &str| -> serde_json::Value {
        let (code, out, err) = run(&["analyze", path_of(&f), "--json", "--depth", depth]);
        assert_eq!(code, 0, "{depth}: {err}");
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("JSON");
        document_schema::assert_valid_at_depth(&parsed["result"], depth);
        parsed["result"].clone()
    };

    let document = result("document");
    assert!(document.get("sections").is_none());
    assert_eq!(document["metrics"]["total_sentences"], 3);

    let paragraph = result("paragraph");
    let para = &paragraph["sections"][0]["paragraphs"][0];
    assert!(para["text"].is_string());
    assert!(para.get("sentences").is_none());

    let sentence = result("sentence");
    let first = &sentence["sections"][0]["paragraphs"][0]["sentences"][0];
    assert!(first["text"].is_string());
    assert!(first.get("tokens").is_none());

    let token = result("token");
    let (_, whole, _) = run(&["analyze", path_of(&f), "--json"]);
    let whole: serde_json::Value = serde_json::from_str(&whole).expect("JSON");
    assert_eq!(
        token, whole["result"],
        "token is the default, and everything"
    );
}

/// `--format blocks` reads a JSON file of blocks, which no extension
/// implies: `Input::file_as` on the command line.
#[test]
#[ignore = "requires the UDPipe model"]
fn format_reads_a_file_as_the_format_named() {
    let f = fixture(
        r#"[{"kind": "heading", "level": 1, "text": "Field notes"},
            {"kind": "paragraph", "text": "The river rose overnight."}]"#,
        ".json",
    );
    let (code, out, err) = run(&["analyze", path_of(&f), "--format", "blocks", "--json"]);
    assert_eq!(code, 0, "{err}");
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("JSON");
    assert_eq!(parsed["result"]["provenance"]["format"], "blocks");
    assert_eq!(parsed["result"]["sections"][0]["heading"], "Field notes");

    // Without the flag the same file is plain text.
    let (_, out, _) = run(&["analyze", path_of(&f), "--json"]);
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("JSON");
    assert_eq!(parsed["result"]["provenance"]["format"], "plain_text");
}

/// `matra clusters`, with the embedder the configuration names.
#[cfg(feature = "model2vec")]
#[test]
#[ignore = "requires the UDPipe and potion-base-8M models"]
fn clusters_groups_sentences_by_meaning() {
    let f = fixture(
        "The committee approved the proposal without debate. \
         The weather turned cold by evening. \
         The proposal was approved by the committee with no discussion.",
        ".txt",
    );
    let (code, out, err) = run(&["clusters", path_of(&f), "--json"]);
    assert_eq!(code, 0, "{err}");
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("JSON");
    assert_eq!(parsed["command"], "clusters");
    document_schema::assert_valid("clusters", &parsed["result"], "matra clusters");
    assert_eq!(
        parsed["result"]["clusters"][0]["members"],
        serde_json::json!([0, 2])
    );
    assert_eq!(
        parsed["result"]["provenance"]["parameters"]["threshold"],
        0.85
    );

    // A threshold no pair clears finds nothing, which is exit 1.
    let (code, _, err) = run(&["clusters", path_of(&f), "--threshold", "0.999"]);
    assert_eq!(code, 1, "{err}");
}

/// Without `model2vec` the command exists, and says what it needs before
/// it loads anything.
#[cfg(not(feature = "model2vec"))]
#[test]
fn clusters_without_an_embedder_says_which_feature_ships_one() {
    let f = fixture(PROSE, ".txt");
    let (code, out, err) = run(&["clusters", path_of(&f), "--json"]);
    assert_eq!(code, 2);
    assert!(err.contains("model2vec"), "{err}");
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("JSON");
    assert_eq!(parsed["error"]["kind"], "invalid_input");
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
    document_schema::assert_valid("document", result, "analyze --json");
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

    assert_eq!(got["result"]["provenance"]["schema"], expect["schema"]);
    assert_eq!(got["result"]["provenance"]["format"], expect["format"]);
    assert_eq!(
        got["result"]["metrics"]["total_sentences"],
        expect["result_sentences"]
    );
    document_schema::assert_valid("document", &got["result"], "the envelope fixture");

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
        "models.embedding",
        "summary.n",
        "summary.method",
        "keyphrases.n",
        "keyphrases.method",
        "clusters.threshold",
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

/// #166: `config show --json` names in `input` the config file it read,
/// and writes `input: null` when it read none, as `--skill` does for a
/// command that reads no document. Naming a path that holds no file would
/// assert a read that never happened; the per-key origins already say
/// that every value is a default. The envelope's other four keys are the
/// ones every command writes.
#[test]
fn config_show_names_the_file_it_read_or_null() {
    let home = tempfile::tempdir().expect("temp dir");
    let envelope = |home: &Path| -> serde_json::Value {
        let out = scoped(home)
            .args(["config", "show", "--json"])
            .assert()
            .code(0)
            .get_output()
            .stdout
            .clone();
        serde_json::from_slice(&out).expect("one JSON object")
    };

    let none = envelope(home.path());
    let mut keys: Vec<&str> = none
        .as_object()
        .expect("an object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["command", "error", "format_version", "input", "result"]
    );
    assert_eq!(none["command"], "config");
    assert!(none["input"].is_null(), "no config file was read: {none}");
    assert!(none["error"].is_null());

    let target = home.path().join("config").join("matra").join("config.toml");
    std::fs::create_dir_all(target.parent().expect("a parent")).expect("config dir");
    std::fs::write(&target, "[summary]\nn = 5\n").expect("write the file");
    let read = envelope(home.path());
    assert_eq!(read["input"], target.display().to_string());
    assert_eq!(read["result"]["summary.n"]["value"], 5);
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
    for section in ["[models]", "[summary]", "[keyphrases]", "[clusters]"] {
        assert!(written.contains(section), "{section} missing:\n{written}");
    }
    // 0.3.0's names only: none the file used to carry.
    for old in ["udpipe", "[semantic]", "[summarize]", "algorithm"] {
        assert!(!written.contains(old), "{old} written:\n{written}");
    }

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

/// The file `matra config init` wrote in 0.2, byte for byte.
const CONFIG_FROM_0_2: &str = r#"[models]
# Model directory defaults to $XDG_DATA_HOME/matra/models (~/.local/share/matra/models).
# Override with MATRA_MODEL_DIR, or MATRA_DATA_DIR for the whole data root.
udpipe = "english-ewt-ud-2.5-191206"
embedding = "potion-base-8M"

[semantic]
threshold = 0.85

[summarize]
n = 3
algorithm = "tfidf"

[keyphrases]
n = 10
algorithm = "rake"
"#;

/// 0.3.0 keeps no old name (EPR-0006, decision 8), so a 0.2 config file
/// fails every command that reads it. The failure names the file and
/// what replaced each old key, on stderr and in the `--json` error, so it
/// can be fixed from the message alone.
#[test]
fn a_config_file_from_0_2_fails_naming_the_new_keys() {
    let home = tempfile::tempdir().expect("temp dir");
    let target = home.path().join("config").join("matra").join("config.toml");
    std::fs::create_dir_all(target.parent().expect("a parent")).expect("config dir");
    std::fs::write(&target, CONFIG_FROM_0_2).expect("write the old file");

    let output = scoped(home.path())
        .args(["config", "show"])
        .assert()
        .code(2)
        .get_output()
        .clone();
    let err = String::from_utf8(output.stderr).expect("utf8");
    assert!(output.stdout.is_empty(), "nothing on stdout without --json");
    for named in [
        target.display().to_string().as_str(),
        "`[summary]`",
        "`summary.method`",
        "`keyphrases.method`",
        "`clusters.threshold`",
        "`models.udpipe`",
    ] {
        assert!(err.contains(named), "{named} not named:\n{err}");
    }

    let output = scoped(home.path())
        .args(["config", "show", "--json"])
        .assert()
        .code(2)
        .get_output()
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one JSON object");
    assert!(json["result"].is_null());
    assert_eq!(json["error"]["kind"], "invalid_input");
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

/// Stdin is read as bytes and its format detected, as a file's is. Bytes
/// that are not text are the library's `unsupported_format`, under
/// `--json` as an envelope, where the read used to fail as a UTF-8 error
/// with no kind. `--format` names the format for stdin as for a file.
#[test]
#[ignore = "requires the UDPipe model"]
fn stdin_is_bytes_detected_like_a_file() {
    let output = Command::cargo_bin("matra")
        .expect("binary built with --features cli")
        .args(["analyze", "-", "--json"])
        .write_stdin(vec![0xff, 0xfe, 0x00])
        .assert()
        .code(2)
        .get_output()
        .clone();
    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is the JSON envelope");
    assert_eq!(parsed["input"], "<stdin>");
    assert_eq!(parsed["error"]["kind"], "unsupported_format");

    let out = Command::cargo_bin("matra")
        .expect("binary built with --features cli")
        .args(["analyze", "-", "--format", "markdown", "--json"])
        .write_stdin(format!("# A Heading\n\n{PROSE}\n"))
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let parsed: serde_json::Value = serde_json::from_slice(&out).expect("JSON");
    assert_eq!(parsed["result"]["provenance"]["format"], "markdown");
    assert_eq!(parsed["result"]["sections"][0]["heading"], "A Heading");
}
