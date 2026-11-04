//! Corpus analysis with per-document error collection.
//!
//! `Inputs::dir` lists every regular file in a directory, `analyze_all`
//! streams them through the pipeline, and collecting into `CorpusResult`
//! partitions the outcomes:
//!   - a `Corpus` holding every document that analyzed successfully, each
//!     naming its path in `provenance.source.path`
//!   - a parallel `Vec<DocumentError>` recording per-document failures
//!
//! Per-file failures never abort the corpus walk. A symlink, an
//! oversized file, or a UDPipe panic on one document leaves the rest
//! of the corpus intact and the failure recorded in the error vector.
//!
//! Run with: cargo run --example corpus -- <directory>
//! (downloads the UDPipe model into the configured model directory on
//! first use; `matra config show` prints where that is)

use matra::domain::CorpusResult;
use matra::{Engine, Inputs};
use std::path::PathBuf;

fn main() -> matra::domain::Result<()> {
    let dir: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./"));

    let engine = Engine::with_defaults()?;
    let result: CorpusResult = engine.analyze_all(Inputs::dir(&dir)?).collect();

    println!(
        "Analyzed {} documents from {}\n",
        result.corpus.documents.len(),
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
