// cargo add matra serde_json
use matra::extraction::textrank_summarize;
use matra::{Engine, Ingest};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Engine::with_defaults()?;
    let doc = engine
        .analyze(Ingest::path("origin-struggle.txt")?)
        .next()
        .ok_or("no document")??
        .analysis;
    // Parse once; the extractors read the sentences off the tree.
    let sentences: Vec<_> = doc.sentences().cloned().collect();
    let summary = textrank_summarize(&sentences, 3)?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}
