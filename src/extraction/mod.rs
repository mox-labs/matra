# RECOVERED-FROM-READ source=[claude-project-path]/[session-id]/subagents/[agent-transcript].jsonl timestamp=2026-04-09T13:02:36.889Z original_path=[path]/src/extraction/mod.rs
//! Extraction algorithms. Each takes NLP parse output and returns scored results.
//!
//! These are standalone functions, not a pipeline. Consumers call
//! the specific algorithm they need by name.

//! Extraction algorithms. Each takes a slice of Sentences and returns
//! scored results. Consumers call the specific algorithm by name.

mod rake;
mod textrank;
mod tfidf;
mod yake;

pub use rake::keyphrases as rake_keyphrases;
pub use textrank::textrank_summarize;
pub use tfidf::summarize as tfidf_summarize;
pub use yake::yake_keyphrases;

[result-id: r11]