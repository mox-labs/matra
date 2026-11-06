//! Embedding port. Defines the boundary between matra and embedding
//! providers.
//!
//! This module contains ONLY the port trait. The [`domain::Embedding`]
//! carrier lives in domain.rs. Adapters live in submodules behind feature
//! flags.
//!
//! Embeddings are Tier 2 output: a model's opinion about meaning, not
//! structure verifiable against the source bytes. RFC-0010 records the
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
/// [`domain::limits::MAX_INPUT_BYTES`], and applications embedding user-supplied
/// text directly should validate size before calling.
///
/// # Errors
///
/// An implementation reports its own failure as one of three kinds, each
/// naming what the caller can do about it, and [`crate::clusters`] passes
/// it through unchanged:
///
/// - [`domain::Error::ModelInvalid`] when its model cannot embed an input
///   (a tokenizer or a lookup that fails). Retrying will not help.
/// - [`domain::Error::Io`] when something it reaches fails: a file, or a
///   remote service. Its `io::ErrorKind` separates a timeout from a
///   refusal, so a caller can tell a retry from a fix.
/// - [`domain::Error::InputTooLarge`] when an input exceeds a cap the
///   implementation holds, with a `what` naming that cap.
///
/// [`domain::Error::InvalidInput`] is not one of them: it is what
/// `clusters` returns when the vectors break the contract above. The one
/// implementation that returns it is the Python binding's bridge to a
/// caller's object, for an object that raised: the bridge cannot know
/// which of the three an arbitrary exception is, and in both cases the
/// embedder the caller passed in is what failed.
pub trait Embedder: Send {
    /// Embed each text into a dense vector.
    fn embed(&self, texts: &[&str]) -> domain::Result<Vec<domain::Embedding>>;

    /// Who produces these vectors, as the parser reports itself
    /// ([`domain::Identity`]): the component's name, the model it runs,
    /// that model's digest and its licence, where known. `clusters`
    /// records it in its result's `parameters.embedder`, so the scores
    /// stay attributable to one geometry and the model's licence travels
    /// with what it produced. Two embedders that can disagree must not
    /// share an identity, so `sha256` covers every artifact that decides
    /// the vectors.
    fn identity(&self) -> domain::Identity;
}
