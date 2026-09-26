// cargo add matra serde_json
use matra::{Engine, Ingest};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Loads the English model, downloading it on first use.
    let engine = Engine::with_defaults()?;
    // A file is a stream of one document.
    let entry = engine
        .analyze(Ingest::path("minutes.txt")?)
        .next()
        .ok_or("no document")??;
    println!("{}", serde_json::to_string_pretty(&entry.analysis)?);
    Ok(())
}
