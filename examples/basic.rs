//! Basic usage: analyze a text string and print metrics.
//!
//! Run with: cargo run --example basic
//! (downloads the UDPipe model into the configured model directory on
//! first use; `matra config show` prints where that is)

use matra::domain::Format;
use matra::{Engine, Ingest};

fn main() -> matra::domain::Result<()> {
    let engine = Engine::with_defaults()?;

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

    // A string is a stream of one document.
    for item in engine.analyze(Ingest::text(text, Format::Markdown)) {
        let analysis = item.map_err(|e| e.error)?.analysis;
        report(&analysis);
    }
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

    println!("Sentences:      {}", analysis.total_sentences());
    println!("Words:          {}", analysis.total_words());
    println!("Passive ratio:  {:.1}%", analysis.passive_ratio() * 100.0);
    println!(
        "Vocabulary TTR: {}",
        or_none(analysis.vocabulary_ttr, 1.0, "")
    );
    println!(
        "Nominalization: {}",
        or_none(analysis.nominalization_ratio, 100.0, "%")
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
