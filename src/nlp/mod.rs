//! NLP port. Defines the boundary between matra and NLP providers.
//!
//! This module contains ONLY the port trait. Domain types (Token, Sentence)
//! live in domain.rs. Adapters live in submodules behind feature flags.

#[cfg(feature = "udpipe")]
pub mod udpipe;

use crate::domain;

/// Any NLP provider implements this. The domain depends on this trait,
/// not on any specific provider.
///
/// Callers should be aware that `parse` is blocking and may be slow on
/// very large inputs. There is no built-in input size limit or cancellation
/// mechanism. Applications accepting user-supplied text should validate
/// input size before calling parse.
pub trait NlpProvider: Send {
    /// Parse text into sentences with POS tags and dependency labels.
    fn parse(&self, text: &str) -> domain::Result<Vec<domain::Sentence>>;
}
