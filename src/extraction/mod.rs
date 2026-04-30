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
