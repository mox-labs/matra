//! The executed-incantation test for the agent skill.
//!
//! A skill whose commands do not run is a defect, not a documentation nit
//! (ADR-0012). Every fenced `console` or `bash` block in
//! `skills/matra/SKILL.md` and its references whose first line is a
//! `matra ...` command is extracted here and driven through
//! [`matra::cli::run`] against the fixtures in `tests/fixtures/skill/`.
//! Drift between the skill text and the command line fails CI the way a
//! stale type name fails the docsite.
//!
//! Two annotations may sit directly above a block:
//!
//! ```text
//! <!-- expect: exit 2 -->   the exit code the block should produce (default 0)
//! <!-- needs: model -->     run only in the model-gated lane
//! ```
//!
//! The model-gated blocks run under the existing convention:
//!
//!     cargo test --features cli --test skill -- --ignored
//!
//! A block takes one of two routes. Any block whose first argument is
//! `config` is run as a subprocess with `MATRA_CONFIG_FILE`,
//! `MATRA_DATA_DIR`, `MATRA_MODEL_DIR`, `XDG_CONFIG_HOME` and
//! `XDG_DATA_HOME` removed and `HOME` pointed at an empty temporary
//! directory, because `config show` resolves its answer out of the
//! process environment and would otherwise report the contributor's own
//! config file. Every other block runs in process through
//! [`matra::cli::run`]. Both routes are held to the same exit code and
//! the same `format_version` rule.
//!
//! The block count is asserted against a fence-aware scan of the same
//! files, so an incantation written outside a block the runner reads
//! cannot hide from it. The scan trims leading whitespace, so an
//! indented `matra ...` line counts, and a command-shaped line that no
//! fence encloses fails the law by name rather than by an off-by-one in
//! the totals.

#![cfg(feature = "cli")]

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The repository root, which is also the plugin root: `skills/` sits
/// directly under it, which is where plugin discovery looks.
const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

/// Where the skill files live.
const SKILL_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/skills/matra");

/// The inputs every incantation names. A bare `notes.md` in the skill is
/// rewritten to this directory, so the skill reads as a user would type
/// it while the runner still finds the file. Rewriting rather than
/// changing the process directory keeps the two lanes safe to run in
/// parallel.
const FIXTURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/skill");

/// The floor the plan sets. Fewer than this and the skill is not
/// documenting the command line. Raised from 8 to 12 when `--skill -r`
/// landed and the references section gained its own incantation: a floor
/// that sits well below what the files carry stops catching a section
/// deleted wholesale.
const MINIMUM_INCANTATIONS: usize = 12;

/// The body cap from ADR-0012: SKILL.md is the top level and stays small
/// enough to read in one pass.
const MAX_SKILL_BODY_LINES: usize = 150;

/// The cap on one reference.
const MAX_REFERENCE_LINES: usize = 200;

// ---------------------------------------------------------------------------
// The files
// ---------------------------------------------------------------------------

fn skill_path() -> PathBuf {
    Path::new(SKILL_DIR).join("SKILL.md")
}

/// Every reference, sorted by name, so a failure names the same file on
/// every platform.
fn reference_paths() -> Vec<PathBuf> {
    let dir = Path::new(SKILL_DIR).join("references");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|entry| entry.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no references under {}", dir.display());
    paths
}

fn every_path() -> Vec<PathBuf> {
    let mut paths = vec![skill_path()];
    paths.extend(reference_paths());
    paths
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

// ---------------------------------------------------------------------------
// Frontmatter
// ---------------------------------------------------------------------------

/// The `key: value` pairs of a leading `---` block, plus the body that
/// follows it. A file without frontmatter is a failure, not an empty map:
/// the flag in M3 reads the reference list out of these keys.
fn frontmatter(path: &Path, text: &str) -> (Vec<(String, String)>, String) {
    let mut lines = text.lines();
    assert_eq!(
        lines.next(),
        Some("---"),
        "{} does not open with frontmatter",
        path.display()
    );

    let mut pairs = Vec::new();
    let mut closed = false;
    for line in lines.by_ref() {
        if line == "---" {
            closed = true;
            break;
        }
        let (key, value) = line.split_once(':').unwrap_or_else(|| {
            panic!("{}: frontmatter line without a key: {line}", path.display())
        });
        pairs.push((key.trim().to_string(), value.trim().to_string()));
    }
    assert!(closed, "{}: frontmatter is never closed", path.display());

    let body: String = lines.collect::<Vec<_>>().join("\n");
    (pairs, body)
}

fn value<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

// ---------------------------------------------------------------------------
// Extraction
// ---------------------------------------------------------------------------

/// One command the skill tells a reader to run.
struct Incantation {
    /// Which file it came from, for the failure message.
    source: String,
    /// The line the fence opened on, likewise.
    line: usize,
    /// The command as written, `$ ` already stripped.
    command: String,
    /// The exit code the block annotates.
    expect: u8,
    /// Whether the block is annotated as needing the UDPipe model.
    needs_model: bool,
}

/// True for a line that a scanner should count as a command: the same
/// shape the extractor accepts, judged without knowing about fences.
///
/// Leading whitespace is trimmed first. Four spaces in front of a
/// command is an indented code block in markdown, which renders as code
/// and runs as nothing; before the trim such a line raised neither half
/// of the count law and the two agreed while the command went unrun.
fn looks_like_a_command(line: &str) -> bool {
    let line = line.trim_start();
    let line = line.strip_prefix("$ ").unwrap_or(line);
    line.starts_with("matra ")
}

/// One command-shaped line the scan found, and whether a fence encloses
/// it. The scan is the `grep -c` half of the count law: it knows only
/// that a fence opens and closes, never which fences the extractor
/// accepts.
struct CommandLine {
    source: String,
    /// 1-based, for the failure message.
    line: usize,
    text: String,
    fenced: bool,
}

/// Every command-shaped line in one file, fenced or loose.
fn scan(path: &Path, text: &str) -> Vec<CommandLine> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("<unnamed>")
        .to_string();

    let mut found = Vec::new();
    let mut inside = false;
    for (index, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            inside = !inside;
            continue;
        }
        if looks_like_a_command(line) {
            found.push(CommandLine {
                source: name.clone(),
                line: index + 1,
                text: line.trim().to_string(),
                fenced: inside,
            });
        }
    }
    found
}

/// Every fenced `console` or `bash` block whose first line is a command.
fn extract(path: &Path, text: &str) -> Vec<Incantation> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("<unnamed>")
        .to_string();

    let lines: Vec<&str> = text.lines().collect();
    let mut found = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let Some(info) = lines[i].strip_prefix("```") else {
            i += 1;
            continue;
        };
        let opened_at = i + 1;
        // Collect the block body, whatever its language: a fence must be
        // consumed so a `json` block full of braces is never scanned for
        // annotations or commands.
        let mut body = Vec::new();
        i += 1;
        while i < lines.len() && !lines[i].starts_with("```") {
            body.push(lines[i]);
            i += 1;
        }
        i += 1; // past the closing fence

        if info.trim() != "console" && info.trim() != "bash" {
            continue;
        }
        let Some(first) = body.first() else { continue };
        if !looks_like_a_command(first) {
            continue;
        }

        let (expect, needs_model) = annotations(&lines[..opened_at - 1]);
        found.push(Incantation {
            source: name.clone(),
            line: opened_at,
            command: first.strip_prefix("$ ").unwrap_or(first).to_string(),
            expect,
            needs_model,
        });
    }
    found
}

/// The annotations directly above a fence: the nearest run of HTML
/// comments, ignoring blank lines between them and the fence.
fn annotations(before: &[&str]) -> (u8, bool) {
    let mut expect = 0u8;
    let mut needs_model = false;
    for line in before.iter().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some(inner) = line
            .strip_prefix("<!--")
            .and_then(|rest| rest.strip_suffix("-->"))
        else {
            break;
        };
        let inner = inner.trim();
        if let Some(code) = inner.strip_prefix("expect: exit ") {
            expect = code.trim().parse().expect("an exit code after `expect:`");
        } else if inner == "needs: model" {
            needs_model = true;
        }
    }
    (expect, needs_model)
}

// ---------------------------------------------------------------------------
// Running
// ---------------------------------------------------------------------------

/// The argument vector for one incantation, with fixture names resolved.
///
/// A skill block is plain argv by rule: no pipe, no redirect, no quoting.
/// That rule is what lets a whitespace split be a faithful reading of the
/// line a user would type, and it is asserted rather than assumed.
fn argv(incantation: &Incantation) -> Vec<OsString> {
    let command = &incantation.command;
    for forbidden in ['|', '>', '<', '"', '\'', '$', '`'] {
        assert!(
            !command.contains(forbidden),
            "{}:{} uses a shell construct (`{forbidden}`); skill blocks are plain argv: {command}",
            incantation.source,
            incantation.line
        );
    }
    command
        .split_whitespace()
        .map(|token| {
            if token.ends_with(".md") || token.ends_with(".txt") {
                OsString::from(Path::new(FIXTURE_DIR).join(token))
            } else {
                OsString::from(token)
            }
        })
        .collect()
}

/// The subcommand a block names: the first argument after the program.
fn subcommand(incantation: &Incantation) -> Option<&str> {
    incantation.command.split_whitespace().nth(1)
}

/// The in-process route. The library module is the program, so a failure
/// points at a function rather than at a subprocess.
fn in_process(args: Vec<OsString>) -> (u8, String, String) {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = matra::cli::run(args, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).expect("stdout is utf8"),
        String::from_utf8(err).expect("stderr is utf8"),
    )
}

/// The subprocess route, for the blocks that read the environment.
///
/// `config show` answers out of `MATRA_CONFIG_FILE`, the XDG variables
/// and `HOME`. In process it would read the contributor's own
/// `~/.config/matra/config.toml`, so an unparsable file there would fail
/// this lane for a reason that has nothing to do with the skill. The
/// child gets those variables removed and an empty `HOME`, which is the
/// same scrub `tests/cli.rs` gives its config tests.
fn spawn_scrubbed(args: Vec<OsString>) -> (u8, String, String) {
    let home = tempfile::tempdir().expect("temp dir");
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_matra"));
    command.args(&args[1..]);
    for key in [
        "MATRA_CONFIG_FILE",
        "MATRA_DATA_DIR",
        "MATRA_MODEL_DIR",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
    ] {
        command.env_remove(key);
    }
    command.env("HOME", home.path());

    let output = command.output().expect("spawn the matra binary");
    let code = output
        .status
        .code()
        .expect("the child exited rather than being signalled");
    (
        u8::try_from(code).expect("an exit code in 0..=255"),
        String::from_utf8(output.stdout).expect("stdout is utf8"),
        String::from_utf8(output.stderr).expect("stderr is utf8"),
    )
}

fn run(incantation: &Incantation) {
    let args = argv(incantation);
    let (code, stdout, stderr) = if subcommand(incantation) == Some("config") {
        spawn_scrubbed(args)
    } else {
        in_process(args)
    };

    assert_eq!(
        code, incantation.expect,
        "{}:{} `{}` exited {code}, expected {}\nstdout: {stdout}\nstderr: {stderr}",
        incantation.source, incantation.line, incantation.command, incantation.expect
    );

    // A `--json` block that succeeded promises the envelope, on either
    // route. `completions` is the exception: it prints a shell script and
    // ignores the flag. Exit 2 writes nothing to stdout by design, so
    // there is no object to parse.
    if incantation.command.contains("--json")
        && code != 2
        && subcommand(incantation) != Some("completions")
    {
        let value: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
            panic!(
                "{}:{} `{}` did not emit JSON: {e}\nstdout: {stdout}",
                incantation.source, incantation.line, incantation.command
            )
        });
        assert_eq!(
            value.get("format_version").and_then(|v| v.as_u64()),
            Some(1),
            "{}:{} `{}` emitted no format_version",
            incantation.source,
            incantation.line,
            incantation.command
        );
    }
}

fn all_incantations() -> Vec<Incantation> {
    every_path()
        .iter()
        .flat_map(|path| extract(path, &read(path)))
        .collect()
}

// ---------------------------------------------------------------------------
// The tests
// ---------------------------------------------------------------------------

/// The runner sees every command the files contain.
///
/// Two things are held. First, no command-shaped line sits outside a
/// fence: prose that names a command, and the four-space indented block
/// that markdown renders as code, are both rejected by name and told to
/// use a fence. Second, the number of command-shaped lines equals the
/// number the extractor reads, so a command inside a fence the extractor
/// skips (a `text` block, or a second line in a `console` one) raises the
/// scan count and not the block count. The scan knows only where fences
/// open and close; the two halves agreeing is what makes "every
/// incantation runs" a statement about the files rather than about the
/// extractor.
#[test]
fn every_command_in_the_files_is_inside_a_block_the_runner_reads() {
    let scanned: Vec<CommandLine> = every_path()
        .iter()
        .flat_map(|path| scan(path, &read(path)))
        .collect();

    let loose: Vec<String> = scanned
        .iter()
        .filter(|line| !line.fenced)
        .map(|line| format!("{}:{} {}", line.source, line.line, line.text))
        .collect();
    assert!(
        loose.is_empty(),
        "command lines that no fence encloses, so the runner never executes them; \
         put each in a ```console block:\n  {}",
        loose.join("\n  ")
    );

    let extracted = all_incantations().len();
    assert_eq!(
        extracted,
        scanned.len(),
        "{} command lines in the skill files, {extracted} inside blocks the runner reads",
        scanned.len()
    );
    assert!(
        extracted >= MINIMUM_INCANTATIONS,
        "the skill documents {extracted} commands, fewer than the {MINIMUM_INCANTATIONS} floor"
    );
}

/// Regression: an indented command used to escape both halves of the law.
///
/// `looks_like_a_command` did not trim, so `    matra --version` inside a
/// four-space block raised neither the scan count nor the block count.
/// The two agreed, the law passed, and the command the reader is told to
/// type was never run. The scan trims now, and an unfenced line is named.
#[test]
fn an_indented_command_is_caught_rather_than_counted_as_absent() {
    let planted = [
        "---",
        "name: planted",
        "summary: a fixture for the count law.",
        "---",
        "",
        "Four spaces, which markdown renders as code and the runner cannot read:",
        "",
        "    matra --version",
        "",
        "```console",
        "$ matra completions zsh",
        "```",
    ]
    .join("\n");
    let path = Path::new("planted.md");

    let scanned = scan(path, &planted);
    assert_eq!(scanned.len(), 2, "the trim makes the indented line count");
    let loose: Vec<&CommandLine> = scanned.iter().filter(|line| !line.fenced).collect();
    assert_eq!(loose.len(), 1, "the indented line is outside every fence");
    assert_eq!(loose[0].line, 8);
    assert_eq!(loose[0].text, "matra --version");

    assert_eq!(
        extract(path, &planted).len(),
        1,
        "the extractor reads the fenced block and nothing else"
    );
}

/// Every incantation that needs no model, executed.
#[test]
fn every_model_free_incantation_runs() {
    let incantations = all_incantations();
    let ran = incantations.iter().filter(|i| !i.needs_model).count();
    assert!(ran > 0, "no model-free incantation to run");
    for incantation in incantations.iter().filter(|i| !i.needs_model) {
        run(incantation);
    }
}

/// Every incantation, including the ones that parse.
#[test]
#[ignore = "requires the UDPipe model"]
fn every_incantation_runs() {
    for incantation in &all_incantations() {
        run(incantation);
    }
}

/// The frontmatter version is the crate version. A skill printed by the
/// binary that names a different version is worse than one that names
/// none, because it looks authoritative.
#[test]
fn the_skill_version_is_the_crate_version() {
    let path = skill_path();
    let (pairs, _) = frontmatter(&path, &read(&path));
    assert_eq!(value(&pairs, "name"), Some("matra"));
    assert_eq!(
        value(&pairs, "version"),
        Some(env!("CARGO_PKG_VERSION")),
        "SKILL.md frontmatter version must equal the crate version"
    );
    let description = value(&pairs, "description").expect("a description");
    assert!(
        description.len() < 200,
        "the description is {} characters, over the 200 cap",
        description.len()
    );
}

/// The plugin manifest names the same crate. `skills/matra/` is both the
/// text the binary prints and the skill the repository distributes as a
/// plugin, so `.claude-plugin/plugin.json` and the frontmatter beside it
/// must agree on the name, the version and the description. A manifest
/// that claims 0.1.0 while the crate has moved on hands an installer a
/// version it cannot check.
#[test]
fn the_plugin_manifest_matches_the_crate_and_the_skill() {
    let path = Path::new(MANIFEST_DIR).join(".claude-plugin/plugin.json");
    let manifest: serde_json::Value = serde_json::from_str(&read(&path))
        .unwrap_or_else(|e| panic!("{} is not valid JSON: {e}", path.display()));

    let skill = skill_path();
    let (pairs, _) = frontmatter(&skill, &read(&skill));

    for (key, expected) in [
        ("version", env!("CARGO_PKG_VERSION")),
        ("name", value(&pairs, "name").expect("a skill name")),
        (
            "description",
            value(&pairs, "description").expect("a skill description"),
        ),
    ] {
        assert_eq!(
            manifest.get(key).and_then(|v| v.as_str()),
            Some(expected),
            "plugin.json `{key}` must equal `{expected}`"
        );
    }
}

/// Every reference declares the two keys the flag reads in M3, and its
/// name is its filename, so `--skill -r <name>` can find the file.
#[test]
fn every_reference_declares_a_name_and_a_summary() {
    for path in reference_paths() {
        let (pairs, _) = frontmatter(&path, &read(&path));
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("utf8 stem");
        assert_eq!(
            value(&pairs, "name"),
            Some(stem),
            "{}: the frontmatter name must be the file name",
            path.display()
        );
        let summary = value(&pairs, "summary")
            .unwrap_or_else(|| panic!("{}: no summary in frontmatter", path.display()));
        assert!(
            !summary.is_empty(),
            "{}: the summary is empty",
            path.display()
        );
    }
}

/// The size caps ADR-0012 sets. They are what keeps the top level worth
/// reading in full and each reference worth loading on demand.
#[test]
fn the_skill_and_its_references_stay_within_their_caps() {
    let path = skill_path();
    let (_, body) = frontmatter(&path, &read(&path));
    let body_lines = body.lines().count();
    assert!(
        body_lines < MAX_SKILL_BODY_LINES,
        "the SKILL.md body is {body_lines} lines, over the {MAX_SKILL_BODY_LINES} cap"
    );

    for reference in reference_paths() {
        let lines = read(&reference).lines().count();
        assert!(
            lines < MAX_REFERENCE_LINES,
            "{} is {lines} lines, over the {MAX_REFERENCE_LINES} cap",
            reference.display()
        );
    }
}
