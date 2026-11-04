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
//!
//! Every failure has a kind, the same `kind` Rust's `Error::kind` and
//! Python's `MatraError.kind` name. Under `--json` a failure is still an
//! envelope on stdout, with `"result": null` and the error in `error`,
//! so a program reading stdout never has to parse prose from stderr. The
//! command line's own refusals (a directory where one file is read, a
//! missing `--skill`, an existing config file) use the kind whose remedy
//! matches: `invalid_input` for a call to change.

mod config_cmd;
mod render;
mod schema;
mod skill;

use std::ffi::OsString;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};

use crate::config::Config;
use crate::domain::limits::MAX_INPUT_BYTES;
use crate::domain::{self, Document, Format};

/// The envelope's version integer. It increments on any change to the
/// envelope's own shape or to the meaning of a field inside `result`.
///
/// cargo's precedent: `cargo metadata --format-version` is a versioned
/// integer with a compatibility policy rather than a published schema.
const FORMAT_VERSION: u32 = 2;

/// Success, and the command found something.
const EXIT_FOUND: u8 = 0;
/// Success, and the command found nothing.
const EXIT_EMPTY: u8 = 1;
/// An error occurred.
const EXIT_ERROR: u8 = 2;

/// The subcommands, by the name each answers to. A usage failure under
/// `--json` names the one its arguments asked for, read from this list
/// because clap reports no subcommand for a command line it refused. A
/// test holds it to the subcommands clap knows.
const COMMANDS: [&str; 6] = [
    "analyze",
    "summarize",
    "keyphrases",
    "clusters",
    "config",
    "completions",
];

type Fallible<T> = Result<T, Failure>;

/// Why a run failed.
#[derive(Debug)]
enum Failure {
    /// A failure with a kind. `--json` writes it as the envelope's
    /// `error`; stderr names `path` before the message when the failure
    /// concerns a file.
    Error {
        error: domain::Error,
        path: Option<PathBuf>,
    },
    /// Writing the output failed. There is nowhere left to write an
    /// envelope, and a broken pipe is no failure at all.
    Output(io::Error),
}

impl Failure {
    /// A refusal of the call as given: the `invalid_input` kind, whose
    /// remedy is to change the call.
    fn refused(message: impl Into<String>) -> Failure {
        Failure::Error {
            error: domain::Error::InvalidInput(message.into()),
            path: None,
        }
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failure::Error {
                error,
                path: Some(path),
            } => write!(f, "{}: {error}", path.display()),
            Failure::Error { error, path: None } => write!(f, "{error}"),
            Failure::Output(e) => write!(f, "cannot write the output: {e}"),
        }
    }
}

impl From<domain::Error> for Failure {
    fn from(error: domain::Error) -> Failure {
        Failure::Error { error, path: None }
    }
}

impl From<domain::DocumentError> for Failure {
    fn from(e: domain::DocumentError) -> Failure {
        Failure::Error {
            error: e.error,
            path: e.path,
        }
    }
}

/// Only the output is written with `?` on an `io::Error`. A read that
/// fails is a [`domain::Error`] at its call site, never this.
impl From<io::Error> for Failure {
    fn from(e: io::Error) -> Failure {
        Failure::Output(e)
    }
}

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
    let args: Vec<OsString> = args.into_iter().collect();
    let cli = match parse(args.iter().cloned()) {
        Ok(cli) => cli,
        Err(early) => {
            // clap renders `--help` and `--version` as an "error" that is
            // not one. `use_stderr` is what separates them from a genuine
            // usage failure, and it decides both the stream and the code.
            let sink: &mut dyn Write = if early.to_stderr {
                &mut *err
            } else {
                &mut *out
            };
            let _ = write!(sink, "{}", early.text);
            if early.to_stderr && asks_for_json(&args) {
                // A usage failure is a failure, and `--json` promises an
                // envelope for every failure. Its kind is the one whose
                // remedy is to change the call.
                let error = domain::Error::InvalidInput(early.summary);
                let _ = write_failure(out, command_named(&args), None, &error, Layout::Pretty);
            }
            return early.code;
        }
    };

    match execute(&cli, out, err) {
        Ok(Outcome::Found) => EXIT_FOUND,
        Ok(Outcome::Empty) => EXIT_EMPTY,
        Ok(Outcome::Failed) => EXIT_ERROR,
        Err(Failure::Output(e)) if e.kind() == io::ErrorKind::BrokenPipe => EXIT_FOUND,
        Err(failure) => {
            let _ = writeln!(err, "matra: {failure}");
            if let (true, Failure::Error { error, .. }) = (cli.json, &failure) {
                let (command, input) = failure_context(&cli);
                let _ = write_failure(out, Some(command), input.as_deref(), error, Layout::Pretty);
            }
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
    /// Emit JSON instead of a human-readable table. A failure is JSON
    /// too: the same envelope, with `result` null and the error in
    /// `error`.
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
    /// the format when `--format` does not, so `notes.md` is read as
    /// markdown.
    #[arg(long, global = true, value_name = "NAME", default_value = "<stdin>")]
    stdin_filename: String,

    /// Print the agent skill: what matra is for, every command with its
    /// JSON shape, and how to read the numbers. Outranks a subcommand.
    #[arg(long, global = true)]
    skill: bool,

    /// With `--skill`: print one reference, or list them all when no name
    /// is given.
    #[arg(
        long,
        short = 'r',
        global = true,
        value_name = "NAME",
        num_args = 0..=1,
    )]
    reference: Option<Option<String>>,

    /// Print the JSON Schema of one output (`document`, `summary`,
    /// `keyphrases`, `clusters`, `error`) or input (`blocks`), or list
    /// them all when no name is given. Outranks a subcommand.
    #[arg(
        long,
        global = true,
        value_name = "NAME",
        num_args = 0..=1,
        conflicts_with = "skill",
    )]
    schema: Option<Option<String>>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Parse a document, or every file in a directory, and report its
    /// metrics.
    Analyze {
        /// File or directory to analyze, or `-` for stdin. A directory
        /// is read file by file, in name order, one result each.
        path: PathBuf,
        /// Also print a per-section breakdown.
        #[arg(long, short)]
        sections: bool,
        /// Read the input as this format instead of detecting it.
        #[arg(long, value_enum, value_name = "FORMAT")]
        format: Option<FormatName>,
        /// How much of the parse `--json` emits: `document` (provenance,
        /// metrics, skipped), `paragraph` (and the sections with their
        /// paragraphs), `sentence` (and the sentences), or `token` (and
        /// the tokens, which is everything). Defaults to `token`.
        #[arg(long, value_enum, value_name = "LEVEL", requires = "json")]
        depth: Option<Depth>,
    },
    /// Extract the most representative sentences.
    Summarize {
        /// File to summarize, or `-` for stdin.
        path: PathBuf,
        /// Number of sentences to return. Defaults to `summary.n`.
        #[arg(short)]
        n: Option<usize>,
        /// Ranking method. Defaults to `summary.method`.
        #[arg(long, value_enum)]
        method: Option<SummaryMethod>,
        /// Read the input as this format instead of detecting it.
        #[arg(long, value_enum, value_name = "FORMAT")]
        format: Option<FormatName>,
    },
    /// Extract keyphrases.
    Keyphrases {
        /// File to extract from, or `-` for stdin.
        path: PathBuf,
        /// Maximum number of phrases to return. Defaults to `keyphrases.n`.
        #[arg(short)]
        n: Option<usize>,
        /// Ranking method. Defaults to `keyphrases.method`.
        #[arg(long, value_enum)]
        method: Option<KeyphraseMethod>,
        /// Read the input as this format instead of detecting it.
        #[arg(long, value_enum, value_name = "FORMAT")]
        format: Option<FormatName>,
    },
    /// Group a document's sentences by the similarity of their
    /// embeddings. Needs a build with the `model2vec` feature.
    Clusters {
        /// File to cluster, or `-` for stdin.
        path: PathBuf,
        /// The cosine similarity at which two sentences are joined.
        /// Defaults to `clusters.threshold`.
        #[arg(long, value_name = "SIMILARITY")]
        threshold: Option<f64>,
        /// Read the input as this format instead of detecting it.
        #[arg(long, value_enum, value_name = "FORMAT")]
        format: Option<FormatName>,
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

/// `--method` for `summarize`: the command line's spelling of
/// [`crate::SummaryMethod`], which [`summary_method_of`] translates.
#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum SummaryMethod {
    Tfidf,
    Textrank,
}

/// `--method` for `keyphrases`: the command line's spelling of
/// [`crate::KeyphraseMethod`], which [`keyphrase_method_of`] translates.
#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum KeyphraseMethod {
    Rake,
    Yake,
}

/// `--format`: the formats a caller can name, spelled as the JSON and
/// Python spell them, which [`format_of`] translates. PDF and DOCX are
/// recognised from their bytes and read by no build, so naming one would
/// only ever fail.
#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum FormatName {
    Markdown,
    #[value(name = "plain_text")]
    PlainText,
    Blocks,
}

/// `--depth`: how far into the parse `analyze --json` goes. Every token
/// shown costs a reader, an agent included, so a caller that needs only
/// the measures asks for no more.
#[derive(Copy, Clone, PartialEq, Eq, Debug, ValueEnum)]
enum Depth {
    /// Provenance, metrics and `skipped`: no `sections`.
    Document,
    /// And the sections with their paragraphs: no `sentences`.
    Paragraph,
    /// And the sentences with their record facts: no `tokens`.
    Sentence,
    /// And the tokens: the whole document.
    Token,
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
/// line: rendered text, the stream it belongs on, the exit code, and the
/// one-line account a `--json` envelope carries.
struct EarlyExit {
    text: String,
    to_stderr: bool,
    code: u8,
    summary: String,
}

fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Cli, EarlyExit> {
    // The program name is set here rather than taken from argv[0], so
    // `--help` reads the same whether the launcher was the Rust binary or
    // the Python entry point.
    //
    // Color is off in clap's own output because the two launchers must
    // produce the same bytes, and the Python launcher renders into a
    // buffer where a terminal check would be meaningless.
    let mut command = Cli::command()
        .name("matra")
        .bin_name("matra")
        .version(version_line())
        .color(clap::ColorChoice::Never);

    let matches = command.try_get_matches_from_mut(args).map_err(early_exit)?;
    let cli = Cli::from_arg_matches(&matches).map_err(early_exit)?;

    // The subcommand is optional in the derive because `--skill` and
    // `--schema` are whole invocations on their own. It is still required
    // of every other run, and the refusal is the one clap gave when the
    // field was not an `Option`: the short help on stderr, exit 2.
    // Rebuilding it here rather than letting a `None` fall through keeps
    // `matra` with no arguments printing what it has always printed.
    //
    // A bare `-r` is exempt so that it reaches the one message that says
    // what it is missing. Answering "you gave me no subcommand" to
    // someone who reached for the skill and forgot half the incantation
    // would be true and useless.
    if cli.command.is_none() && !cli.skill && cli.schema.is_none() && cli.reference.is_none() {
        return Err(EarlyExit {
            text: command.render_help().to_string(),
            to_stderr: true,
            code: EXIT_ERROR,
            summary: "no command given; run `matra --help` for the list".to_string(),
        });
    }
    Ok(cli)
}

fn early_exit(e: clap::Error) -> EarlyExit {
    let to_stderr = e.use_stderr();
    let text = e.render().to_string();
    // clap's first paragraph is the failure, sometimes over two lines
    // ("required arguments were not provided:" then the argument); the
    // rest is usage and a pointer to `--help`, which a program reading
    // the envelope has no use for.
    let summary = text
        .lines()
        .take_while(|line| !line.trim().is_empty())
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ")
        .trim_start_matches("error: ")
        .to_string();
    EarlyExit {
        text,
        to_stderr,
        // clap reports usage failures as exit 2, which is also matra's
        // "an error occurred". Help and version are not failures.
        code: if to_stderr { EXIT_ERROR } else { EXIT_FOUND },
        summary,
    }
}

/// Whether a raw argument vector asks for JSON: `--json` anywhere
/// before a bare `--`. Read raw because this is asked of a command line
/// clap refused, which has no parsed form.
fn asks_for_json(args: &[OsString]) -> bool {
    args.iter()
        .skip(1)
        .take_while(|arg| *arg != "--")
        .any(|arg| arg == "--json")
}

/// The command a raw argument vector names, for the envelope of a usage
/// failure: the first subcommand among the arguments, or `skill` or
/// `schema` for those flags. `None` when it names none.
fn command_named(args: &[OsString]) -> Option<&'static str> {
    args.iter()
        .skip(1)
        .take_while(|arg| *arg != "--")
        .filter_map(|arg| arg.to_str())
        .find_map(|arg| match arg {
            "--skill" | "-r" | "--reference" => Some("skill"),
            _ if arg == "--schema" || arg.starts_with("--schema=") => Some("schema"),
            _ => COMMANDS.iter().copied().find(|name| *name == arg),
        })
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
    /// At least one item of a walk failed, and each failure has already
    /// been reported where it happened. The walk went on past it.
    Failed,
}

fn outcome(empty: bool) -> Outcome {
    if empty {
        Outcome::Empty
    } else {
        Outcome::Found
    }
}

/// The command and the input a failure envelope names: the input is the
/// path the command was given, or the name given for stdin, and null for
/// a command that reads no document.
fn failure_context(cli: &Cli) -> (&'static str, Option<String>) {
    if cli.skill || (cli.reference.is_some() && cli.schema.is_none()) {
        return ("skill", None);
    }
    if cli.schema.is_some() {
        return ("schema", None);
    }
    let label = |path: &Path| Some(Target::label(path, &cli.stdin_filename));
    match &cli.command {
        Some(Command::Analyze { path, .. }) => ("analyze", label(path)),
        Some(Command::Summarize { path, .. }) => ("summarize", label(path)),
        Some(Command::Keyphrases { path, .. }) => ("keyphrases", label(path)),
        Some(Command::Clusters { path, .. }) => ("clusters", label(path)),
        Some(Command::Config { .. }) => ("config", None),
        Some(Command::Completions { .. }) => ("completions", None),
        // `parse` refuses a run with no command and neither flag.
        None => ("matra", None),
    }
}

fn execute(cli: &Cli, out: &mut dyn Write, err: &mut dyn Write) -> Fallible<Outcome> {
    // `--skill` and `--schema` are properties of the program rather than
    // actions on a document, so they outrank a subcommand: `matra analyze
    // x --skill` prints the skill and ignores the analysis. Dispatching
    // them first is what makes that true, and it is also what lets each
    // flag stand alone with no subcommand at all.
    if cli.skill {
        return skill::run(cli, out);
    }
    if let Some(name) = &cli.schema {
        return schema::run(cli, name.as_deref(), out);
    }
    if cli.reference.is_some() {
        return Err(Failure::refused(
            "--reference names a section of the agent skill; pass --skill with it, \
             or run `matra --skill -r` to list the references",
        ));
    }

    // `parse` refuses a run with no subcommand and neither flag, so there
    // is one by the time execution reaches here.
    let Some(command) = &cli.command else {
        return Err(Failure::refused(
            "no command given; run `matra --help` for the list",
        ));
    };

    match command {
        // The two config actions and the completion script touch neither
        // the model nor any input.
        Command::Config { action } => config_cmd::run(cli, action, out),
        Command::Completions { shell } => {
            let shell = match shell {
                CompletionShell::Bash => clap_complete::Shell::Bash,
                CompletionShell::Zsh => clap_complete::Shell::Zsh,
                CompletionShell::Fish => clap_complete::Shell::Fish,
            };
            let mut command = Cli::command().name("matra").bin_name("matra");
            clap_complete::generate(shell, &mut command, "matra", out);
            Ok(Outcome::Found)
        }
        Command::Analyze {
            path,
            sections,
            format,
            depth,
        } => analyze(
            cli,
            path,
            *format,
            *sections,
            depth.unwrap_or(Depth::Token),
            out,
            err,
        ),
        Command::Summarize {
            path,
            n,
            method,
            format,
        } => {
            let (label, doc, cfg) = one_document(cli, "summarize", path, *format, err)?;
            // The configuration applies here, at the application tier:
            // the library's own default is `summarize(doc, n)`, and the
            // command line always names the method it was told to use.
            let n = n.unwrap_or_else(|| cfg.summary_n());
            let method = match method {
                Some(m) => summary_method_of(*m),
                None => summary_method(cfg.summary_method())?,
            };
            let summary = crate::summarize_with(&doc, n, method)?;
            if cli.json {
                write_envelope(out, "summarize", Some(&label), &summary)?;
            } else if !cli.quiet {
                render::sentences(out, &summary)?;
            }
            Ok(outcome(summary.sentences.is_empty()))
        }
        Command::Keyphrases {
            path,
            n,
            method,
            format,
        } => {
            let (label, doc, cfg) = one_document(cli, "keyphrases", path, *format, err)?;
            let n = n.unwrap_or_else(|| cfg.keyphrases_n());
            let method = match method {
                Some(m) => keyphrase_method_of(*m),
                None => keyphrase_method(cfg.keyphrases_method())?,
            };
            let found = crate::keyphrases_with(&doc, n, method)?;
            if cli.json {
                write_envelope(out, "keyphrases", Some(&label), &found)?;
            } else if !cli.quiet {
                render::phrases(out, &found)?;
            }
            Ok(outcome(found.phrases.is_empty()))
        }
        Command::Clusters {
            path,
            threshold,
            format,
        } => clusters(cli, path, *threshold, *format, out, err),
    }
}

/// `matra clusters`: the document's sentences grouped by an embedder the
/// command line builds from the configuration. The library builds none
/// inside an outcome call, because building the default one downloads a
/// model; the application tier is where that choice belongs.
#[cfg(feature = "model2vec")]
fn clusters(
    cli: &Cli,
    path: &Path,
    threshold: Option<f64>,
    format: Option<FormatName>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Fallible<Outcome> {
    let (label, doc, cfg) = one_document(cli, "clusters", path, format, err)?;
    let threshold = threshold.unwrap_or_else(|| cfg.clusters_threshold());
    let embedder = build_embedder(cli, &cfg, err)?;
    let found = crate::clusters(&doc, &embedder, threshold)?;
    if cli.json {
        write_envelope(out, "clusters", Some(&label), &found)?;
    } else if !cli.quiet {
        render::clusters(out, &doc, &found)?;
    }
    Ok(outcome(found.clusters.is_empty()))
}

/// Without `model2vec` this build ships no embedder, and the command says
/// so before it reads anything or loads a model.
#[cfg(not(feature = "model2vec"))]
fn clusters(
    _cli: &Cli,
    _path: &Path,
    _threshold: Option<f64>,
    _format: Option<FormatName>,
    _out: &mut dyn Write,
    _err: &mut dyn Write,
) -> Fallible<Outcome> {
    Err(Failure::refused(
        "clusters needs an embedder, and this build was compiled without the `model2vec` \
         feature that ships one; install the Python wheel, or build with \
         `--features cli,model2vec`",
    ))
}

/// `matra analyze`: one document, or every file in a directory.
fn analyze(
    cli: &Cli,
    path: &Path,
    format: Option<FormatName>,
    sections: bool,
    depth: Depth,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Fallible<Outcome> {
    let style = render::Style::new(colorize(cli.color));
    let inputs = match Target::of(path)? {
        Target::Dir(inputs) => inputs,
        one => {
            let (label, doc, _) = analyzed(cli, one, path, format, err)?;
            if cli.json {
                write_document(out, Some(&label), &doc, depth, Layout::Pretty)?;
            } else if !cli.quiet {
                render::metrics(out, &label, &doc, style)?;
                if sections {
                    render::sections(out, &doc)?;
                }
            }
            return Ok(outcome(doc.metrics.total_sentences == 0));
        }
    };

    if format.is_some() {
        return Err(Failure::refused(format!(
            "--format reads one file or stdin as the format named; {} is a directory, \
             and each of its files is read as its own name and bytes say",
            path.display()
        )));
    }
    let cfg = resolve_config(cli)?;
    let engine = build_engine(cli, &cfg, err)?;

    // One result per file, in name order, and a failed file is a result
    // too: its line says what went wrong, and the walk goes on. The exit
    // code says whether any file failed.
    let mut found = false;
    let mut failed = false;
    let mut first = true;
    for item in engine.analyze_all(inputs) {
        match item {
            Ok(doc) => {
                let label = doc
                    .provenance
                    .source
                    .path
                    .as_deref()
                    .map(|p| p.display().to_string());
                found |= doc.metrics.total_sentences > 0;
                if cli.json {
                    write_document(out, label.as_deref(), &doc, depth, Layout::Line)?;
                } else if !cli.quiet {
                    if !first {
                        writeln!(out)?;
                    }
                    render::metrics(out, label.as_deref().unwrap_or_default(), &doc, style)?;
                    if sections {
                        render::sections(out, &doc)?;
                    }
                }
                first = false;
            }
            Err(e) => {
                failed = true;
                // A diagnostic that cannot be written is not worth
                // stopping the walk over.
                let _ = writeln!(err, "matra: {e}");
                if cli.json {
                    let label = e.path.as_deref().map(|p| p.display().to_string());
                    write_failure(
                        out,
                        Some("analyze"),
                        label.as_deref(),
                        &e.error,
                        Layout::Line,
                    )?;
                }
            }
        }
    }
    Ok(if failed {
        Outcome::Failed
    } else {
        outcome(!found)
    })
}

/// The one document `summarize`, `keyphrases` and `clusters` read, with
/// its label and the configuration that resolved. A directory is refused
/// before the engine is built: these commands report on one document.
fn one_document(
    cli: &Cli,
    command: &str,
    path: &Path,
    format: Option<FormatName>,
    err: &mut dyn Write,
) -> Fallible<(String, Document, Config)> {
    let target = Target::of(path)?;
    if let Target::Dir(_) = target {
        return Err(Failure::refused(format!(
            "{} is a directory; {command} reads one document, and `matra analyze` reads a \
             directory",
            path.display()
        )));
    }
    analyzed(cli, target, path, format, err)
}

/// One file or stdin, analysed: its label, the document, and the
/// configuration the engine was built from.
///
/// Stdin is read before the engine is built, so an oversized stream
/// costs no model load.
fn analyzed(
    cli: &Cli,
    target: Target,
    path: &Path,
    format: Option<FormatName>,
    err: &mut dyn Write,
) -> Fallible<(String, Document, Config)> {
    let format = format.map(format_of);
    let (input, file) = match target {
        Target::File(file) => {
            let input = match &format {
                Some(format) => crate::Input::file_as(&file, format.clone()),
                None => crate::Input::file(&file),
            };
            (input, Some(file))
        }
        Target::Stdin => {
            // The name given for stdin stands in for a file name: its
            // extension decides the format when `--format` does not, and
            // a PDF or DOCX signature in the bytes outranks both.
            let format = format.unwrap_or_else(|| Format::from_path(&cli.stdin_filename));
            (crate::Input::bytes(read_stdin()?, Some(format)), None)
        }
        Target::Dir(_) => unreachable!("a directory is walked, never analysed as one document"),
    };
    let cfg = resolve_config(cli)?;
    let engine = build_engine(cli, &cfg, err)?;
    let doc = engine
        .analyze(input)
        .map_err(|error| Failure::Error { error, path: file })?;
    Ok((Target::label(path, &cli.stdin_filename), doc, cfg))
}

/// The standard pipeline, saying so on stderr before it downloads a
/// model it does not have.
///
/// The first run fetches 16 MB from a university server in Prague.
/// Measured cold starts ran from 3 to 35 seconds, and until this line
/// existed every one of them wrote zero bytes to either stream before
/// the result: a blank terminal, indistinguishable from a hung process,
/// under a README section titled "No setup". The notice fires only when
/// the model is not already on disk, so a warm run is as quiet as it
/// ever was.
///
/// It goes to stderr, which keeps `--json` stdout a single object, and
/// `--quiet` silences it: that flag means "no human-readable output",
/// and this is human-readable output.
fn build_engine(cli: &Cli, cfg: &Config, err: &mut dyn Write) -> domain::Result<crate::Engine> {
    if cli.quiet {
        return crate::Engine::from_config(cfg);
    }
    crate::Engine::from_config_with_notice(cfg, |notice| announce(err, notice))
}

/// The configured embedder, saying so on stderr before it downloads the
/// 30.9 MB it does not have, in the same two lines and under the same
/// `--quiet` as the parser's notice ([`build_engine`]). Until it could,
/// the first `matra clusters` waited on that download with nothing on
/// either stream.
#[cfg(feature = "model2vec")]
fn build_embedder(
    cli: &Cli,
    cfg: &Config,
    err: &mut dyn Write,
) -> domain::Result<crate::embed::model2vec::Model2Vec> {
    if cli.quiet {
        return crate::embed::model2vec::Model2Vec::from_config(cfg);
    }
    crate::embed::model2vec::Model2Vec::from_config_with_notice(cfg, |notice| {
        announce(err, notice);
    })
}

/// Write a download notice to stderr. A diagnostic that cannot be
/// written is not worth failing a run over, and there is nowhere left to
/// report it to.
fn announce(err: &mut dyn Write, notice: &domain::ProvisionNotice) {
    let _ = err.write_all(notice_text(notice).as_bytes());
    let _ = err.flush();
}

/// What a first run writes to stderr before the fetch: the artifact,
/// its size and destination, then the license it is published under.
///
/// The second line exists because the model is a separate work from
/// matra with terms of its own, and the download is the one moment
/// every user of the command line passes through. The license and its
/// URL come from the notice, pinned beside the model's URL, so a change
/// of model cannot leave this line naming the old terms.
fn notice_text(notice: &domain::ProvisionNotice) -> String {
    format!(
        "matra: downloading {} ({}) into {}\n\
         matra: the model is licensed {}, separately from matra: {}\n",
        notice.artifact,
        human_bytes(notice.bytes),
        notice.destination.display(),
        notice.license,
        notice.license_url
    )
}

/// A byte count as a person reads a download size, in decimal units,
/// which is what a browser, `curl` and every hosting page report.
fn human_bytes(bytes: u64) -> String {
    #[allow(clippy::cast_precision_loss)]
    let n = bytes as f64;
    if bytes >= 1_000_000 {
        format!("{:.1} MB", n / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1} kB", n / 1_000.0)
    } else {
        format!("{bytes} bytes")
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

/// The library's format for `--format`.
fn format_of(name: FormatName) -> Format {
    match name {
        FormatName::Markdown => Format::Markdown,
        FormatName::PlainText => Format::PlainText,
        FormatName::Blocks => Format::Blocks,
    }
}

/// The library's method for `--method`.
fn summary_method_of(method: SummaryMethod) -> crate::SummaryMethod {
    match method {
        SummaryMethod::Tfidf => crate::SummaryMethod::TfIdf,
        SummaryMethod::Textrank => crate::SummaryMethod::TextRank,
    }
}

/// The library's method for `--method`.
fn keyphrase_method_of(method: KeyphraseMethod) -> crate::KeyphraseMethod {
    match method {
        KeyphraseMethod::Rake => crate::KeyphraseMethod::Rake,
        KeyphraseMethod::Yake => crate::KeyphraseMethod::Yake,
    }
}

/// The library's method for the configured `summary.method`. The names
/// are the flag's, which `Config::resolve` already checked.
fn summary_method(name: &str) -> Fallible<crate::SummaryMethod> {
    match name {
        "tfidf" => Ok(crate::SummaryMethod::TfIdf),
        "textrank" => Ok(crate::SummaryMethod::TextRank),
        other => Err(Failure::refused(format!(
            "summary.method is `{other}`, which this build cannot run"
        ))),
    }
}

/// The library's method for the configured `keyphrases.method`.
fn keyphrase_method(name: &str) -> Fallible<crate::KeyphraseMethod> {
    match name {
        "rake" => Ok(crate::KeyphraseMethod::Rake),
        "yake" => Ok(crate::KeyphraseMethod::Yake),
        other => Err(Failure::refused(format!(
            "keyphrases.method is `{other}`, which this build cannot run"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

/// What a command's path argument names.
enum Target {
    /// `-`: the process's stdin.
    Stdin,
    /// A path that is not a directory. Whether it is a readable file is
    /// the library's to say, when it is analysed.
    File(PathBuf),
    /// A directory, listed.
    Dir(crate::Inputs),
}

impl Target {
    /// What `path` names, decided before the engine is built.
    ///
    /// The question goes to [`crate::Inputs::dir`], which answers it for
    /// the library: a directory is listed, and anything else is refused
    /// with `invalid_input` naming `Input::file`, which is what reads a
    /// file. A path that is missing, unreadable or a symlink fails there
    /// with the error the library gives for it, path detail and all, and
    /// it fails before a model is loaded: downloading 16 MB to then
    /// report that the file does not exist is a poor trade.
    fn of(path: &Path) -> Fallible<Target> {
        if path.as_os_str() == "-" {
            return Ok(Target::Stdin);
        }
        match crate::Inputs::dir(path) {
            Ok(inputs) => Ok(Target::Dir(inputs)),
            Err(domain::Error::InvalidInput(_)) => Ok(Target::File(path.to_path_buf())),
            Err(error) => Err(Failure::Error { error, path: None }),
        }
    }

    /// The name an input answers to in the JSON envelope and in the
    /// human header: the path as given, or the name given for stdin.
    fn label(path: &Path, stdin_filename: &str) -> String {
        if path.as_os_str() == "-" {
            stdin_filename.to_string()
        } else {
            path.display().to_string()
        }
    }
}

/// Read the process's stdin, capped.
///
/// The process is read here and the decision is made in [`read_capped`],
/// which takes the stream as an argument so the cap is testable without
/// a subprocess.
fn read_stdin() -> Fallible<Vec<u8>> {
    read_capped(&mut io::stdin().lock())
}

/// Read `source`, refusing more than the pipeline's own cap.
///
/// The cap is checked while reading rather than after it: a stream is
/// unbounded by construction, and reading one to the end would allocate
/// without limit before any gate could fire.
///
/// The bytes are returned undecoded. Whether they are text, and which
/// format, is the library's to decide, the same way it decides for a
/// file: stdin is read as bytes and detected.
fn read_capped(source: &mut dyn Read) -> Fallible<Vec<u8>> {
    let mut bytes: Vec<u8> = Vec::new();
    source
        .take(MAX_INPUT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(domain::Error::Io)?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(domain::Error::InputTooLarge {
            limit: MAX_INPUT_BYTES,
            actual: bytes.len(),
            what: "input",
        }
        .into());
    }
    Ok(bytes)
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

/// The one shape every `--json` invocation emits.
///
/// `result` is the serde form of the domain value, unchanged. Everything
/// that identifies the run sits beside it rather than inside it, so a
/// consumer can dispatch on `command` without inspecting `result` first.
///
/// The five keys are always present, so a consumer that reads them never
/// has to discover that one is sometimes missing. Exactly one of
/// `result` and `error` is null: `result` on a failure, `error` on a
/// success. `input` is null for a command that reads no document
/// (`--skill`, `--schema`, `config show` with no config file). `command`
/// is null only for a usage failure whose arguments name no command.
#[derive(serde::Serialize)]
struct Envelope<'a, T> {
    format_version: u32,
    command: Option<&'a str>,
    input: Option<&'a str>,
    result: Option<T>,
    error: Option<&'a domain::Error>,
}

/// How an envelope is laid out: indented, for a run that writes one, or
/// on one line, for a directory walk that writes one per file.
#[derive(Copy, Clone)]
enum Layout {
    Pretty,
    Line,
}

/// A successful run's envelope.
fn write_envelope<T: serde::Serialize>(
    out: &mut dyn Write,
    command: &str,
    input: Option<&str>,
    result: T,
) -> Fallible<()> {
    let envelope = Envelope {
        format_version: FORMAT_VERSION,
        command: Some(command),
        input,
        result: Some(result),
        error: None,
    };
    Ok(emit(out, &envelope, Layout::Pretty)?)
}

/// A failed run's envelope, or a failed file's in a walk.
fn write_failure(
    out: &mut dyn Write,
    command: Option<&str>,
    input: Option<&str>,
    error: &domain::Error,
    layout: Layout,
) -> io::Result<()> {
    let envelope: Envelope<'_, ()> = Envelope {
        format_version: FORMAT_VERSION,
        command,
        input,
        result: None,
        error: Some(error),
    };
    emit(out, &envelope, layout)
}

/// An analysed document's envelope, cut to `depth`.
fn write_document(
    out: &mut dyn Write,
    input: Option<&str>,
    doc: &Document,
    depth: Depth,
    layout: Layout,
) -> Fallible<()> {
    fn envelope<'a, T>(input: Option<&'a str>, result: T) -> Envelope<'a, T> {
        Envelope {
            format_version: FORMAT_VERSION,
            command: Some("analyze"),
            input,
            result: Some(result),
            error: None,
        }
    }
    if depth == Depth::Token {
        // The whole document, written straight from the domain value.
        return Ok(emit(out, &envelope(input, doc), layout)?);
    }
    let cut = at_depth(doc, depth)?;
    Ok(emit(out, &envelope(input, cut), layout)?)
}

/// The document's JSON with what lies below `depth` removed: the key, not
/// an empty array in its place, because an empty `sentences` would say a
/// paragraph has none. The document schema marks the three keys this can
/// remove as absent only here.
///
/// The keys keep the order the whole document is written in: the `cli`
/// feature turns on `serde_json/preserve_order`, so the `Value` keeps the
/// domain types' declaration order rather than sorting it, and
/// `shift_remove` takes a key out without moving the others.
fn at_depth(doc: &Document, depth: Depth) -> io::Result<serde_json::Value> {
    let mut value = serde_json::to_value(doc).map_err(io::Error::other)?;
    let each = |value: &mut serde_json::Value, key: &str| -> Vec<serde_json::Value> {
        value
            .get_mut(key)
            .and_then(serde_json::Value::as_array_mut)
            .map(std::mem::take)
            .unwrap_or_default()
    };
    match depth {
        Depth::Token => {}
        Depth::Document => {
            if let Some(object) = value.as_object_mut() {
                object.shift_remove("sections");
            }
        }
        Depth::Paragraph | Depth::Sentence => {
            let mut sections = each(&mut value, "sections");
            for section in &mut sections {
                let mut paragraphs = each(section, "paragraphs");
                for paragraph in &mut paragraphs {
                    if depth == Depth::Paragraph {
                        if let Some(object) = paragraph.as_object_mut() {
                            object.shift_remove("sentences");
                        }
                        continue;
                    }
                    let mut sentences = each(paragraph, "sentences");
                    for sentence in &mut sentences {
                        if let Some(object) = sentence.as_object_mut() {
                            object.shift_remove("tokens");
                        }
                    }
                    paragraph["sentences"] = serde_json::Value::Array(sentences);
                }
                section["paragraphs"] = serde_json::Value::Array(paragraphs);
            }
            value["sections"] = serde_json::Value::Array(sections);
        }
    }
    Ok(value)
}

fn emit<T: serde::Serialize>(
    out: &mut dyn Write,
    envelope: &Envelope<'_, T>,
    layout: Layout,
) -> io::Result<()> {
    let text = match layout {
        Layout::Pretty => serde_json::to_string_pretty(envelope),
        Layout::Line => serde_json::to_string(envelope),
    }
    .map_err(io::Error::other)?;
    writeln!(out, "{text}")
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

    /// The list a usage failure reads its command from is the list clap
    /// parses. A subcommand added to one and not the other fails here.
    #[test]
    fn the_command_list_is_the_subcommands_clap_knows() {
        let known: Vec<String> = Cli::command()
            .get_subcommands()
            .map(|c| c.get_name().to_string())
            .collect();
        assert_eq!(known, COMMANDS);
    }

    #[test]
    fn dash_is_stdin_and_a_directory_is_listed() {
        let dir = tempfile::tempdir().expect("temp dir");
        assert!(matches!(Target::of(Path::new("-")), Ok(Target::Stdin)));
        assert!(matches!(Target::of(dir.path()), Ok(Target::Dir(_))));
        let file = dir.path().join("notes.md");
        std::fs::write(&file, "text").expect("write");
        assert!(matches!(Target::of(&file), Ok(Target::File(_))));
        assert_eq!(Target::label(Path::new("-"), "notes.md"), "notes.md");
        assert_eq!(Target::label(&file, "notes.md"), file.display().to_string());
    }

    /// A missing path fails before the engine is built, with the error the
    /// library gives for it: the `io` kind, `not_found`, and the path as a
    /// detail as well as in the message.
    #[test]
    fn a_missing_path_is_the_library_s_not_found() {
        let dir = tempfile::tempdir().expect("temp dir");
        let missing = dir.path().join("absent.txt");
        let Err(Failure::Error { error, .. }) = Target::of(&missing) else {
            panic!("a missing path is refused");
        };
        assert_eq!(error.kind(), "io");
        let details = error.details();
        assert_eq!(
            details.get("io_kind"),
            Some(&domain::DetailValue::Text("not_found".to_string()))
        );
        assert_eq!(
            details.get("path"),
            Some(&domain::DetailValue::Text(missing.display().to_string()))
        );
    }

    #[test]
    fn json_is_asked_for_before_a_bare_double_dash_only() {
        let args = |list: &[&str]| -> Vec<OsString> { list.iter().map(OsString::from).collect() };
        assert!(asks_for_json(&args(&["matra", "analyze", "--json"])));
        assert!(!asks_for_json(&args(&["matra", "analyze", "--", "--json"])));
        assert!(!asks_for_json(&args(&["--json"])), "argv[0] is the program");
        assert_eq!(
            command_named(&args(&["matra", "--json", "summarize", "x"])),
            Some("summarize")
        );
        assert_eq!(
            command_named(&args(&["matra", "--schema=x"])),
            Some("schema")
        );
        assert_eq!(command_named(&args(&["matra", "--json", "--bogus"])), None);
    }

    /// Regression: an oversized input whose cap-sized prefix ends inside
    /// a multi-byte character used to be reported as invalid UTF-8,
    /// because the read decoded before it counted. The bytes are now
    /// counted and never decoded here, so the size is what is reported.
    #[test]
    fn oversized_input_is_too_large_not_invalid_utf8() {
        let text = "é".repeat(MAX_INPUT_BYTES / 2 + 1);
        assert!(text.len() > MAX_INPUT_BYTES);

        let Err(Failure::Error { error, .. }) = read_capped(&mut text.as_bytes()) else {
            panic!("over the cap");
        };
        assert!(
            matches!(
                error,
                domain::Error::InputTooLarge {
                    limit: MAX_INPUT_BYTES,
                    what: "input",
                    ..
                }
            ),
            "{error}"
        );
    }

    /// An input exactly at the cap is read, and its bytes come back
    /// unchanged. The cap is an upper bound, not an exclusive one.
    #[test]
    fn input_at_the_cap_is_read_whole() {
        let text = "a".repeat(MAX_INPUT_BYTES);
        let read = read_capped(&mut text.as_bytes()).expect("at the cap");
        assert_eq!(read.len(), MAX_INPUT_BYTES);
    }

    /// Bytes that are not UTF-8 come back as they arrived: whether they
    /// are text is the library's decision, which reports them as an
    /// unsupported format, the same as for a file.
    #[test]
    fn stdin_bytes_are_not_decoded_here() {
        let read = read_capped(&mut &b"ok\xff\xfe"[..]).expect("under the cap");
        assert_eq!(read, b"ok\xff\xfe");
    }

    /// `--depth` removes the levels below it as keys, never as empty
    /// arrays: an empty `sentences` would claim a paragraph has none.
    #[test]
    fn depth_removes_the_levels_below_it() {
        let doc = Document::new(
            domain::Provenance::for_test(),
            vec![domain::Section::new(
                Some("Notes".to_string()),
                1,
                None,
                Vec::new(),
            )],
            Vec::new(),
        );
        let document = at_depth(&doc, Depth::Document).expect("value");
        assert!(document.get("sections").is_none());
        assert!(document.get("metrics").is_some());
        assert!(document.get("provenance").is_some());

        let paragraph = at_depth(&doc, Depth::Paragraph).expect("value");
        assert_eq!(paragraph["sections"][0]["heading"], "Notes");
    }

    /// `--depth` output keeps the keys in the order the whole document is
    /// written in, the domain types' declaration order, so a cut document
    /// diffs against the whole one line for line. Built through a sorted
    /// map, it ordered every object alphabetically instead.
    #[test]
    fn depth_keeps_the_key_order_of_the_whole_document() {
        let mut paragraph = domain::Paragraph::new(
            "A sentence.".to_string(),
            domain::ParagraphKind::Paragraph,
            domain::Span::new(0, 11),
        );
        paragraph
            .sentences
            .push(domain::Sentence::new("A sentence.".to_string(), Vec::new()));
        let doc = Document::new(
            domain::Provenance::for_test(),
            vec![domain::Section::new(None, 0, None, vec![paragraph])],
            Vec::new(),
        );
        // The whole document as `--json` writes it, read back in order.
        let whole: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&doc).expect("json")).expect("parse");
        let keys = |value: &serde_json::Value| -> Vec<String> {
            value
                .as_object()
                .expect("an object")
                .keys()
                .cloned()
                .collect()
        };
        let without = |value: &serde_json::Value, gone: &str| -> Vec<String> {
            keys(value).into_iter().filter(|k| k != gone).collect()
        };

        let paragraph_whole = &whole["sections"][0]["paragraphs"][0];
        let cut = at_depth(&doc, Depth::Paragraph).expect("value");
        assert_eq!(
            keys(&cut["sections"][0]["paragraphs"][0]),
            without(paragraph_whole, "sentences")
        );
        let cut = at_depth(&doc, Depth::Sentence).expect("value");
        assert_eq!(
            keys(&cut["sections"][0]["paragraphs"][0]["sentences"][0]),
            without(&paragraph_whole["sentences"][0], "tokens")
        );
        assert_eq!(
            keys(&cut["sections"][0]["paragraphs"][0]),
            keys(paragraph_whole)
        );
        let cut = at_depth(&doc, Depth::Document).expect("value");
        assert_eq!(keys(&cut), without(&whole, "sections"));

        // The declaration order is not the alphabetical one, so a sorted
        // map would fail the assertions above.
        let mut sorted = keys(paragraph_whole);
        sorted.sort();
        assert_ne!(keys(paragraph_whole), sorted);
    }

    /// Regression: the notice a first run prints names the artifact, a
    /// size a person can read, and the directory it is going into. Every
    /// user meets that line on their first command, and before it
    /// existed they met 3 to 35 seconds of a blank terminal instead.
    ///
    /// The second line names the model's license. The parsing model is
    /// CC BY-NC-SA 4.0, not matra's MIT, and until this line existed
    /// nothing matra printed or published said so.
    #[test]
    fn the_download_notice_names_the_artifact_size_destination_and_license() {
        let notice = domain::ProvisionNotice {
            artifact: "english-ewt-ud-2.5-191206.udpipe".to_string(),
            bytes: 16_309_608,
            destination: PathBuf::from("/home/u/.local/share/matra/models"),
            license: "CC BY-NC-SA 4.0 (non-commercial)".to_string(),
            license_url: "https://creativecommons.org/licenses/by-nc-sa/4.0/".to_string(),
        };
        assert_eq!(
            notice_text(&notice),
            "matra: downloading english-ewt-ud-2.5-191206.udpipe (16.3 MB) \
into /home/u/.local/share/matra/models\n\
matra: the model is licensed CC BY-NC-SA 4.0 (non-commercial), separately from matra: \
https://creativecommons.org/licenses/by-nc-sa/4.0/\n"
        );
    }

    #[test]
    fn byte_counts_read_the_way_a_download_size_reads() {
        assert_eq!(human_bytes(16_309_608), "16.3 MB");
        assert_eq!(human_bytes(1_000_000), "1.0 MB");
        assert_eq!(human_bytes(2_048), "2.0 kB");
        assert_eq!(human_bytes(999), "999 bytes");
        assert_eq!(human_bytes(0), "0 bytes");
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
