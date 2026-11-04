//! Corpus analysis with per-document error collection.
//!
//! `Inputs::dir` lists every regular file in a directory, and
//! `analyze_all` streams them through the pipeline, one result per file
//! in path order: a `Document` naming its path in
//! `provenance.source.path`, or a `DocumentError` recording why that one
//! file failed.
//!
//! Per-file failures never abort the corpus walk. A symlink, an
//! oversized file, or a UDPipe panic on one document leaves the rest
//! of the corpus intact and the failure recorded beside it.
//!
//! How to aggregate across documents is the caller's decision, so this
//! example makes it in the open: it sums counts, which add across
//! documents, and reports nothing that does not (a vocabulary ratio
//! depends on each document's length).
//!
//! Run with: cargo run --example corpus -- <directory>
//! (downloads the UDPipe model into the configured model directory on
//! first use; `matra config show` prints where that is)

use matra::domain::{Document, DocumentError};
use matra::{Engine, Inputs};
use std::path::PathBuf;

fn main() -> matra::domain::Result<()> {
    let dir: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./"));

    let engine = Engine::new()?;
    let mut documents: Vec<Document> = Vec::new();
    let mut errors: Vec<DocumentError> = Vec::new();
    for item in engine.analyze_all(Inputs::dir(&dir)?) {
        match item {
            Ok(document) => documents.push(document),
            Err(error) => errors.push(error),
        }
    }

    println!(
        "Analyzed {} documents from {}\n",
        documents.len(),
        dir.display(),
    );

    let words: usize = documents.iter().map(|d| d.metrics.total_words).sum();
    let sentences: usize = documents.iter().map(|d| d.metrics.total_sentences).sum();
    let passive = documents
        .iter()
        .flat_map(Document::sentences)
        .filter(|s| s.passive)
        .count();
    println!("  total words:     {words}");
    println!("  total sentences: {sentences}");
    if sentences > 0 {
        println!(
            "  passive ratio:   {:.1}%",
            passive as f64 / sentences as f64 * 100.0
        );
    }

    // Per-document failures are surfaced, not silenced. DocumentError
    // renders its path when it has one.
    if !errors.is_empty() {
        println!("\nPer-document failures ({}):", errors.len());
        for err in &errors {
            println!("  {err}");
        }
    }

    Ok(())
}
