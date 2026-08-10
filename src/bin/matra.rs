//! matra command-line interface.
//!
//! Application tier. The library returns typed errors and structured data;
//! this binary decides how to render them, which exit code to use, and what
//! to do when a single file in a batch fails.
//!
//! Exit codes follow the ripgrep convention:
//!   0  success, and (where applicable) something was found
//!   1  success, but nothing was found
//!   2  an error occurred

use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use matra::domain::{Document, Keyphrase, ScoredSentence, Sentence};
use matra::nlp::NlpProvider;
use matra::nlp::udpipe::Udpipe;

#[derive(Parser)]
#[command(
    name = "matra",
    version,
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

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse a document and report its metrics.
    Analyze {
        /// File to analyze.
        path: PathBuf,
    },
    /// Extract the most representative sentences.
    Summarize {
        /// File to summarize.
        path: PathBuf,
        /// Number of sentences to return.
        #[arg(short, default_value_t = 3)]
        n: usize,
        /// Ranking algorithm.
        #[arg(long, value_enum, default_value_t = SummaryMethod::Tfidf)]
        method: SummaryMethod,
    },
    /// Extract keyphrases.
    Keyphrases {
        /// File to extract from.
        path: PathBuf,
        /// Maximum number of phrases to return.
        #[arg(short, default_value_t = 10)]
        n: usize,
        /// Extraction algorithm.
        #[arg(long, value_enum, default_value_t = KeyphraseMethod::Rake)]
        method: KeyphraseMethod,
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

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(Outcome::Found) => ExitCode::from(0),
        Ok(Outcome::Empty) => ExitCode::from(1),
        Err(e) => {
            // A broken pipe means the reader went away (`matra analyze x | head`).
            // That is not an error worth reporting.
            if is_broken_pipe(e.as_ref()) {
                return ExitCode::from(0);
            }
            eprintln!("matra: {e}");
            ExitCode::from(2)
        }
    }
}

enum Outcome {
    Found,
    Empty,
}

fn run(cli: &Cli) -> Result<Outcome, Box<dyn std::error::Error>> {
    // Validate the input before touching the model. Downloading 16 MB to then
    // report that the file does not exist is a poor trade for the reader.
    let input = match &cli.command {
        Command::Analyze { path }
        | Command::Summarize { path, .. }
        | Command::Keyphrases { path, .. } => path,
    };
    if !input.exists() {
        return Err(format!("no such file: {}", input.display()).into());
    }

    let nlp = load_model(cli.model_dir.as_deref())?;
    let mut out = io::stdout().lock();

    match &cli.command {
        Command::Analyze { path } => {
            let doc = matra::analyze_file(path, &nlp)?;
            if cli.json {
                writeln!(out, "{}", serde_json::to_string_pretty(&doc)?)?;
            } else {
                render_metrics(&mut out, path, &doc)?;
            }
            Ok(if doc.total_sentences() > 0 {
                Outcome::Found
            } else {
                Outcome::Empty
            })
        }
        Command::Summarize { path, n, method } => {
            let sentences = sentences_of(path, &nlp)?;
            let picked = match method {
                SummaryMethod::Tfidf => matra::extraction::tfidf_summarize(&sentences, *n)?,
                SummaryMethod::Textrank => matra::extraction::textrank_summarize(&sentences, *n)?,
            };
            if cli.json {
                writeln!(out, "{}", serde_json::to_string_pretty(&picked)?)?;
            } else {
                render_sentences(&mut out, &picked)?;
            }
            Ok(outcome(picked.is_empty()))
        }
        Command::Keyphrases { path, n, method } => {
            let sentences = sentences_of(path, &nlp)?;
            let phrases = match method {
                KeyphraseMethod::Rake => matra::extraction::rake_keyphrases(&sentences, *n)?,
                KeyphraseMethod::Yake => matra::extraction::yake_keyphrases(&sentences, *n)?,
            };
            if cli.json {
                writeln!(out, "{}", serde_json::to_string_pretty(&phrases)?)?;
            } else {
                render_phrases(&mut out, &phrases)?;
            }
            Ok(outcome(phrases.is_empty()))
        }
    }
}

fn outcome(empty: bool) -> Outcome {
    if empty {
        Outcome::Empty
    } else {
        Outcome::Found
    }
}

/// Resolve the model directory, defaulting to `~/.matra/models`.
fn load_model(explicit: Option<&Path>) -> Result<impl NlpProvider, Box<dyn std::error::Error>> {
    let dir = match explicit {
        Some(d) => d.to_path_buf(),
        None if std::env::var_os("MATRA_MODEL_DIR").is_some() => {
            PathBuf::from(std::env::var_os("MATRA_MODEL_DIR").expect("checked"))
        }
        None => home_dir()
            .ok_or("cannot determine home directory; pass --model-dir")?
            .join(".matra")
            .join("models"),
    };
    Ok(Udpipe::english(&dir)?)
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Sentences for the extractors, taken through the same format detection and
/// decomposition `analyze` uses.
///
/// Reading the file and calling `parse` on it directly would feed markdown
/// headings and fenced code to the extractors as if they were prose, so
/// `summarize` on a README returns its headings. Going through
/// `analyze_file` applies the right decomposer for the extension and skips
/// blockquotes, and the sentences it produces are the ones the extractors
/// should rank.
fn sentences_of(
    path: &Path,
    nlp: &dyn NlpProvider,
) -> Result<Vec<Sentence>, Box<dyn std::error::Error>> {
    let doc = matra::analyze_file(path, nlp)?;
    Ok(doc.sentences().cloned().collect())
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

fn render_metrics(out: &mut impl Write, path: &Path, doc: &Document) -> io::Result<()> {
    let bold = if io::stdout().is_terminal() {
        "\x1b[1m"
    } else {
        ""
    };
    let reset = if io::stdout().is_terminal() {
        "\x1b[0m"
    } else {
        ""
    };
    writeln!(out, "{bold}{}{reset}", path.display())?;
    writeln!(out, "  sentences          {}", doc.total_sentences())?;
    writeln!(out, "  words              {}", doc.total_words())?;
    writeln!(
        out,
        "  mean sentence len  {:.1}",
        doc.mean_sentence_length()
    )?;
    writeln!(out, "  sentence len sd    {:.1}", doc.sentence_length_std())?;
    writeln!(out, "  passive ratio      {:.3}", doc.passive_ratio())?;
    Ok(())
}

fn render_sentences(out: &mut impl Write, picked: &[ScoredSentence]) -> io::Result<()> {
    for s in picked {
        writeln!(out, "{:.3}  {}", s.score, s.text)?;
    }
    Ok(())
}

fn render_phrases(out: &mut impl Write, phrases: &[Keyphrase]) -> io::Result<()> {
    for p in phrases {
        writeln!(out, "{:.3}  {}", p.score, p.phrase)?;
    }
    Ok(())
}
