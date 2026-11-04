// cargo add matra@0.2 serde_json
use matra::{Engine, Input};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Loads the English model, downloading it on first use.
    let engine = Engine::new()?;
    let doc = engine.analyze(Input::file("field-notes.md"))?;
    println!("{}", serde_json::to_string_pretty(&doc)?);
    Ok(())
}
