//! Basic usage: analyze a text string and print metrics.
//!
//! Run with: cargo run --example basic
//! (downloads the UDPipe model into the configured model directory on
//! first use; `matra config show` prints where that is)

use matra::domain::Format;
use matra::{Engine, Input};

fn main() -> matra::domain::Result<()> {
    let engine = Engine::new()?;

    let text = r#"
## The Problem

In 2016 I was part of a team assigned the mandate for bot detection.
We needed a programmable reverse proxy, and we picked Styx: JVM-based,
reactive, open source but built in-house. Envoy had been open sourced
that year, but the enterprise was a Java shop and programmability in
the native language was the key affordance.

## The Cost

The edge gateway we had built had hundreds of internal users and zero
external contributors. Best-in-class had hundreds of external
contributors solving problems before you knew you needed them solved.
"#;

    let analysis = engine.analyze(Input::text(text, Format::Markdown))?;
    report(&analysis);
    Ok(())
}

fn report(analysis: &matra::domain::Document) {
    // A metric slot is `None` when the metric declined to run, which is
    // not the same as a computed zero, so it is printed as such.
    let or_none = |value: Option<f64>, scale: f64, unit: &str| {
        value.map_or_else(
            || "not computed".to_string(),
            |v| format!("{:.2}{unit}", v * scale),
        )
    };

    let metrics = &analysis.metrics;
    println!("Sentences:      {}", metrics.total_sentences);
    println!("Words:          {}", metrics.total_words);
    println!(
        "Passive ratio:  {}",
        or_none(metrics.passive_ratio, 100.0, "%")
    );
    println!(
        "Vocabulary TTR: {}",
        or_none(metrics.vocabulary_ttr, 1.0, "")
    );
    println!(
        "Nominalization: {}",
        or_none(metrics.nominalization_ratio, 100.0, "%")
    );

    println!("\nSections:");
    for section in &analysis.sections {
        let heading = section.heading.as_deref().unwrap_or("(intro)");
        let words: usize = section.paragraphs.iter().map(|p| p.word_count()).sum();
        println!(
            "  {}: {} paragraphs, {} words",
            heading,
            section.paragraphs.len(),
            words
        );
    }
}
