//! Embedding port. Defines the boundary between matra and embedding
//! providers.
//!
//! This module contains ONLY the port trait. The [`domain::Embedding`]
//! carrier lives in domain.rs. Adapters live in submodules behind feature
//! flags.
//!
//! Embeddings are Tier 2 output: a model's opinion about meaning, not
//! structure verifiable against the source bytes. ADR-0010 records the
//! channel discipline: nothing derived from embeddings becomes a field on
//! the deterministic pipeline's types.

#[cfg(feature = "model2vec")]
pub mod model2vec;

use crate::domain;

/// Any embedding provider implements this. Consumers depend on this trait,
/// not on any specific backend.
///
/// # Contract
///
/// `embed` returns exactly one vector per input text, in input order, and
/// every returned vector has the same dimension. An implementation that
/// cannot honor this for some input returns an error rather than a short
/// or ragged result.
///
/// Callers should be aware that `embed` is blocking and may allocate
/// proportionally to the batch. There is no built-in input size limit;
/// text that has passed through the pipeline is already bounded by
/// [`domain::MAX_INPUT_BYTES`], and applications embedding user-supplied
/// text directly should validate size before calling.
pub trait Embedder: Send {
    /// Embed each text into a dense vector.
    fn embed(&self, texts: &[&str]) -> domain::Result<Vec<domain::Embedding>>;
}
