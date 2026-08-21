//! Corpus analysis with per-document error collection.
//!
//! `Ingest::path` on a directory streams every readable file through the
//! pipeline, and collecting into `CorpusResult` partitions the outcomes:
//!   - a `Corpus` containing entries for every document that analyzed
//!     successfully
//!   - a parallel `Vec<DocumentError>` recording per-document failures
//!
//! Per-file failures never abort the corpus walk. A symlink, an
//! oversized file, or a UDPipe panic on one document leaves the rest
//! of the corpus intact and the failure recorded in the error vector.
//!
//! Run with: cargo run --example corpus -- <directory>
//! (requires UDPipe model at /tmp/matra-models/)

use matra::domain::CorpusResult;
use matra::nlp::udpipe::Udpipe;
use matra::{Engine, Ingest};
use std::path::PathBuf;

fn main() -> matra::domain::Result<()> {
    let dir: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./"));

    let nlp = Udpipe::english("/tmp/matra-models")?;
    let engine = Engine::new(Box::new(nlp), matra::standard_decomposers());
    let result: CorpusResult = engine.analyze(Ingest::path(&dir)?).collect();

    println!(
        "Analyzed {} documents from {}\n",
        result.corpus.entries.len(),
        dir.display(),
    );

    println!("  total words:    {}", result.corpus.total_words(),);
    println!(
        "  passive ratio:  {:.1}%",
        result.corpus.passive_ratio() * 100.0,
    );
    println!(
        "  mean readability: {:.1}",
        result.corpus.mean_readability(),
    );

    // Per-document failures are surfaced, not silenced. DocumentError
    // renders its path when it has one.
    if !result.errors.is_empty() {
        println!("\nPer-document failures ({}):", result.errors.len());
        for err in &result.errors {
            println!("  {err}");
        }
    }

    Ok(())
}
