// cargo add matra@0.2 serde_json
use matra::{Engine, Input};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Loads the English model, downloading it on first use.
    let engine = Engine::with_defaults()?;
    let doc = engine.analyze(Input::file("federalist-10.txt"))?;
    println!("{}", serde_json::to_string_pretty(&doc)?);
    Ok(())
}
