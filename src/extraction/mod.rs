//! The outcomes derived from a parsed document: a summary, keyphrases,
//! and the clustering step of `matra::clusters`.
//!
//! The decision this module hides is which method ranks sentences or
//! phrases. The outcome functions here ([`summarize`], [`keyphrases`] and
//! their `_with` forms) are public, re-exported at the crate root; the
//! method modules behind them are private, so a method can change, or a
//! new one arrive, without a published name moving. A caller who needs a
//! particular method names it with [`SummaryMethod`] or
//! [`KeyphraseMethod`], and every result records the method that ran and
//! its parameters in its provenance.
//!
//! Every function here reads only a [`Document`] (boundary rule 5): none
//! needs a parser, so all of them run in a build without UDPipe.
//! Embedding is the composition root's half of `matra::clusters`; the
//! clustering over the vectors is [`cluster`], here.

mod rake;
mod semantic;
mod textrank;
mod tfidf;
mod yake;

use crate::domain::{
    Document, Keyphrase, Keyphrases, Parameters, Result, Sentence, Summary, SummarySentence,
};

pub(crate) use semantic::cluster;
pub(crate) use textrank::MAX_SENTENCES as MAX_CLUSTER_SENTENCES;

/// How [`summarize_with`] ranks a document's sentences.
///
/// The selector is sent, never received: a result names the method that
/// ran as a string in `provenance.method`, which a reader treats as an
/// open set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SummaryMethod {
    /// TF-IDF: each sentence scores the mean TF-IDF of its content
    /// lemmas, each sentence counting as one document for the IDF.
    /// Recorded as `tfidf`.
    TfIdf,
    /// TextRank: PageRank over a graph whose edges are the content lemmas
    /// two sentences share. Recorded as `textrank`.
    TextRank,
}

impl SummaryMethod {
    /// The method's name, as `provenance.method` records it and as the
    /// command line's `--method` and the configuration file spell it.
    pub(crate) fn name(self) -> &'static str {
        match self {
            SummaryMethod::TfIdf => "tfidf",
            SummaryMethod::TextRank => "textrank",
        }
    }
}

/// How [`keyphrases_with`] ranks a document's phrases.
///
/// The selector is sent, never received: a result names the method that
/// ran as a string in `provenance.method`, which a reader treats as an
/// open set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KeyphraseMethod {
    /// RAKE: runs of nouns, adjectives and proper nouns between stop
    /// words, scored by their words' co-occurrence degree over frequency.
    /// Recorded as `rake`.
    Rake,
    /// YAKE: one to three content lemmas, scored by where and how often
    /// their words occur and how varied their neighbours are. Recorded as
    /// `yake`.
    Yake,
}

impl KeyphraseMethod {
    /// The method's name, as `provenance.method` records it and as the
    /// command line's `--method` and the configuration file spell it.
    pub(crate) fn name(self) -> &'static str {
        match self {
            KeyphraseMethod::Rake => "rake",
            KeyphraseMethod::Yake => "yake",
        }
    }
}

/// The method [`summarize`] runs: TF-IDF, the default
/// `config/default.toml` ships as `summarize.algorithm`. Written here as
/// well, because the library reads no configuration; a test holds the two
/// together. Changing it changes what `summarize` returns, which is a
/// breaking change.
pub(crate) const DEFAULT_SUMMARY_METHOD: SummaryMethod = SummaryMethod::TfIdf;

/// The method [`keyphrases`] runs: RAKE, the default
/// `config/default.toml` ships as `keyphrases.algorithm`, held to it by
/// the same test. Changing it is a breaking change.
pub(crate) const DEFAULT_KEYPHRASE_METHOD: KeyphraseMethod = KeyphraseMethod::Rake;

/// The `n` sentences that best represent `doc`, in document order, ranked
/// by the default method (TF-IDF).
///
/// `n` of 0, or a document with no sentences, is an empty summary, not an
/// error. The result's provenance is the document's, with `method` and
/// `parameters.n` recorded.
///
/// ```
/// # fn example(doc: &matra::domain::Document) -> matra::domain::Result<()> {
/// let summary = matra::summarize(doc, 3)?;
/// for sentence in &summary.sentences {
///     println!("{:.3} {}", sentence.score, sentence.text);
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// [`crate::domain::Error::InputTooLarge`] when the document has more
/// sentences than the method's cap (2,000, `what` naming the method).
pub fn summarize(doc: &Document, n: usize) -> Result<Summary> {
    summarize_with(doc, n, DEFAULT_SUMMARY_METHOD)
}

/// [`summarize`], ranked by the method the caller names.
///
/// ```
/// # fn example(doc: &matra::domain::Document) -> matra::domain::Result<()> {
/// let summary = matra::summarize_with(doc, 3, matra::SummaryMethod::TextRank)?;
/// assert_eq!(summary.provenance.method.as_deref(), Some("textrank"));
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// As [`summarize`].
pub fn summarize_with(doc: &Document, n: usize, method: SummaryMethod) -> Result<Summary> {
    let sentences: Vec<&Sentence> = doc.sentences().collect();
    let ranked = match method {
        SummaryMethod::TfIdf => tfidf::rank(&sentences, n)?,
        SummaryMethod::TextRank => textrank::rank(&sentences, n)?,
    };
    Ok(Summary {
        provenance: doc.provenance.derived(method.name(), Parameters::count(n)),
        sentences: ranked
            .into_iter()
            .map(|(index, score)| SummarySentence::new(index, sentences[index], score))
            .collect(),
    })
}

/// At most `n` phrases that best name what `doc` is about, highest score
/// first, ranked by the default method (RAKE), each with every place it
/// occurs.
///
/// `n` of 0, or a document with no content words, is an empty result,
/// not an error. The result's provenance is the document's, with `method`
/// and `parameters.n` recorded.
///
/// ```
/// # fn example(doc: &matra::domain::Document) -> matra::domain::Result<()> {
/// let found = matra::keyphrases(doc, 10)?;
/// for phrase in &found.phrases {
///     println!("{:.3} {} ({} times)", phrase.score, phrase.phrase, phrase.occurrences.len());
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// [`crate::domain::Error::InputTooLarge`] when the document has more
/// tokens than the method's cap (200,000, `what` naming the method).
pub fn keyphrases(doc: &Document, n: usize) -> Result<Keyphrases> {
    keyphrases_with(doc, n, DEFAULT_KEYPHRASE_METHOD)
}

/// [`keyphrases`], ranked by the method the caller names.
///
/// ```
/// # fn example(doc: &matra::domain::Document) -> matra::domain::Result<()> {
/// let found = matra::keyphrases_with(doc, 10, matra::KeyphraseMethod::Yake)?;
/// assert_eq!(found.provenance.method.as_deref(), Some("yake"));
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// As [`keyphrases`].
pub fn keyphrases_with(doc: &Document, n: usize, method: KeyphraseMethod) -> Result<Keyphrases> {
    let sentences: Vec<&Sentence> = doc.sentences().collect();
    let phrases = match method {
        KeyphraseMethod::Rake => rake::keyphrases(&sentences, n)?,
        KeyphraseMethod::Yake => yake::keyphrases(&sentences, n)?,
    };
    Ok(Keyphrases {
        provenance: doc.provenance.derived(method.name(), Parameters::count(n)),
        phrases,
    })
}

/// Rank `candidates` by score, highest first, keeping the order they were
/// inserted in among equal scores, and keep the first `n`.
///
/// Both keyphrase methods collect their candidates in the order each
/// phrase is first met (RAKE in document order, YAKE by window size
/// within each sentence), so a tie resolves the same way on every run,
/// rather than in a hash map's iteration order.
fn rank_phrases(mut candidates: Vec<Keyphrase>, n: usize) -> Vec<Keyphrase> {
    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates.truncate(n);
    candidates
}
