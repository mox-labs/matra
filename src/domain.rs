# RECOVERED-FROM-READ source=[claude-project-path]/[session-id]/subagents/[agent-transcript].jsonl timestamp=2026-04-09T13:02:33.911Z original_path=[path]/src/domain.rs
//! Domain types. The core model that everything else depends on.
//! No external dependencies beyond serde and std.

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// All errors vaani can produce. Matchable, not opaque.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Model file does not exist at the given path.
    ModelNotFound(PathBuf),
    /// Model file exists but could not be loaded (corrupt, wrong format).
    ModelInvalid(String),
    /// NLP parsing failed on the input text.
    ParseFailed(String),
    /// File I/O error.
    Io(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ModelNotFound(p) => write!(f, "model not found: {}", p.display()),
            Error::ModelInvalid(msg) => write!(f, "invalid model: {msg}"),
            Error::ParseFailed(msg) => write!(f, "parse failed: {msg}"),
            Error::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

/// Result type for vaani operations.
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
    /// #[non_exhaustive] prevents struct literal syntax outside the crate).
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
    pub fn xpos(mut self, xpos: String) -> Self {
        self.xpos = xpos;
        self
    }
    pub fn feats(mut self, feats: String) -> Self {
        self.feats = feats;
        self
    }
    pub fn deps(mut self, deps: String) -> Self {
        self.deps = deps;
        self
    }
    pub fn misc(mut self, misc: String) -> Self {
        self.misc = misc;
        self
    }
    pub fn is_punct(mut self, is_punct: bool) -> Self {
        self.is_punct = is_punct;
        self
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Sentence {
    pub text: String,
    pub tokens: Vec<Token>,
}

impl Sentence {
    pub fn new(text: String, tokens: Vec<Token>) -> Self {
        Self { text, tokens }
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
        self.tokens.iter().any(|t| {
            t.dep == "nsubj:pass" || t.dep == "nsubjpass" || t.dep == "aux:pass"
        })
    }

    /// Maximum depth of the dependency tree.
    pub fn tree_depth(&self) -> usize {
        self.tokens
            .iter()
            .map(|t| {
                let mut depth = 0;
                let mut head = t.head;
                while head != 0 && depth < 20 {
                    head = self
                        .tokens
                        .iter()
                        .find(|h| h.id == head)
                        .map(|h| h.head)
                        .unwrap_or(0);
                    depth += 1;
                }
                depth
            })
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
        self.tokens
            .iter()
            .find(|t| t.id == id)
            .and_then(|t| {
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
// Analysis output -- what encoders produce
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Paragraph {
    pub text: String,
    pub in_blockquote: bool,
    pub sentences: Vec<Sentence>,
    pub readability_grade: Option<f64>,
    pub lexical_density: Option<f64>,
    pub compression_ratio: Option<f64>,
}

impl Paragraph {
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
    pub heading: Option<String>,
    pub level: usize,
    pub paragraphs: Vec<Paragraph>,
}

impl Section {
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
pub struct Analysis {
    pub sections: Vec<Section>,
    pub vocabulary_ttr: Option<f64>,
    pub nominalization_ratio: Option<f64>,
}

impl Analysis {
    pub fn new(sections: Vec<Section>) -> Self {
        Self {
            sections,
            vocabulary_ttr: None,
            nominalization_ratio: None,
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

    pub fn total_sentences(&self) -> usize {
        self.paragraphs().map(|p| p.sentence_count()).sum()
    }

    pub fn total_words(&self) -> usize {
        self.sentences().map(|s| s.word_count()).sum()
    }

    pub fn passive_ratio(&self) -> f64 {
        let total = self.total_sentences();
        if total == 0 {
            return 0.0;
        }
        self.sentences().filter(|s| s.is_passive()).count() as f64 / total as f64
    }

    pub fn mean_sentence_length(&self) -> f64 {
        let total = self.total_sentences();
        if total == 0 {
            return 0.0;
        }
        self.total_words() as f64 / total as f64
    }

    pub fn sentence_length_std(&self) -> f64 {
        let lengths: Vec<f64> = self.sentences().map(|s| s.word_count() as f64).collect();
        if lengths.len() <= 1 {
            return 0.0;
        }
        let mean = lengths.iter().sum::<f64>() / lengths.len() as f64;
        let var = lengths.iter().map(|l| (l - mean).powi(2)).sum::<f64>()
            / (lengths.len() - 1) as f64;
        var.sqrt()
    }
}

// ---------------------------------------------------------------------------
// Extraction output — what extraction algorithms produce
// ---------------------------------------------------------------------------

/// A sentence ranked by relevance score with its original document position.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ScoredSentence {
    pub text: String,
    pub score: f64,
    pub position: usize,
}

impl ScoredSentence {
    pub fn new(text: String, score: f64, position: usize) -> Self {
        Self {
            text,
            score,
            position,
        }
    }
}

/// A ranked keyphrase extracted from text.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Keyphrase {
    pub phrase: String,
    pub score: f64,
}

impl Keyphrase {
    pub fn new(phrase: String, score: f64) -> Self {
        Self { phrase, score }
    }
}

// ---------------------------------------------------------------------------
// Source / corpus types
// ---------------------------------------------------------------------------

/// Document format, detected from file extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Format {
    Markdown,
    PlainText,
    Pdf,
    Docx,
}

/// A raw document before decomposition. Output of Source, input to Decomposer.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RawDocument {
    pub text: String,
    pub path: Option<PathBuf>,
    pub format: Format,
}

impl RawDocument {
    pub fn new(text: String, path: Option<PathBuf>, format: Format) -> Self {
        Self { text, path, format }
    }
}

/// A single entry in a corpus: one analyzed document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CorpusEntry {
    pub path: Option<PathBuf>,
    pub analysis: Analysis,
}

impl CorpusEntry {
    pub fn new(path: Option<PathBuf>, analysis: Analysis) -> Self {
        Self { path, analysis }
    }
}

/// A collection of analyzed documents.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Corpus {
    pub entries: Vec<CorpusEntry>,
}

impl Corpus {
    pub fn new(entries: Vec<CorpusEntry>) -> Self {
        Self { entries }
    }

    pub fn total_words(&self) -> usize {
        self.entries.iter().map(|e| e.analysis.total_words()).sum()
    }

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
        Sentence {
            text: "The system was built by the team".to_string(),
            tokens: vec![
                make_token(1, "The", "DET", "det", 2),
                make_token(2, "system", "NOUN", "nsubj:pass", 4),
                make_token(3, "was", "AUX", "aux:pass", 4),
                make_token(4, "built", "VERB", "root", 0),
                make_token(5, "by", "ADP", "case", 7),
                make_token(6, "the", "DET", "det", 7),
                make_token(7, "team", "NOUN", "obl", 4),
            ],
        }
    }

    fn active_sentence() -> Sentence {
        Sentence {
            text: "The team shipped the product".to_string(),
            tokens: vec![
                make_token(1, "The", "DET", "det", 2),
                make_token(2, "team", "NOUN", "nsubj", 3),
                make_token(3, "shipped", "VERB", "root", 0),
                make_token(4, "the", "DET", "det", 5),
                make_token(5, "product", "NOUN", "obj", 3),
            ],
        }
    }

    #[test]
    fn content_tokens_excludes_punct() {
        let sent = Sentence {
            text: "Hello, world!".to_string(),
            tokens: vec![
                make_token(1, "Hello", "INTJ", "root", 0),
                make_token(2, ",", "PUNCT", "punct", 1),
                make_token(3, "world", "NOUN", "vocative", 1),
                make_token(4, "!", "PUNCT", "punct", 1),
            ],
        };
        assert_eq!(sent.content_tokens().len(), 2);
        assert_eq!(sent.word_count(), 2);
    }

    #[test]
    fn is_passive_detects_passive() {
        assert!(passive_sentence().is_passive());
        assert!(!active_sentence().is_passive());
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
        let sent = Sentence {
            text: "non contiguous ids".to_string(),
            tokens: vec![
                make_token(10, "A", "NOUN", "dep", 20),
                make_token(20, "B", "NOUN", "dep", 30),
                make_token(30, "C", "VERB", "root", 0),
            ],
        };
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
        let sent = Sentence {
            text: "cyclic".to_string(),
            tokens: vec![
                make_token(1, "A", "NOUN", "dep", 2),
                make_token(2, "B", "NOUN", "dep", 1),
            ],
        };
        let sub = sent.subtree(1);
        assert_eq!(sub.len(), 2);
    }

    #[test]
    fn empty_sentence_is_safe() {
        let sent = Sentence {
            text: String::new(),
            tokens: vec![],
        };
        assert_eq!(sent.word_count(), 0);
        assert!(!sent.is_passive());
        assert_eq!(sent.tree_depth(), 0);
        assert!(sent.root_token().is_none());
        assert!(sent.children_of(1).is_empty());
        assert!(sent.head_of(1).is_none());
        assert!(sent.subtree(1).is_empty());
    }
}

[result-id: r2]