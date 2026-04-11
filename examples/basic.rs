# RECOVERED-FROM-READ source=[claude-project-path]/[session-id]/subagents/[agent-transcript].jsonl timestamp=2026-04-09T13:02:27.722Z original_path=[path]/examples/basic.rs
//! Basic usage: analyze a text string and print metrics.
//!
//! Run with: cargo run --example basic
//! (requires UDPipe model at /tmp/vaani-models/)

use vaani::nlp::udpipe::Udpipe;

fn main() {
    let nlp = Udpipe::english("/tmp/vaani-models")
        .expect("Failed to load English model");

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

    let analysis = vaani::analyze_markdown(text, &nlp).unwrap();

    println!("Sentences:      {}", analysis.total_sentences());
    println!("Words:          {}", analysis.total_words());
    println!("Passive ratio:  {:.1}%", analysis.passive_ratio() * 100.0);
    println!("Vocabulary TTR: {:.2}", analysis.vocabulary_ttr.unwrap_or(0.0));
    println!("Nominalization: {:.1}%", analysis.nominalization_ratio.unwrap_or(0.0) * 100.0);

    println!("\nSections:");
    for section in &analysis.sections {
        let heading = section.heading.as_deref().unwrap_or("(intro)");
        let words: usize = section.paragraphs.iter().map(|p| p.word_count()).sum();
        println!("  {} — {} paragraphs, {} words", heading, section.paragraphs.len(), words);
    }
}

[result-id: r21]