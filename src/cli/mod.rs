//! The matra command line, as a library module.
//!
//! Application tier, compiled into the library so both launchers run the
//! same program. The library returns typed errors and structured data;
//! this module decides how to render them, which exit code to use, and
//! what to do when a single file in a batch fails.
//!
//! [`run`] is the whole surface. It never calls `std::process::exit` and
//! never touches the process's own stdout or stderr handles: the caller
//! passes both, so `src/bin/matra.rs` can hand it a locked stdout while
//! the Python launcher hands it a buffer.
//!
//! Exit codes follow the ripgrep convention:
//!   0  success, and (where applicable) something was found
//!   1  success, but nothing was found
//!   2  an error occurred
//!
//! A broken pipe is exit 0: the reader went away, which is what
//! `matra analyze x | head` does on purpose.

mod config_cmd;
mod render;

use std::ffi::OsString;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};

use crate::config::Config;
use crate::domain::{self, Document, Format, MAX_INPUT_BYTES, RawDocument, Sentence};

/// The envelope's version integer. It increments on any change to the
/// envelope's own shape or to the meaning of a field inside `result`.
///
/// cargo's precedent: `cargo metadata --format-version` is a versioned
/// integer with a compatibility policy rather than a published schema.
const FORMAT_VERSION: u32 = 1;

/// Success, and the command found something.
const EXIT_FOUND: u8 = 0;
/// Success, and the command found nothing.
const EXIT_EMPTY: u8 = 1;
/// An error occurred.
const EXIT_ERROR: u8 = 2;

type Fallible<T> = Result<T, Box<dyn std::error::Error>>;

/// Run the command line.
///
/// `args` is the full argument vector including the program name, the
/// way `std::env::args_os()` yields it. `out` receives everything a
/// reader is meant to consume (the rendered tables, the JSON, the
/// completion script, the help and version text); `err` receives
/// diagnostics.
///
/// Returns the process exit code. It is a `u8` rather than a
/// [`std::process::ExitCode`] because `ExitCode` is opaque: there is no
/// public way to read the number back out of one, so the Python launcher
/// could not turn it into an `int`. `src/bin/matra.rs` converts with
/// `ExitCode::from`.
///
/// Never panics on a caller's behalf and never exits the process. A
/// write failure on `out` is reported through the return code, not by
/// unwinding.
///
/// ```no_run
/// let mut out = Vec::new();
/// let mut err = Vec::new();
/// let code = matra::cli::run(
///     ["matra", "--version"].into_iter().map(Into::into),
///     &mut out,
///     &mut err,
/// );
/// assert_eq!(code, 0);
/// ```
pub fn run(
    args: impl IntoIterator<Item = OsString>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let cli = match parse(args) {
        Ok(cli) => cli,
        Err(early) => {
            // clap renders `--help` and `--version` as an "error" that is
            // not one. `use_stderr` is what separates them from a genuine
            // usage failure, and it decides both the stream and the code.
            let sink: &mut dyn Write = if early.to_stderr { err } else { out };
            let _ = write!(sink, "{}", early.text);
            return early.code;
        }
    };

    match execute(&cli, out) {
        Ok(Outcome::Found) => EXIT_FOUND,
        Ok(Outcome::Empty) => EXIT_EMPTY,
        Err(e) => {
            if is_broken_pipe(e.as_ref()) {
                return EXIT_FOUND;
            }
            let _ = writeln!(err, "matra: {e}");
            EXIT_ERROR
        }
    }
}

// ---------------------------------------------------------------------------
// The command line
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    about = "Text in, structured analysis out.",
    long_about = "matra parses text into CoNLL-U structure and measures it. \
                  It reports what is there; interpretation is yours."
)]
struct Cli {
    /// Emit JSON instead of a human-readable table.
    #[arg(long, global = true)]
    json: bool,

    /// Directory holding the UDPipe model. Downloads on first use.
    #[arg(long, global = true, value_name = "DIR")]
    model_dir: Option<PathBuf>,

    /// Suppress the human-readable output. Exit codes are unchanged, and
    /// `--json` is unaffected.
    #[arg(long, short, global = true)]
    quiet: bool,

    /// When to colorize output. `auto` also honors NO_COLOR.
    #[arg(long, global = true, value_enum, default_value_t = ColorWhen::Auto, value_name = "WHEN")]
    color: ColorWhen,

    /// Name to report for input read from stdin. Its extension selects
    /// the decomposer, so `notes.md` is read as markdown.
    #[arg(long, global = true, value_name = "NAME", default_value = "<stdin>")]
    stdin_filename: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse a document and report its metrics.
    Analyze {
        /// File to analyze, or `-` for stdin.
        path: PathBuf,
        /// Also print a per-section breakdown.
        #[arg(long, short)]
        sections: bool,
    },
    /// Extract the most representative sentences.
    Summarize {
        /// File to summarize, or `-` for stdin.
        path: PathBuf,
        /// Number of sentences to return. Defaults to `summarize.n`.
        #[arg(short)]
        n: Option<usize>,
        /// Ranking algorithm. Defaults to `summarize.algorithm`.
        #[arg(long, value_enum)]
        method: Option<SummaryMethod>,
    },
    /// Extract keyphrases.
    Keyphrases {
        /// File to extract from, or `-` for stdin.
        path: PathBuf,
        /// Maximum number of phrases to return. Defaults to `keyphrases.n`.
        #[arg(short)]
        n: Option<usize>,
        /// Extraction algorithm. Defaults to `keyphrases.algorithm`.
        #[arg(long, value_enum)]
        method: Option<KeyphraseMethod>,
    },
    /// Inspect and create the configuration file.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Print a shell completion script.
    Completions {
        /// Shell to generate for.
        #[arg(value_enum)]
        shell: CompletionShell,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Print every resolved value with the rung it came from.
    Show,
    /// Write the shipped defaults to the resolved config path.
    Init {
        /// Overwrite an existing file.
        #[arg(long)]
        force: bool,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum SummaryMethod {
    Tfidf,
    Textrank,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum KeyphraseMethod {
    Rake,
    Yake,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum ColorWhen {
    Auto,
    Always,
    Never,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum CompletionShell {
    Bash,
    Zsh,
    Fish,
}

/// What clap produced when it declined to hand back a parsed command
/// line: rendered text, the stream it belongs on, and the exit code.
struct EarlyExit {
    text: String,
    to_stderr: bool,
    code: u8,
}

fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Cli, EarlyExit> {
    // The program name is set here rather than taken from argv[0], so
    // `--help` reads the same whether the launcher was the Rust binary or
    // the Python entry point.
    //
    // Color is off in clap's own output because the two launchers must
    // produce the same bytes, and the Python launcher renders into a
    // buffer where a terminal check would be meaningless.
    let command = Cli::command()
        .name("matra")
        .bin_name("matra")
        .version(version_line())
        .color(clap::ColorChoice::Never);

    let matches = command.try_get_matches_from(args).map_err(early_exit)?;
    Cli::from_arg_matches(&matches).map_err(early_exit)
}

fn early_exit(e: clap::Error) -> EarlyExit {
    let to_stderr = e.use_stderr();
    EarlyExit {
        text: e.render().to_string(),
        to_stderr,
        // clap reports usage failures as exit 2, which is also matra's
        // "an error occurred". Help and version are not failures.
        code: if to_stderr { EXIT_ERROR } else { EXIT_FOUND },
    }
}

/// `matra <version>`, then the features this build was compiled with.
///
/// clap prints the binary name ahead of this string, so the first line
/// reads `matra 0.1.0` and the second names what is compiled in. The
/// list is built from `cfg!` so it cannot drift from the build.
fn version_line() -> &'static str {
    // clap holds its version as a `&'static str`, and the feature list is
    // only known once `cfg!` has been evaluated, so the built string is
    // interned here rather than leaked at every call.
    static VERSION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    VERSION.get_or_init(|| {
        let compiled = [
            ("udpipe", cfg!(feature = "udpipe")),
            ("model2vec", cfg!(feature = "model2vec")),
            ("python", cfg!(feature = "python")),
            ("cli", cfg!(feature = "cli")),
        ];
        let mut line = String::from(env!("CARGO_PKG_VERSION"));
        line.push_str("\nfeatures:");
        for (name, on) in compiled {
            if on {
                line.push(' ');
                line.push_str(name);
            }
        }
        line
    })
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

enum Outcome {
    Found,
    Empty,
}

fn outcome(empty: bool) -> Outcome {
    if empty {
        Outcome::Empty
    } else {
        Outcome::Found
    }
}

/// Where a subcommand's text comes from.
enum Input {
    Path(PathBuf),
    Stdin,
}

impl Input {
    fn of(path: &Path) -> Input {
        if path.as_os_str() == "-" {
            Input::Stdin
        } else {
            Input::Path(path.to_path_buf())
        }
    }

    /// The name this input answers to in the JSON envelope and in the
    /// human header.
    fn label<'a>(&'a self, stdin_filename: &'a str) -> std::borrow::Cow<'a, str> {
        match self {
            Input::Path(p) => p.display().to_string().into(),
            Input::Stdin => stdin_filename.into(),
        }
    }
}

fn execute(cli: &Cli, out: &mut dyn Write) -> Fallible<Outcome> {
    // The two config actions and the completion script touch neither the
    // model nor any input, so they are dispatched before anything is
    // loaded.
    match &cli.command {
        Command::Config { action } => return config_cmd::run(cli, action, out),
        Command::Completions { shell } => {
            let shell = match shell {
                CompletionShell::Bash => clap_complete::Shell::Bash,
                CompletionShell::Zsh => clap_complete::Shell::Zsh,
                CompletionShell::Fish => clap_complete::Shell::Fish,
            };
            let mut command = Cli::command().name("matra").bin_name("matra");
            clap_complete::generate(shell, &mut command, "matra", out);
            return Ok(Outcome::Found);
        }
        _ => {}
    }

    let path = match &cli.command {
        Command::Analyze { path, .. }
        | Command::Summarize { path, .. }
        | Command::Keyphrases { path, .. } => path,
        Command::Config { .. } | Command::Completions { .. } => unreachable!("dispatched above"),
    };
    let input = Input::of(path);

    // Validate the input before touching the model. Downloading 16 MB to
    // then report that the file does not exist is a poor trade for the
    // reader.
    if let Input::Path(p) = &input
        && !p.exists()
    {
        return Err(format!("no such file: {}", p.display()).into());
    }

    let cfg = resolve_config(cli)?;
    let engine = crate::Engine::from_config(&cfg)?;
    let doc = document_of(&input, &cli.stdin_filename, &engine)?;
    let label = input.label(&cli.stdin_filename);
    let style = render::Style::new(colorize(cli.color));

    match &cli.command {
        Command::Analyze { sections, .. } => {
            if cli.json {
                write_envelope(out, "analyze", &label, &doc)?;
            } else if !cli.quiet {
                render::metrics(out, &label, &doc, style)?;
                if *sections {
                    render::sections(out, &doc)?;
                }
            }
            Ok(outcome(doc.total_sentences() == 0))
        }
        Command::Summarize { n, method, .. } => {
            let sentences: Vec<Sentence> = doc.sentences().cloned().collect();
            let n = n.unwrap_or_else(|| cfg.summarize_n());
            let method = match method {
                Some(m) => *m,
                None => summary_method(cfg.summarize_algorithm())?,
            };
            let picked = match method {
                SummaryMethod::Tfidf => crate::extraction::tfidf_summarize(&sentences, n)?,
                SummaryMethod::Textrank => crate::extraction::textrank_summarize(&sentences, n)?,
            };
            if cli.json {
                write_envelope(out, "summarize", &label, &picked)?;
            } else if !cli.quiet {
                render::sentences(out, &picked)?;
            }
            Ok(outcome(picked.is_empty()))
        }
        Command::Keyphrases { n, method, .. } => {
            let sentences: Vec<Sentence> = doc.sentences().cloned().collect();
            let n = n.unwrap_or_else(|| cfg.keyphrases_n());
            let method = match method {
                Some(m) => *m,
                None => keyphrase_method(cfg.keyphrases_algorithm())?,
            };
            let phrases = match method {
                KeyphraseMethod::Rake => crate::extraction::rake_keyphrases(&sentences, n)?,
                KeyphraseMethod::Yake => crate::extraction::yake_keyphrases(&sentences, n)?,
            };
            if cli.json {
                write_envelope(out, "keyphrases", &label, &phrases)?;
            } else if !cli.quiet {
                render::phrases(out, &phrases)?;
            }
            Ok(outcome(phrases.is_empty()))
        }
        Command::Config { .. } | Command::Completions { .. } => unreachable!("dispatched above"),
    }
}

/// The resolved configuration, with `--model-dir` layered on top.
///
/// An explicit directory is the [`crate::config::ValueSource::Argument`]
/// rung, which outranks the environment and the file, so `config show`
/// under `--model-dir` reports where the value actually came from.
fn resolve_config(cli: &Cli) -> domain::Result<Config> {
    let cfg = Config::resolve()?;
    Ok(match &cli.model_dir {
        Some(dir) => cfg.with_model_dir(dir),
        None => cfg,
    })
}

fn summary_method(name: &str) -> Fallible<SummaryMethod> {
    match name {
        "tfidf" => Ok(SummaryMethod::Tfidf),
        "textrank" => Ok(SummaryMethod::Textrank),
        other => {
            Err(format!("summarize.algorithm is `{other}`, which this build cannot run").into())
        }
    }
}

fn keyphrase_method(name: &str) -> Fallible<KeyphraseMethod> {
    match name {
        "rake" => Ok(KeyphraseMethod::Rake),
        "yake" => Ok(KeyphraseMethod::Yake),
        other => {
            Err(format!("keyphrases.algorithm is `{other}`, which this build cannot run").into())
        }
    }
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

/// One analyzed document, through the pipeline.
///
/// Reading the file and parsing it directly would feed markdown headings
/// and fenced code to the extractors as if they were prose, so
/// `summarize` on a README would return its headings. Going through the
/// pipeline applies the right decomposer for the extension and skips
/// blockquotes.
fn document_of(input: &Input, stdin_filename: &str, engine: &crate::Engine) -> Fallible<Document> {
    match input {
        Input::Path(path) => {
            let mut stream = engine.analyze(crate::Ingest::path(path)?);
            match stream.next() {
                Some(Ok(entry)) => Ok(entry.analysis),
                Some(Err(e)) => Err(Box::new(e)),
                None => Err(format!("no documents at {}", path.display()).into()),
            }
        }
        Input::Stdin => {
            let text = read_stdin()?;
            let raw = RawDocument::new(text, None, format_of(Path::new(stdin_filename)));
            Ok(engine.analyze_one(raw)?.analysis)
        }
    }
}

/// Read stdin, refusing more than the pipeline's own cap.
///
/// The cap is checked while reading rather than after it: a stream is
/// unbounded by construction, and `read_to_string` on one would allocate
/// without limit before any gate could fire.
fn read_stdin() -> Fallible<String> {
    let mut text = String::new();
    io::stdin()
        .lock()
        .take(MAX_INPUT_BYTES as u64 + 1)
        .read_to_string(&mut text)?;
    if text.len() > MAX_INPUT_BYTES {
        return Err(Box::new(domain::Error::InputTooLarge {
            limit: MAX_INPUT_BYTES,
            actual: text.len(),
            what: "input",
        }));
    }
    Ok(text)
}

/// The format a name implies. Mirrors what the file source does with a
/// path on disk, so `--stdin-filename notes.md` is read as markdown.
fn format_of(name: &Path) -> Format {
    match name.extension().and_then(|e| e.to_str()) {
        Some("md" | "markdown") => Format::Markdown,
        Some("pdf") => Format::Pdf,
        Some("docx") => Format::Docx,
        _ => Format::PlainText,
    }
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

/// The one shape every `--json` invocation emits.
///
/// `result` is the serde form of the domain value, unchanged. Everything
/// that identifies the run sits beside it rather than inside it, so a
/// consumer can dispatch on `command` without inspecting `result` first.
#[derive(serde::Serialize)]
struct Envelope<'a, T> {
    format_version: u32,
    command: &'a str,
    input: &'a str,
    result: T,
}

fn write_envelope<T: serde::Serialize>(
    out: &mut dyn Write,
    command: &str,
    input: &str,
    result: T,
) -> Fallible<()> {
    let envelope = Envelope {
        format_version: FORMAT_VERSION,
        command,
        input,
        result,
    };
    writeln!(out, "{}", serde_json::to_string_pretty(&envelope)?)?;
    Ok(())
}

fn is_broken_pipe(e: &(dyn std::error::Error + 'static)) -> bool {
    let mut source = Some(e);
    while let Some(err) = source {
        if let Some(io_err) = err.downcast_ref::<io::Error>()
            && io_err.kind() == io::ErrorKind::BrokenPipe
        {
            return true;
        }
        source = err.source();
    }
    false
}

/// Whether ANSI styling belongs in this run's output.
///
/// The process is read here and the decision is made in [`decide_color`],
/// which takes both signals as arguments so the precedence is testable
/// without mutating the environment of a parallel test run.
fn colorize(when: ColorWhen) -> bool {
    let no_color = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
    decide_color(when, no_color, io::stdout().is_terminal())
}

/// `auto` colors only an interactive terminal, and only when `NO_COLOR`
/// is absent or empty: no-color.org disables on a variable that is
/// present and not an empty string, whatever its value. An explicit
/// `--color always` is the user asking for color in so many words, and
/// outranks `NO_COLOR`; `--color never` is the same request inverted.
///
/// `auto` looks at the process's own stdout rather than at the sink it
/// was handed, because that is where the bytes end up on both launchers:
/// the Rust binary writes stdout directly, and the Python launcher
/// buffers and then writes the same stdout.
fn decide_color(when: ColorWhen, no_color: bool, stdout_is_terminal: bool) -> bool {
    match when {
        ColorWhen::Always => true,
        ColorWhen::Never => false,
        ColorWhen::Auto => !no_color && stdout_is_terminal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_line_names_the_compiled_features() {
        let line = version_line();
        assert!(line.starts_with(env!("CARGO_PKG_VERSION")));
        assert!(line.contains("\nfeatures:"));
        // This module only compiles under `cli`, which implies `udpipe`.
        assert!(line.contains("udpipe"));
        assert!(line.contains("cli"));
    }

    #[test]
    fn dash_is_stdin_and_anything_else_is_a_path() {
        assert!(matches!(Input::of(Path::new("-")), Input::Stdin));
        assert!(matches!(Input::of(Path::new("./-")), Input::Path(_)));
        assert!(matches!(Input::of(Path::new("notes.md")), Input::Path(_)));
    }

    #[test]
    fn stdin_filename_selects_the_decomposer() {
        assert!(matches!(format_of(Path::new("notes.md")), Format::Markdown));
        assert!(matches!(
            format_of(Path::new("notes.markdown")),
            Format::Markdown
        ));
        assert!(matches!(format_of(Path::new("<stdin>")), Format::PlainText));
    }

    /// The full precedence table, since NO_COLOR is the one rule a
    /// reviewer is likely to get backwards.
    #[test]
    fn color_precedence() {
        for tty in [true, false] {
            for no_color in [true, false] {
                assert!(
                    decide_color(ColorWhen::Always, no_color, tty),
                    "--color always is explicit and wins"
                );
                assert!(
                    !decide_color(ColorWhen::Never, no_color, tty),
                    "--color never is explicit and wins"
                );
            }
        }
        assert!(
            decide_color(ColorWhen::Auto, false, true),
            "a bare terminal"
        );
        assert!(
            !decide_color(ColorWhen::Auto, true, true),
            "NO_COLOR disables on a terminal"
        );
        assert!(
            !decide_color(ColorWhen::Auto, false, false),
            "a pipe is not colored"
        );
    }
}
