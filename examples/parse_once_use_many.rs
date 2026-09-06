//! Parse once, use many: the no-double-parse pattern.
//!
//! When you want both a Document and one or more extractions over the
//! same text, run the pipeline once and read the sentences back off the
//! tree. The parse inside `annotate` is the single expensive step
//! (UDPipe runs a full dependency analysis on every sentence); the
//! extractors are cheap functions over the sentences it attached.
//!
//! Run with: cargo run --example parse_once_use_many
//! (requires UDPipe model at /tmp/matra-models/)

use matra::Engine;
use matra::domain::{Format, RawDocument, Sentence};
use matra::extraction::{rake_keyphrases, tfidf_summarize};
use matra::nlp::udpipe::Udpipe;

fn main() -> matra::domain::Result<()> {
    let nlp = Udpipe::english("/tmp/matra-models")?;
    let engine = Engine::new(Box::new(nlp), matra::standard_decomposers());

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

    // Annotate once: decompose, parse per paragraph, attach sentences.
    let raw = RawDocument::new(text.to_string(), None, Format::Markdown);
    let mut analysis = engine.annotate(&raw)?;
    engine.compose(&mut analysis);

    // Read the sentences back off the tree and hand them to multiple
    // consumers. Nothing is parsed twice.
    let sentences: Vec<Sentence> = analysis.sentences().cloned().collect();
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
