// cargo add matra serde_json
use matra::extraction::rake_keyphrases;
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
    let phrases = rake_keyphrases(&sentences, 10)?;
    println!("{}", serde_json::to_string_pretty(&phrases)?);
    Ok(())
}
