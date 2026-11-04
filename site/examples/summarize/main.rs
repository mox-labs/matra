// cargo add matra@0.3 serde_json
use matra::domain::Format;
use matra::{Engine, Input, SummaryMethod};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Engine::new()?;
    let text = std::fs::read_to_string("origin-struggle.txt")?;
    let doc = engine.analyze(Input::text(text, Format::PlainText))?;
    // Parse once; a summary is a function of the document.
    let summary = matra::summarize_with(&doc, 3, SummaryMethod::TextRank)?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}
