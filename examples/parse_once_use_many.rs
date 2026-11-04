//! Parse once, use many: the no-double-parse pattern.
//!
//! When you want both a Document and one or more outcomes over the same
//! text, analyze once and hand the document to each outcome function.
//! The parse inside `analyze` is the single expensive step (UDPipe runs
//! a full dependency analysis on every sentence); `summarize` and
//! `keyphrases` are cheap functions over the document it returns.
//!
//! Run with: cargo run --example parse_once_use_many
//! (downloads the UDPipe model into the configured model directory on
//! first use; `matra config show` prints where that is)

use matra::domain::Format;
use matra::{Engine, Input};

fn main() -> matra::domain::Result<()> {
    let engine = Engine::new()?;

    let text = "\
# Core libraries

A core library is one that other code is built on. \
It owns the data types, the boundary traits, and the discipline. \
Downstream code composes against the core; it does not \
modify it. The core is small and stable; interpretation lives in \
the caller's code.

# Why hex for a core library

Three forces pushed the hex shape. Variable I/O needs mean a CLI \
batch tool wants different ingestion than an editor streaming \
documents. Cross-language reach means the domain types must travel \
across FFI without dragging the adapters along. Pre-publish economics \
mean the public surface locks at 0.1.0 and stays small.
";

    // Analyze once: read, parse per paragraph, attach sentences, measure.
    let analysis = engine.analyze(Input::text(text, Format::Markdown))?;

    // Hand the same document to each outcome. Nothing is parsed twice.
    let summary = matra::summarize(&analysis, 2)?;
    let phrases = matra::keyphrases(&analysis, 6)?;

    // Each consumer reads what it needs from the same parsed data.
    println!(
        "Document: {} sentences, {} words, passive ratio {:.1}%",
        analysis.metrics.total_sentences,
        analysis.metrics.total_words,
        analysis.metrics.passive_ratio.unwrap_or(0.0) * 100.0,
    );

    // Each result names the method that ranked it.
    println!(
        "\nTop sentences ({}):",
        summary.provenance.method.as_deref().unwrap_or("?")
    );
    for (rank, sent) in summary.sentences.iter().enumerate() {
        println!("  {}. (score {:.3}) {}", rank + 1, sent.score, sent.text);
    }

    println!(
        "\nKeyphrases ({}):",
        phrases.provenance.method.as_deref().unwrap_or("?")
    );
    for kp in &phrases.phrases {
        println!(
            "  {:.2}  {} ({}x)",
            kp.score,
            kp.phrase,
            kp.occurrences.len()
        );
    }

    Ok(())
}
