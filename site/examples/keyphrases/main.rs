// cargo add matra@0.3 serde_json
use matra::domain::Format;
use matra::{Engine, Input, KeyphraseMethod};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Engine::new()?;
    let text = std::fs::read_to_string("origin-struggle.txt")?;
    let doc = engine.analyze(Input::text(text, Format::PlainText))?;
    // Parse once; keyphrases are a function of the document.
    let phrases = matra::keyphrases_with(&doc, 10, KeyphraseMethod::Rake)?;
    println!("{}", serde_json::to_string_pretty(&phrases)?);
    Ok(())
}
