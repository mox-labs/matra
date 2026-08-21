//! Domain types. The core model that everything else depends on.
//! Dependencies are bounded to serde, thiserror, and std.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Resource bounds
// ---------------------------------------------------------------------------

/// Default upper bound on text input to public `analyze*` and `parse` functions.
///
/// 8 MiB accommodates book-length English (a typical novel is ~1.5 MiB / 200k
/// words) with headroom for multilingual prose and structured documents. Beyond
/// this bound, the underlying NLP provider's intermediate memory grows past
/// safe limits on a typical workstation (UDPipe's per-token allocations cross
/// ~1 GiB resident at this input size).
///
/// Public entry points enforce this and return [`Error::InputTooLarge`] with
/// `what = "input"` when exceeded. Adapters may apply tighter bounds for their
/// own constraints (e.g., `FileSource` checks file size before reading).
pub const MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// All errors matra can produce. Matchable, not opaque.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Model file does not exist at the given path.
    #[error("model not found: {}", .0.display())]
    ModelNotFound(PathBuf),
    /// Model file exists but could not be loaded (corrupt, wrong format).
    #[error("invalid model: {0}")]
    ModelInvalid(String),
    /// NLP parsing failed on the input text.
    #[error("parse failed: {0}")]
    ParseFailed(String),
    /// Input exceeded a bounded limit (e.g. too many sentences for
    /// an O(n^2) algorithm like TextRank).
    #[error("{what} input too large: {actual} > limit {limit}")]
    InputTooLarge {
        /// The size cap that was exceeded.
        limit: usize,
        /// The actual size that exceeded the cap.
        actual: usize,
        /// Discriminator naming which gate fired (e.g. `"input"`,
        /// `"file_source"`, `"tfidf"`, `"textrank"`, `"rake"`, `"yake"`).
        /// Lets consumers route differently per gate.
        what: &'static str,
    },
    /// The document format has no registered decomposer in this build.
    /// Seen when analyzing a Pdf/Docx file without the relevant adapter.
    #[error("unsupported format: {0:?}")]
    UnsupportedFormat(Format),
    /// File I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for matra operations.
pub type Result<T> = std::result::Result<T, Error>;

// ---------------------------------------------------------------------------
// Core linguistic types
// ---------------------------------------------------------------------------

/// A parsed token carrying the full CoNLL-U annotation set.
///
/// All ten CoNLL-U columns are preserved from the NLP provider plus
/// one derived convenience field. Downstream algorithms should never
/// need to re-parse to access annotation data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Token {
    /// 1-based position within the sentence (CoNLL-U column 1).
    pub id: usize,
    /// Surface form (CoNLL-U column 2).
    pub text: String,
    /// Dictionary form (CoNLL-U column 3).
    pub lemma: String,
    /// Universal POS tag (CoNLL-U column 4).
    pub pos: String,
    /// Language-specific POS tag (CoNLL-U column 5).
    pub xpos: String,
    /// Morphological features, pipe-separated (CoNLL-U column 6).
    pub feats: String,
    /// Head token id, 0 = root (CoNLL-U column 7).
    pub head: usize,
    /// Dependency relation to head (CoNLL-U column 8).
    pub dep: String,
    /// Enhanced dependency graph (CoNLL-U column 9).
    pub deps: String,
    /// Miscellaneous annotations (CoNLL-U column 10).
    pub misc: String,
    /// Derived: true if this token is punctuation.
    pub is_punct: bool,
}

impl Token {
    /// Preferred construction path for external crates (fields are pub but
    /// `#[non_exhaustive]` prevents struct literal syntax outside the crate).
    pub fn builder(
        id: usize,
        text: String,
        lemma: String,
        pos: String,
        head: usize,
        dep: String,
    ) -> TokenBuilder {
        TokenBuilder {
            id,
            text,
            lemma,
            pos,
            head,
            dep,
            xpos: String::new(),
            feats: String::new(),
            deps: String::new(),
            misc: String::new(),
            is_punct: false,
        }
    }

    /// Look up one morphological feature by exact key in the CoNLL-U
    /// `feats` string (column 6): `feat("Mood")` is `Some("Ind")` when
    /// `feats` is `"Mood=Ind|Tense=Pres"`.
    ///
    /// A linear scan over the pipe-separated pairs, first exact-key
    /// match, no allocation. The value is borrowed raw from `feats`,
    /// so multi-valued features (`Case=Nom,Acc`) come back unsplit:
    /// matra exposes what the provider emitted and does not normalise
    /// it. Both the empty string and the CoNLL-U placeholder `"_"`
    /// contain no `key=value` pair, so every lookup on them returns
    /// `None` by construction.
    ///
    /// Rust-only by design: `feats` already crosses FFI as a string,
    /// so this view adds no information to the wire (ADR-0009).
    pub fn feat(&self, key: &str) -> Option<&str> {
        self.feats
            .split('|')
            .find_map(|pair| match pair.split_once('=') {
                Some((k, v)) if k == key => Some(v),
                _ => None,
            })
    }
}

/// Builder for [`Token`]. Created via [`Token::builder`].
///
/// The six CoNLL-U essentials (id, text, lemma, pos, head, dep) are required.
/// Optional fields default to empty strings; `is_punct` defaults to false.
pub struct TokenBuilder {
    id: usize,
    text: String,
    lemma: String,
    pos: String,
    head: usize,
    dep: String,
    xpos: String,
    feats: String,
    deps: String,
    misc: String,
    is_punct: bool,
}

impl TokenBuilder {
    /// Set the language-specific POS tag (CoNLL-U column 5).
    pub fn xpos(mut self, xpos: String) -> Self {
        self.xpos = xpos;
        self
    }
    /// Set the morphological features string (CoNLL-U column 6).
    pub fn feats(mut self, feats: String) -> Self {
        self.feats = feats;
        self
    }
    /// Set the enhanced dependency graph string (CoNLL-U column 9).
    pub fn deps(mut self, deps: String) -> Self {
        self.deps = deps;
        self
    }
    /// Set the miscellaneous annotations string (CoNLL-U column 10).
    pub fn misc(mut self, misc: String) -> Self {
        self.misc = misc;
        self
    }
    /// Set whether the token is punctuation.
    pub fn is_punct(mut self, is_punct: bool) -> Self {
        self.is_punct = is_punct;
        self
    }
    /// Finalize the builder and return the constructed [`Token`].
    pub fn build(self) -> Token {
        Token {
            id: self.id,
            text: self.text,
            lemma: self.lemma,
            pos: self.pos,
            xpos: self.xpos,
            feats: self.feats,
            head: self.head,
            dep: self.dep,
            deps: self.deps,
            misc: self.misc,
            is_punct: self.is_punct,
        }
    }
}

/// One negation cue in a sentence, referenced by token id.
///
/// Reports structure only: which token carries the cue, its lemma, and
/// the head it attaches to in the dependency graph. What the negation
/// means for the sentence is the consumer's reading, not matra's.
///
/// Derived at [`Sentence`] construction from the dependency graph
/// (see [`Sentence::new`]) and serialized with the sentence, so every
/// crust reads the same detection (ADR-0008).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Negation {
    /// Token id of the cue (`not`, `never`, `no`, `neither`, `nor`).
    pub cue_id: usize,
    /// Lemma of the cue token.
    pub cue_lemma: String,
    /// Token id the cue attaches to (`0` if the cue is the root).
    pub head_id: usize,
}

/// Negation cue detection over a sentence's tokens.
///
/// The single Rust implementation behind [`Sentence::negations`]
/// (ADR-0008). The shapes were verified against live UDPipe parses:
/// `not` and `never` attach as `advmod` to the word they negate,
/// `no` and `neither` attach as `det` to the noun they negate, and
/// `nor` attaches as `cc` to the conjunct it links. Matching on the
/// cue lemma plus the carrying relation keeps precision: `nothing`
/// as a subject (`nsubj`) does not fire, and tokenizer-split `cannot`
/// (`can` + `not`) fires exactly once, on the `not` token.
fn detect_negations(tokens: &[Token]) -> Vec<Negation> {
    const CUE_LEMMAS: [&str; 5] = ["not", "never", "no", "neither", "nor"];
    const CUE_DEPS: [&str; 3] = ["advmod", "det", "cc"];
    tokens
        .iter()
        .filter(|t| CUE_LEMMAS.contains(&t.lemma.as_str()) && CUE_DEPS.contains(&t.dep.as_str()))
        .map(|t| Negation {
            cue_id: t.id,
            cue_lemma: t.lemma.clone(),
            head_id: t.head,
        })
        .collect()
}

/// One parsed sentence: a verbatim text string plus its ordered tokens.
///
/// Invariants downstream code relies on:
/// - `tokens` are id-sorted ascending.
/// - Exactly one token has `head == 0` (the syntactic root).
/// - All `head` references point to another token in the same sentence
///   or to `0`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Sentence {
    /// Verbatim sentence text as produced by the NLP provider.
    pub text: String,
    /// CoNLL-U tokens in id-sorted order.
    pub tokens: Vec<Token>,
    /// Negation cues derived from the dependency graph at construction
    /// (see [`Sentence::new`]). Serialized with the sentence so the
    /// detection crosses FFI as data (ADR-0008). Defaults to empty when
    /// deserializing sentences serialized before this field existed.
    #[serde(default)]
    pub negations: Vec<Negation>,
}

impl Sentence {
    /// Construct a `Sentence` from a text string and a token vector.
    ///
    /// The caller is responsible for upholding the invariants documented
    /// on [`Sentence`]; this constructor does not validate them. Derived
    /// fields (`negations`) are computed here and reflect `tokens` as
    /// passed in; mutating `tokens` afterwards does not recompute them,
    /// and keeping them consistent is part of the same caller contract.
    pub fn new(text: String, tokens: Vec<Token>) -> Self {
        let negations = detect_negations(&tokens);
        Self {
            text,
            tokens,
            negations,
        }
    }

    /// Tokens excluding punctuation.
    pub fn content_tokens(&self) -> Vec<&Token> {
        self.tokens.iter().filter(|t| !t.is_punct).collect()
    }

    /// Count of non-punctuation tokens (NLP token-based).
    ///
    /// This counts linguistic tokens as identified by the NLP provider,
    /// excluding punctuation. It may differ from a naive whitespace split
    /// on the same text. Readability (Flesch-Kincaid) uses whitespace
    /// splitting per the formula's specification.
    pub fn word_count(&self) -> usize {
        self.tokens.iter().filter(|t| !t.is_punct).count()
    }

    /// Whether this sentence contains passive voice constructions.
    pub fn is_passive(&self) -> bool {
        self.tokens
            .iter()
            .any(|t| t.dep == "nsubj:pass" || t.dep == "nsubjpass" || t.dep == "aux:pass")
    }

    /// Maximum depth of the dependency tree (longest path from any token
    /// to root, where root is the token with `head = 0`).
    ///
    /// O(n) per sentence: builds a `HashMap<id, head>` once, then walks
    /// each token's ancestor chain with depth memoization in a second
    /// `HashMap<id, usize>`. Each token's depth is computed at most once;
    /// the visited set inside each walk detects cycles and returns
    /// `usize::MAX` for that token (and any token transitively rooted in
    /// the cycle), surfacing the malformed parse loudly rather than
    /// silently truncating.
    ///
    /// On a malformed parse where some tokens form a cycle, the depth
    /// returned for the sentence is `usize::MAX` (the max over per-token
    /// depths). On a well-formed tree of any depth (no artificial
    /// ceiling), the depth is the true tree depth.
    pub fn tree_depth(&self) -> usize {
        use std::collections::{HashMap, HashSet};

        // Build id → head once. `0` head means root.
        let head_by_id: HashMap<usize, usize> =
            self.tokens.iter().map(|t| (t.id, t.head)).collect();

        // Memoize depth per token id. usize::MAX is the cycle sentinel.
        let mut depth_by_id: HashMap<usize, usize> = HashMap::new();

        // Compute depth for one token. Walks up the head chain, using
        // a visited set to detect cycles in this single chain. On hit,
        // memoizes via the cache to amortize across tokens that share
        // ancestors.
        fn depth_of(
            id: usize,
            head_by_id: &HashMap<usize, usize>,
            depth_by_id: &mut HashMap<usize, usize>,
        ) -> usize {
            if let Some(&d) = depth_by_id.get(&id) {
                return d;
            }
            let mut visited: HashSet<usize> = HashSet::new();
            let mut chain: Vec<usize> = Vec::new();
            let mut cur = id;

            // Walk to root or to a memoized ancestor or to a cycle.
            let base_depth = loop {
                if !visited.insert(cur) {
                    // Cycle detected — every token in the chain is malformed.
                    for &cid in &chain {
                        depth_by_id.insert(cid, usize::MAX);
                    }
                    return usize::MAX;
                }
                let head = head_by_id.get(&cur).copied().unwrap_or(0);
                if head == 0 {
                    // cur is root or its head is missing: depth contribution from this point is 0.
                    chain.push(cur);
                    break 0usize;
                }
                if let Some(&d) = depth_by_id.get(&head) {
                    chain.push(cur);
                    break d.saturating_add(1);
                }
                chain.push(cur);
                cur = head;
            };

            // Walk back down the chain, assigning depths in order.
            // chain[last] is the closest-to-root token; chain[0] is the
            // original `id`. Depth of chain[last] = base_depth; each step
            // away from root adds 1.
            let n = chain.len();
            for (i, &cid) in chain.iter().enumerate() {
                let d = base_depth.saturating_add(n - 1 - i);
                depth_by_id.insert(cid, d);
            }
            depth_by_id[&id]
        }

        self.tokens
            .iter()
            .map(|t| depth_of(t.id, &head_by_id, &mut depth_by_id))
            .max()
            .unwrap_or(0)
    }

    /// The root token (head = 0). Returns None if no root found.
    pub fn root_token(&self) -> Option<&Token> {
        self.tokens.iter().find(|t| t.head == 0)
    }

    /// Direct children of the token with the given id.
    pub fn children_of(&self, id: usize) -> Vec<&Token> {
        self.tokens.iter().filter(|t| t.head == id).collect()
    }

    /// The head token of the token with the given id.
    pub fn head_of(&self, id: usize) -> Option<&Token> {
        self.tokens.iter().find(|t| t.id == id).and_then(|t| {
            if t.head == 0 {
                None
            } else {
                self.tokens.iter().find(|h| h.id == t.head)
            }
        })
    }

    /// All tokens in the subtree rooted at the given id (including the root).
    /// Returns tokens sorted by id (document order).
    /// Safe on cyclic graphs: visited set prevents infinite loops.
    pub fn subtree(&self, id: usize) -> Vec<&Token> {
        let mut result = Vec::new();
        let mut stack = vec![id];
        let mut visited = std::collections::HashSet::new();
        while let Some(current) = stack.pop() {
            if !visited.insert(current) {
                continue;
            }
            if let Some(token) = self.tokens.iter().find(|t| t.id == current) {
                result.push(token);
                for child in &self.tokens {
                    if child.head == current && child.id != current {
                        stack.push(child.id);
                    }
                }
            }
        }
        result.sort_by_key(|t| t.id);
        result
    }
}

// ---------------------------------------------------------------------------
// Document output -- what encoders produce
// ---------------------------------------------------------------------------

/// One paragraph of prose with metric slots filled in during the
/// pipeline's `measure` stage.
///
/// `in_blockquote = true` paragraphs are skipped during metric
/// computation; their `Option<f64>` slots stay `None`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Paragraph {
    /// Verbatim paragraph text.
    pub text: String,
    /// Whether the paragraph is inside a blockquote (skipped by metrics).
    ///
    /// **Deprecation notice (v0.2):** this boolean field is planned to be
    /// replaced with `kind: ParagraphKind` once the variant inventory
    /// (Body / Quote / Code / List / Caption) is justified by real
    /// consumer semantics. The boolean stays in the 0.0.x and 0.1.x lines
    /// because its job (gate measure or not) is binary today. See
    /// [ADR-0006](https://github.com/mox-labs/matra/blob/main/docs/decisions/0006-abstract-tier-vocabulary-lock.md)
    /// for the abstract-tier vocabulary lock.
    pub in_blockquote: bool,
    /// Sentences produced by parsing this paragraph (populated by the
    /// pipeline's `parse` stage).
    pub sentences: Vec<Sentence>,
    /// Flesch-Kincaid grade level, if `measure` ran on this paragraph.
    pub readability_grade: Option<f64>,
    /// Content-word ratio, if `measure` ran on this paragraph.
    pub lexical_density: Option<f64>,
    /// Brotli compression ratio (a redundancy proxy), if `measure` ran.
    pub compression_ratio: Option<f64>,
}

impl Paragraph {
    /// Construct a new `Paragraph` with empty sentences and `None`
    /// metric slots. The pipeline's `parse` and `measure` stages fill
    /// the slots in.
    pub fn new(text: String, in_blockquote: bool) -> Self {
        Self {
            text,
            in_blockquote,
            sentences: Vec::new(),
            readability_grade: None,
            lexical_density: None,
            compression_ratio: None,
        }
    }

    /// Total word count across all sentences.
    pub fn word_count(&self) -> usize {
        self.sentences.iter().map(|s| s.word_count()).sum()
    }

    /// Number of sentences.
    pub fn sentence_count(&self) -> usize {
        self.sentences.len()
    }
}

/// A structural section of a document (heading + paragraphs).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Section {
    /// Section heading, if any. `None` for the intro section of a
    /// markdown document with no leading heading, and for plain-text
    /// decomposition (which produces one heading-less section).
    pub heading: Option<String>,
    /// Heading depth (0 for plain text, 1+ for markdown `#`/`##`/etc.).
    pub level: usize,
    /// Paragraphs in document order.
    pub paragraphs: Vec<Paragraph>,
}

impl Section {
    /// Construct a new section with the given heading, level, and paragraphs.
    pub fn new(heading: Option<String>, level: usize, paragraphs: Vec<Paragraph>) -> Self {
        Self {
            heading,
            level,
            paragraphs,
        }
    }
}

/// The full analysis output. Encoders populate this from NLP parse results.
///
/// Paragraphs live in sections (single source of truth). Derived methods
/// compute document-level metrics on the fly from the section tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Document {
    /// Section tree (the single source of truth for paragraph ownership).
    pub sections: Vec<Section>,
    /// Document-level vocabulary type-token ratio, if `measure` ran.
    pub vocabulary_ttr: Option<f64>,
    /// Document-level nominalization ratio, if `measure` ran.
    pub nominalization_ratio: Option<f64>,
    /// Fraction of sentences containing a passive-voice construction,
    /// if `measure` ran. Materialized so the aggregate crosses FFI as
    /// data instead of being re-derived per crust (ADR-0008). Defaults
    /// to `None` when deserializing documents serialized before this
    /// field existed.
    #[serde(default)]
    pub passive_ratio: Option<f64>,
}

impl Document {
    /// Construct a new `Document` from a section tree with `None` for
    /// the document-level metric slots; `measure` fills them in.
    pub fn new(sections: Vec<Section>) -> Self {
        Self {
            sections,
            vocabulary_ttr: None,
            nominalization_ratio: None,
            passive_ratio: None,
        }
    }

    /// Flat iterator over all paragraphs.
    pub fn paragraphs(&self) -> impl Iterator<Item = &Paragraph> {
        self.sections.iter().flat_map(|s| s.paragraphs.iter())
    }

    /// Flat mutable iterator over all paragraphs.
    pub fn paragraphs_mut(&mut self) -> impl Iterator<Item = &mut Paragraph> {
        self.sections
            .iter_mut()
            .flat_map(|s| s.paragraphs.iter_mut())
    }

    /// Count of all paragraphs.
    pub fn paragraph_count(&self) -> usize {
        self.sections.iter().map(|s| s.paragraphs.len()).sum()
    }

    /// Flat iterator over all sentences across all paragraphs.
    pub fn sentences(&self) -> impl Iterator<Item = &Sentence> {
        self.sections
            .iter()
            .flat_map(|s| s.paragraphs.iter())
            .flat_map(|p| p.sentences.iter())
    }

    /// Flat iterator over all tokens across all sentences.
    pub fn tokens(&self) -> impl Iterator<Item = &Token> {
        self.sections
            .iter()
            .flat_map(|s| s.paragraphs.iter())
            .flat_map(|p| p.sentences.iter())
            .flat_map(|s| s.tokens.iter())
    }

    /// Total sentence count across all paragraphs.
    pub fn total_sentences(&self) -> usize {
        self.paragraphs().map(|p| p.sentence_count()).sum()
    }

    /// Total non-punctuation token count across all sentences.
    pub fn total_words(&self) -> usize {
        self.sentences().map(|s| s.word_count()).sum()
    }

    /// Fraction of sentences containing a passive-voice construction.
    /// Returns `0.0` when there are no sentences.
    ///
    /// This is the computation behind the [`Document::passive_ratio`]
    /// field, which the metric suite fills so the value crosses FFI
    /// (ADR-0008). The method stays for Rust callers who want the
    /// ratio on an unmeasured document.
    pub fn passive_ratio(&self) -> f64 {
        let total = self.total_sentences();
        if total == 0 {
            return 0.0;
        }
        self.sentences().filter(|s| s.is_passive()).count() as f64 / total as f64
    }

    /// Mean sentence length in words. Returns `0.0` when there are no
    /// sentences.
    pub fn mean_sentence_length(&self) -> f64 {
        let total = self.total_sentences();
        if total == 0 {
            return 0.0;
        }
        self.total_words() as f64 / total as f64
    }

    /// Sample standard deviation of sentence length in words. Returns
    /// `0.0` when there is fewer than two sentences (no variance defined).
    pub fn sentence_length_std(&self) -> f64 {
        let lengths: Vec<f64> = self.sentences().map(|s| s.word_count() as f64).collect();
        if lengths.len() <= 1 {
            return 0.0;
        }
        let mean = lengths.iter().sum::<f64>() / lengths.len() as f64;
        let var =
            lengths.iter().map(|l| (l - mean).powi(2)).sum::<f64>() / (lengths.len() - 1) as f64;
        var.sqrt()
    }
}

// ---------------------------------------------------------------------------
// Extraction output — what extraction algorithms produce
// ---------------------------------------------------------------------------

/// A sentence ranked by relevance score with its original document position.
///
/// Output of [`extraction::tfidf_summarize`](crate::extraction::tfidf_summarize)
/// and [`extraction::textrank_summarize`](crate::extraction::textrank_summarize).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ScoredSentence {
    /// Verbatim sentence text.
    pub text: String,
    /// Relevance score, higher is more relevant.
    pub score: f64,
    /// Original document position (sentence index), preserved so
    /// consumers can re-anchor scored sentences in document order.
    pub position: usize,
}

impl ScoredSentence {
    /// Construct a new `ScoredSentence`.
    pub fn new(text: String, score: f64, position: usize) -> Self {
        Self {
            text,
            score,
            position,
        }
    }
}

/// A ranked keyphrase extracted from text.
///
/// Output of [`extraction::rake_keyphrases`](crate::extraction::rake_keyphrases)
/// and [`extraction::yake_keyphrases`](crate::extraction::yake_keyphrases).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Keyphrase {
    /// The keyphrase text.
    pub phrase: String,
    /// Relevance score, higher is more relevant.
    pub score: f64,
}

impl Keyphrase {
    /// Construct a new `Keyphrase`.
    pub fn new(phrase: String, score: f64) -> Self {
        Self { phrase, score }
    }
}

// ---------------------------------------------------------------------------
// Source / corpus types
// ---------------------------------------------------------------------------

/// Document format, detected from file extension.
///
/// `Pdf` and `Docx` are reserved variants; the library returns
/// [`Error::UnsupportedFormat`] for those today.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Format {
    /// Markdown source.
    Markdown,
    /// Plain text.
    PlainText,
    /// PDF (reserved; no decomposer ships today).
    Pdf,
    /// DOCX (reserved; no decomposer ships today).
    Docx,
}

/// A raw document before decomposition. Output of Source, input to Decomposer.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RawDocument {
    /// The document text.
    pub text: String,
    /// Source path, if the document came from disk. `None` for
    /// in-memory text.
    pub path: Option<PathBuf>,
    /// Detected format.
    pub format: Format,
}

impl RawDocument {
    /// Construct a new `RawDocument`.
    pub fn new(text: String, path: Option<PathBuf>, format: Format) -> Self {
        Self { text, path, format }
    }
}

/// A single entry in a corpus: one analyzed document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CorpusEntry {
    /// Source path, if the document came from disk.
    pub path: Option<PathBuf>,
    /// The document's analysis output.
    pub analysis: Document,
}

impl CorpusEntry {
    /// Construct a new `CorpusEntry`.
    pub fn new(path: Option<PathBuf>, analysis: Document) -> Self {
        Self { path, analysis }
    }
}

/// A collection of analyzed documents.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Corpus {
    /// One entry per successfully analyzed document.
    pub entries: Vec<CorpusEntry>,
}

impl Corpus {
    /// Construct a new `Corpus` from a vector of entries.
    pub fn new(entries: Vec<CorpusEntry>) -> Self {
        Self { entries }
    }

    /// Total non-punctuation token count across every entry's analysis.
    pub fn total_words(&self) -> usize {
        self.entries.iter().map(|e| e.analysis.total_words()).sum()
    }

    /// Fraction of sentences across all entries containing a passive-
    /// voice construction. Returns `0.0` when the corpus has no
    /// sentences.
    pub fn passive_ratio(&self) -> f64 {
        let total: usize = self
            .entries
            .iter()
            .map(|e| e.analysis.total_sentences())
            .sum();
        if total == 0 {
            return 0.0;
        }
        let passive: usize = self
            .entries
            .iter()
            .map(|e| e.analysis.sentences().filter(|s| s.is_passive()).count())
            .sum();
        passive as f64 / total as f64
    }

    /// Mean readability grade across all paragraphs that carry a
    /// `readability_grade` value. Returns `0.0` when no paragraphs have
    /// been measured.
    pub fn mean_readability(&self) -> f64 {
        let grades: Vec<f64> = self
            .entries
            .iter()
            .flat_map(|e| e.analysis.paragraphs())
            .filter_map(|p| p.readability_grade)
            .collect();
        if grades.is_empty() {
            return 0.0;
        }
        grades.iter().sum::<f64>() / grades.len() as f64
    }
}

/// A per-document failure: the error plus the path it occurred at, if
/// the document came from disk.
///
/// `path` is `Option` because in-memory text has no path. This closes a
/// real hole: the previous corpus surface fabricated an empty `PathBuf`
/// for a path-less document, which collided with a genuinely empty path.
#[derive(Debug)]
#[non_exhaustive]
pub struct DocumentError {
    /// Source path, if the document came from disk.
    pub path: Option<PathBuf>,
    /// What went wrong.
    pub error: Error,
}

impl DocumentError {
    /// Construct a new `DocumentError`.
    pub fn new(path: Option<PathBuf>, error: Error) -> Self {
        Self { path, error }
    }
}

impl std::fmt::Display for DocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.path {
            Some(p) => write!(f, "{}: {}", p.display(), self.error),
            None => write!(f, "{}", self.error),
        }
    }
}

impl std::error::Error for DocumentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

/// The outcome of analyzing a stream of documents: every success in a
/// [`Corpus`], every per-document failure alongside it.
///
/// The partition invariant: entries plus errors equals documents
/// consumed. Nothing is silently dropped.
///
/// Not `Serialize`: [`DocumentError`] wraps [`Error`], which wraps
/// `std::io::Error`. Crossing a language boundary needs a projection
/// with stable kind strings, not a serialization of `io::Error`.
#[derive(Debug)]
#[non_exhaustive]
pub struct CorpusResult {
    /// Every successfully analyzed document.
    pub corpus: Corpus,
    /// Every per-document failure, in consumption order.
    pub errors: Vec<DocumentError>,
}

impl CorpusResult {
    /// Construct a new `CorpusResult`.
    pub fn new(corpus: Corpus, errors: Vec<DocumentError>) -> Self {
        Self { corpus, errors }
    }
}

/// `collect()` is the corpus constructor: a stream of per-document
/// results partitions into successes and failures, preserving order
/// within each.
impl FromIterator<std::result::Result<CorpusEntry, DocumentError>> for CorpusResult {
    fn from_iter<I: IntoIterator<Item = std::result::Result<CorpusEntry, DocumentError>>>(
        iter: I,
    ) -> Self {
        let mut entries = Vec::new();
        let mut errors = Vec::new();
        for item in iter {
            match item {
                Ok(entry) => entries.push(entry),
                Err(err) => errors.push(err),
            }
        }
        CorpusResult::new(Corpus::new(entries), errors)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_token(id: usize, text: &str, pos: &str, dep: &str, head: usize) -> Token {
        Token {
            id,
            text: text.to_string(),
            lemma: text.to_lowercase(),
            pos: pos.to_string(),
            xpos: String::new(),
            feats: String::new(),
            head,
            dep: dep.to_string(),
            deps: String::new(),
            misc: String::new(),
            is_punct: pos == "PUNCT",
        }
    }

    fn passive_sentence() -> Sentence {
        Sentence::new(
            "The system was built by the team".to_string(),
            vec![
                make_token(1, "The", "DET", "det", 2),
                make_token(2, "system", "NOUN", "nsubj:pass", 4),
                make_token(3, "was", "AUX", "aux:pass", 4),
                make_token(4, "built", "VERB", "root", 0),
                make_token(5, "by", "ADP", "case", 7),
                make_token(6, "the", "DET", "det", 7),
                make_token(7, "team", "NOUN", "obl", 4),
            ],
        )
    }

    fn active_sentence() -> Sentence {
        Sentence::new(
            "The team shipped the product".to_string(),
            vec![
                make_token(1, "The", "DET", "det", 2),
                make_token(2, "team", "NOUN", "nsubj", 3),
                make_token(3, "shipped", "VERB", "root", 0),
                make_token(4, "the", "DET", "det", 5),
                make_token(5, "product", "NOUN", "obj", 3),
            ],
        )
    }

    #[test]
    fn content_tokens_excludes_punct() {
        let sent = Sentence::new(
            "Hello, world!".to_string(),
            vec![
                make_token(1, "Hello", "INTJ", "root", 0),
                make_token(2, ",", "PUNCT", "punct", 1),
                make_token(3, "world", "NOUN", "vocative", 1),
                make_token(4, "!", "PUNCT", "punct", 1),
            ],
        );
        assert_eq!(sent.content_tokens().len(), 2);
        assert_eq!(sent.word_count(), 2);
    }

    #[test]
    fn is_passive_detects_passive() {
        assert!(passive_sentence().is_passive());
        assert!(!active_sentence().is_passive());
    }

    fn token_with_feats(feats: &str) -> Token {
        Token::builder(
            1,
            "x".to_string(),
            "x".to_string(),
            "NOUN".to_string(),
            0,
            "root".to_string(),
        )
        .feats(feats.to_string())
        .build()
    }

    /// Feats strings harvested verbatim from a live parse with the
    /// English UDPipe model (2026-08-21), plus the empty string the
    /// udpipe adapter stores for feature-less tokens.
    const HARVESTED_FEATS: &[&str] = &[
        "",
        "Case=Nom|Gender=Neut|Number=Sing|Person=3|PronType=Prs",
        "Mood=Ind|Number=Sing|Person=3|Tense=Past|VerbForm=Fin",
        "Tense=Past|VerbForm=Part|Voice=Pass",
        "Definite=Def|PronType=Art",
        "Number=Sing",
        "Degree=Pos",
        "Mood=Ind|Tense=Past|VerbForm=Fin",
        "VerbForm=Fin",
        "Case=Nom|Number=Plur|Person=1|PronType=Prs",
        "VerbForm=Inf",
        "Number=Sing|PronType=Dem",
        "Mood=Ind|Number=Sing|Person=3|Tense=Pres|VerbForm=Fin",
        "Number=Plur",
    ];

    #[test]
    fn feat_round_trips_every_harvested_pair() {
        // Property over the harvested corpus: every key=value pair a
        // feats string carries is returned exactly by feat(key), and
        // keys the string does not carry return None.
        for feats in HARVESTED_FEATS {
            let tok = token_with_feats(feats);
            for pair in feats.split('|').filter(|p| !p.is_empty()) {
                let (k, v) = pair.split_once('=').expect("harvested pair has =");
                assert_eq!(tok.feat(k), Some(v), "feats {feats:?} key {k:?}");
            }
            assert_eq!(tok.feat("Absent"), None, "feats {feats:?}");
            assert_eq!(tok.feat(""), None, "feats {feats:?}");
        }
    }

    #[test]
    fn feat_none_on_empty_and_underscore_placeholder() {
        // The udpipe adapter stores "" for feature-less tokens
        // (verified against a live parse); "_" is the CoNLL-U
        // placeholder seen when reading CoNLL-U from disk. Both must
        // yield None for every key.
        for feats in ["", "_"] {
            let tok = token_with_feats(feats);
            for key in ["Mood", "VerbForm", "_", ""] {
                assert_eq!(tok.feat(key), None, "feats {feats:?} key {key:?}");
            }
        }
    }

    #[test]
    fn feat_single_feature() {
        let tok = token_with_feats("Number=Sing");
        assert_eq!(tok.feat("Number"), Some("Sing"));
        assert_eq!(tok.feat("number"), None, "keys match exactly");
        assert_eq!(tok.feat("Num"), None, "no prefix match");
    }

    #[test]
    fn feat_multi_value_returned_unsplit() {
        let tok = token_with_feats("Case=Nom,Acc|Number=Sing");
        assert_eq!(tok.feat("Case"), Some("Nom,Acc"));
    }

    #[test]
    fn feat_malformed_input_does_not_panic() {
        // Segments without '=' are never matched; a second '=' stays
        // in the value; duplicate keys resolve to the first match.
        let tok = token_with_feats("junk|Mood=Ind=extra|Mood=Sub|=orphan|Tense=");
        assert_eq!(tok.feat("junk"), None);
        assert_eq!(tok.feat("Mood"), Some("Ind=extra"));
        assert_eq!(tok.feat("Tense"), Some(""));
        assert_eq!(tok.feat("orphan"), None);
    }

    // Negation fixtures mirror live UDPipe parses (verified 2026-08-21):
    // `not` is PART/advmod, `never` is ADV/advmod, `no` and `neither`
    // are DET/det, `nor` is CCONJ/cc.

    #[test]
    fn negation_detects_not_as_advmod() {
        // "The plan is not ready."
        let sent = Sentence::new(
            "The plan is not ready.".to_string(),
            vec![
                make_token(1, "The", "DET", "det", 2),
                make_token(2, "plan", "NOUN", "nsubj", 5),
                make_token(3, "is", "AUX", "cop", 5),
                make_token(4, "not", "PART", "advmod", 5),
                make_token(5, "ready", "ADJ", "root", 0),
                make_token(6, ".", "PUNCT", "punct", 5),
            ],
        );
        assert_eq!(sent.negations.len(), 1);
        assert_eq!(sent.negations[0].cue_id, 4);
        assert_eq!(sent.negations[0].cue_lemma, "not");
        assert_eq!(sent.negations[0].head_id, 5);
    }

    #[test]
    fn negation_detects_never_as_advmod() {
        // "It was never reviewed."
        let sent = Sentence::new(
            "It was never reviewed.".to_string(),
            vec![
                make_token(1, "It", "PRON", "nsubj:pass", 4),
                make_token(2, "was", "AUX", "aux:pass", 4),
                make_token(3, "never", "ADV", "advmod", 4),
                make_token(4, "reviewed", "VERB", "root", 0),
                make_token(5, ".", "PUNCT", "punct", 4),
            ],
        );
        assert_eq!(sent.negations.len(), 1);
        assert_eq!(sent.negations[0].cue_lemma, "never");
        assert_eq!(sent.negations[0].cue_id, 3);
        assert_eq!(sent.negations[0].head_id, 4);
    }

    #[test]
    fn negation_detects_no_as_det() {
        // "No changes were made."
        let sent = Sentence::new(
            "No changes were made.".to_string(),
            vec![
                make_token(1, "No", "DET", "det", 2),
                make_token(2, "changes", "NOUN", "nsubj:pass", 4),
                make_token(3, "were", "AUX", "aux:pass", 4),
                make_token(4, "made", "VERB", "root", 0),
                make_token(5, ".", "PUNCT", "punct", 4),
            ],
        );
        assert_eq!(sent.negations.len(), 1);
        assert_eq!(sent.negations[0].cue_lemma, "no");
        assert_eq!(sent.negations[0].head_id, 2);
    }

    #[test]
    fn negation_detects_neither_and_nor() {
        // "Neither option worked, nor did the fallback."
        let sent = Sentence::new(
            "Neither option worked, nor did the fallback.".to_string(),
            vec![
                make_token(1, "Neither", "DET", "det", 2),
                make_token(2, "option", "NOUN", "nsubj", 3),
                make_token(3, "worked", "VERB", "root", 0),
                make_token(4, ",", "PUNCT", "punct", 6),
                make_token(5, "nor", "CCONJ", "cc", 6),
                make_token(6, "did", "VERB", "conj", 3),
                make_token(7, "the", "DET", "det", 8),
                make_token(8, "fallback", "NOUN", "obj", 6),
                make_token(9, ".", "PUNCT", "punct", 3),
            ],
        );
        let lemmas: Vec<&str> = sent
            .negations
            .iter()
            .map(|n| n.cue_lemma.as_str())
            .collect();
        assert_eq!(lemmas, vec!["neither", "nor"]);
        assert_eq!(sent.negations[0].head_id, 2);
        assert_eq!(sent.negations[1].head_id, 6);
    }

    #[test]
    fn negation_no_false_positive_on_nothing_as_subject() {
        // "Nothing happened." UDPipe lemmatizes `Nothing` to `nothing`
        // (PRON, nsubj), which is not a cue: the indefinite carries the
        // negation lexically, not as a dependency-graph cue token.
        let sent = Sentence::new(
            "Nothing happened.".to_string(),
            vec![
                make_token(1, "Nothing", "PRON", "nsubj", 2),
                make_token(2, "happened", "VERB", "root", 0),
                make_token(3, ".", "PUNCT", "punct", 2),
            ],
        );
        assert!(sent.negations.is_empty());
    }

    #[test]
    fn negation_on_tokenizer_split_cannot_fires_once_on_not() {
        // "We cannot merge this." The tokenizer splits `cannot` into
        // `can` + `not` (verified against a live parse); exactly one
        // cue must fire, on the `not` token, not on `can` and not twice.
        let sent = Sentence::new(
            "We cannot merge this.".to_string(),
            vec![
                make_token(1, "We", "PRON", "nsubj", 4),
                make_token(2, "can", "AUX", "aux", 4),
                make_token(3, "not", "PART", "advmod", 4),
                make_token(4, "merge", "VERB", "root", 0),
                make_token(5, "this", "PRON", "obj", 4),
                make_token(6, ".", "PUNCT", "punct", 4),
            ],
        );
        assert_eq!(sent.negations.len(), 1);
        assert_eq!(sent.negations[0].cue_id, 3);
        assert_eq!(sent.negations[0].cue_lemma, "not");
        assert_eq!(sent.negations[0].head_id, 4);
    }

    #[test]
    fn negation_empty_on_unnegated_sentences() {
        assert!(active_sentence().negations.is_empty());
        assert!(passive_sentence().negations.is_empty());
    }

    #[test]
    fn negations_serialize_and_default_on_old_json() {
        // The field crosses the wire (ADR-0008) ...
        let sent = Sentence::new(
            "It was never reviewed.".to_string(),
            vec![make_token(1, "never", "ADV", "advmod", 0)],
        );
        let json = serde_json::to_value(&sent).unwrap();
        assert_eq!(json["negations"][0]["cue_lemma"], "never");
        // ... and JSON serialized before the field existed still
        // deserializes, with an empty default.
        let old = r#"{"text":"old","tokens":[]}"#;
        let sent: Sentence = serde_json::from_str(old).unwrap();
        assert!(sent.negations.is_empty());
    }

    #[test]
    fn tree_depth_computes_max() {
        let sent = active_sentence();
        assert!(sent.tree_depth() > 0);
    }

    #[test]
    fn root_token_finds_root() {
        let sent = passive_sentence();
        let root = sent.root_token().unwrap();
        assert_eq!(root.text, "built");
    }

    #[test]
    fn children_of_returns_direct_children() {
        let sent = passive_sentence();
        let children = sent.children_of(4);
        let texts: Vec<&str> = children.iter().map(|t| t.text.as_str()).collect();
        assert!(texts.contains(&"system"));
        assert!(texts.contains(&"was"));
        assert!(texts.contains(&"team"));
    }

    #[test]
    fn head_of_returns_head() {
        let sent = passive_sentence();
        let head = sent.head_of(2).unwrap();
        assert_eq!(head.text, "built");
    }

    #[test]
    fn head_of_root_returns_none() {
        let sent = passive_sentence();
        assert!(sent.head_of(4).is_none());
    }

    #[test]
    fn subtree_includes_all_descendants() {
        let sent = passive_sentence();
        let sub = sent.subtree(4);
        assert_eq!(sub.len(), sent.tokens.len());
    }

    #[test]
    fn subtree_leaf_returns_single() {
        let sent = passive_sentence();
        let sub = sent.subtree(1);
        assert_eq!(sub.len(), 1);
        assert_eq!(sub[0].text, "The");
    }

    #[test]
    fn invalid_id_returns_empty() {
        let sent = passive_sentence();
        assert!(sent.children_of(99).is_empty());
        assert!(sent.head_of(99).is_none());
        assert!(sent.subtree(99).is_empty());
    }

    #[test]
    fn tree_depth_with_non_contiguous_ids() {
        // Token IDs 10, 20, 30 -- not contiguous, not 1-based sequential.
        // tree: 30 (root) -> 20 -> 10
        let sent = Sentence::new(
            "non contiguous ids".to_string(),
            vec![
                make_token(10, "A", "NOUN", "dep", 20),
                make_token(20, "B", "NOUN", "dep", 30),
                make_token(30, "C", "VERB", "root", 0),
            ],
        );
        // Depth from A: A->20(B)->30(C)->root = 2
        // Depth from B: B->30(C)->root = 1
        // Depth from C: root = 0
        assert_eq!(sent.tree_depth(), 2);
        assert_eq!(sent.root_token().unwrap().text, "C");
        assert_eq!(sent.children_of(30).len(), 1);
        assert_eq!(sent.head_of(10).unwrap().text, "B");
    }

    #[test]
    fn subtree_safe_on_cyclic_graph() {
        // A -> B -> A (cycle of length 2). Must not infinite-loop.
        let sent = Sentence::new(
            "cyclic".to_string(),
            vec![
                make_token(1, "A", "NOUN", "dep", 2),
                make_token(2, "B", "NOUN", "dep", 1),
            ],
        );
        let sub = sent.subtree(1);
        assert_eq!(sub.len(), 2);
    }

    #[test]
    fn empty_sentence_is_safe() {
        let sent = Sentence::new(String::new(), vec![]);
        assert_eq!(sent.word_count(), 0);
        assert!(!sent.is_passive());
        assert_eq!(sent.tree_depth(), 0);
        assert!(sent.root_token().is_none());
        assert!(sent.children_of(1).is_empty());
        assert!(sent.head_of(1).is_none());
        assert!(sent.subtree(1).is_empty());
    }

    /// Build a straight head chain: token 1 is root (head=0), token 2's
    /// head is 1, ..., token n's head is n-1. Depth from token n to root
    /// is n-1.
    fn straight_chain(n: usize) -> Sentence {
        let tokens: Vec<Token> = (1..=n)
            .map(|i| make_token(i, "x", "NOUN", "dep", if i == 1 { 0 } else { i - 1 }))
            .collect();
        Sentence::new("chain".to_string(), tokens)
    }

    #[test]
    fn tree_depth_25_chain_returns_24() {
        // Verifies the previously-magic-< 20 ceiling is gone: a 25-token
        // chain has true depth 24 (token 25 is 24 hops from root).
        let sent = straight_chain(25);
        assert_eq!(sent.tree_depth(), 24);
    }

    #[test]
    fn tree_depth_1000_chain_returns_999_in_bounded_time() {
        // Verifies O(n) complexity: a 1000-token chain returns 999, fast.
        // The pre-fix O(n^2) impl would do ~10^6 inner finds; this is well
        // under 50ms on commodity hardware.
        let sent = straight_chain(1000);
        let start = std::time::Instant::now();
        let depth = sent.tree_depth();
        let elapsed = start.elapsed();
        assert_eq!(depth, 999);
        assert!(
            elapsed < std::time::Duration::from_millis(50),
            "tree_depth on 1000-chain took {elapsed:?} (expected < 50ms; suggests non-linear complexity)"
        );
    }

    #[test]
    fn corpus_result_partition_holds() {
        // |entries| + |errors| = items consumed, order preserved per side.
        let items: Vec<std::result::Result<CorpusEntry, DocumentError>> = vec![
            Ok(CorpusEntry::new(
                Some(PathBuf::from("a.md")),
                Document::new(Vec::new()),
            )),
            Err(DocumentError::new(
                Some(PathBuf::from("b.md")),
                Error::ParseFailed("bad".to_string()),
            )),
            Ok(CorpusEntry::new(None, Document::new(Vec::new()))),
            Err(DocumentError::new(
                None,
                Error::ParseFailed("worse".to_string()),
            )),
        ];
        let consumed = items.len();
        let result: CorpusResult = items.into_iter().collect();
        assert_eq!(result.corpus.entries.len() + result.errors.len(), consumed);
        assert_eq!(result.corpus.entries.len(), 2);
        assert_eq!(
            result.corpus.entries[0].path,
            Some(PathBuf::from("a.md")),
            "success order preserved"
        );
        assert_eq!(
            result.errors[0].path,
            Some(PathBuf::from("b.md")),
            "failure order preserved"
        );
        assert_eq!(result.errors[1].path, None, "path-less failure stays None");
    }

    #[test]
    fn corpus_result_from_empty_iterator() {
        let result: CorpusResult = std::iter::empty().collect();
        assert!(result.corpus.entries.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn document_error_display_with_and_without_path() {
        let with_path = DocumentError::new(
            Some(PathBuf::from("essay.md")),
            Error::ParseFailed("boom".to_string()),
        );
        assert_eq!(with_path.to_string(), "essay.md: parse failed: boom");

        let without = DocumentError::new(None, Error::ParseFailed("boom".to_string()));
        assert_eq!(without.to_string(), "parse failed: boom");
    }

    #[test]
    fn document_error_source_is_the_domain_error() {
        use std::error::Error as _;
        let err = DocumentError::new(None, Error::ParseFailed("boom".to_string()));
        assert!(err.source().is_some());
    }

    #[test]
    fn format_equality() {
        assert_eq!(Format::Markdown, Format::Markdown);
        assert_ne!(Format::Markdown, Format::PlainText);
    }

    #[test]
    fn tree_depth_returns_max_on_cycle() {
        // A -> B -> A is a cycle. The new impl returns usize::MAX for
        // tokens transitively rooted in the cycle, surfacing the
        // malformed parse loudly. The previous magic-< 20 ceiling
        // silently truncated to 20 — protective by accident, not by
        // design (and silently wrong on legitimate deep parses).
        let sent = Sentence::new(
            "cyclic".to_string(),
            vec![
                make_token(1, "A", "NOUN", "dep", 2),
                make_token(2, "B", "NOUN", "dep", 1),
            ],
        );
        assert_eq!(sent.tree_depth(), usize::MAX);
    }
}
