//! Parse once, use many: the no-double-parse pattern.
//!
//! When you want both an Document and one or more extractions over the
//! same text, parse the text once and hand the sentences to the
//! consumers. `parse` is the single expensive step (UDPipe runs a full
//! dependency analysis on every sentence); the other consumers are
//! cheap.
//!
//! Run with: cargo run --example parse_once_use_many
//! (requires UDPipe model at /tmp/matra-models/)

use matra::decompose::Decomposer;
use matra::decompose::markdown::MarkdownDecomposer;
use matra::extraction::{rake_keyphrases, tfidf_summarize};
use matra::nlp::udpipe::Udpipe;

fn main() -> matra::domain::Result<()> {
    let nlp = Udpipe::english("/tmp/matra-models")?;

    let text = "\
# Substrate libraries

A substrate library is one that other libraries are built on. \
It owns the data types, the boundary traits, and the discipline. \
Downstream consumers compose against the substrate; they do not \
modify it. The substrate is small and stable; opinions live in \
consumer code.

# Why hex for a substrate

Three forces pushed the hex shape. Variable I/O needs mean a CLI \
batch tool wants different ingestion than an editor streaming \
documents. Cross-language reach means the domain types must travel \
across FFI without dragging the adapters along. Pre-publish economics \
mean the public surface locks at 0.1.0 and stays small.
";

    // Decompose and parse once.
    let sections = MarkdownDecomposer.decompose(text);
    let sentences = matra::parse(text, &nlp)?;

    // Hand the parsed sentences to multiple consumers.
    let analysis = matra::analyze_from(sections, &sentences)?;
    let summary = tfidf_summarize(&sentences, 2)?;
    let phrases = rake_keyphrases(&sentences, 6)?;

    // Each consumer reads what it needs from the same parsed data.
    println!(
        "Document: {} sentences, {} words, passive ratio {:.1}%",
        analysis.total_sentences(),
        analysis.total_words(),
        analysis.passive_ratio() * 100.0,
    );

    println!("\nTop sentences (TF-IDF):");
    for (rank, sent) in summary.iter().enumerate() {
        println!("  {}. (score {:.3}) {}", rank + 1, sent.score, sent.text);
    }

    println!("\nKeyphrases (RAKE):");
    for kp in &phrases {
        println!("  {:.2}  {}", kp.score, kp.phrase);
    }

    Ok(())
}
