//! NLP port. Defines the boundary between matra and NLP providers.
//!
//! This module contains ONLY the port trait. Domain types (Token, Sentence)
//! live in domain.rs. Adapters live in submodules behind feature flags.

#[cfg(feature = "udpipe")]
pub mod udpipe;

use crate::domain;

/// Any NLP provider implements this. The pipeline depends on this trait,
/// not on any specific provider.
///
/// # Contract
///
/// Every [`domain::Sentence`] a provider returns satisfies the invariants
/// the domain documents on that type, because the tree walks and the
/// metrics read them without checking:
///
/// - `tokens` are in ascending `id` order, numbered `1..=n` with no gaps,
///   where `n` is the token count.
/// - Every `head` is `0` or the `id` of a token in the same sentence.
/// - Exactly one token has `head == 0`: the root.
/// - Following `head` from any token reaches the root; the references
///   form no cycle.
/// - The sentence is built with [`domain::Sentence::new`], which derives
///   the structural fields from `tokens`. `hearst_pairs` is left empty;
///   `Engine::annotate` fills it.
///
/// The trait cannot enforce any of this and nothing validates it on the
/// way in. A provider that breaks the cycle rule is reported rather than
/// trusted where it matters most: [`domain::Sentence::tree_depth`]
/// returns `usize::MAX` on a cycle, and [`domain::Sentence::subtree`]
/// terminates on one.
///
/// A failure is an `Err`, typically [`domain::Error::ParseFailed`], never
/// a panic: a provider wrapping native code converts a panic at its own
/// boundary, as the UDPipe adapter does.
///
/// # Size and blocking
///
/// `parse` is blocking and may be slow on very large inputs. The trait
/// has no input size limit and no cancellation mechanism.
/// `Engine::annotate` applies [`domain::MAX_INPUT_BYTES`] before any text
/// reaches a provider; a caller invoking `parse` directly bypasses that
/// bound and should validate size itself.
pub trait NlpProvider: Send {
    /// Parse text into sentences with POS tags and dependency labels,
    /// satisfying the contract above.
    fn parse(&self, text: &str) -> domain::Result<Vec<domain::Sentence>>;
}
