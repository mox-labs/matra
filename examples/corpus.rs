//! Corpus analysis with per-document error collection.
//!
//! `analyze_directory` walks a directory of markdown / plain-text files,
//! analyzes each one, and returns:
//!   - a `Corpus` containing entries for every document that analyzed
//!     successfully
//!   - a parallel `Vec<(PathBuf, Error)>` recording per-document failures
//!
//! Per-file failures never abort the corpus walk. A symlink, an
//! oversized file, or a UDPipe panic on one document leaves the rest
//! of the corpus intact and the failure recorded in the error vector.
//!
//! Run with: cargo run --example corpus -- <directory>
//! (requires UDPipe model at /tmp/vaani-models/)

use std::path::PathBuf;
use vaani::nlp::udpipe::Udpipe;

fn main() -> vaani::domain::Result<()> {
    let dir: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./"));

    let nlp = Udpipe::english("/tmp/vaani-models")?;
    let (corpus, errors) = vaani::analyze_directory(&dir, &nlp)?;

    println!(
        "Analyzed {} documents from {}\n",
        corpus.entries.len(),
        dir.display(),
    );

    println!("  total words:    {}", corpus.total_words(),);
    println!("  passive ratio:  {:.1}%", corpus.passive_ratio() * 100.0,);
    println!("  mean readability: {:.1}", corpus.mean_readability(),);

    // Per-document failures are surfaced, not silenced.
    if !errors.is_empty() {
        println!("\nPer-document failures ({}):", errors.len());
        for (path, err) in &errors {
            println!("  {}: {}", path.display(), err);
        }
    }

    Ok(())
}
