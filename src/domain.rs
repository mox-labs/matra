//! Domain types. The core model that everything else depends on.
//! Dependencies are bounded to serde, thiserror, and std.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Resource bounds
// ---------------------------------------------------------------------------

/// The upper bound on text entering the pipeline, in bytes. A constant,
/// not a setting: no configuration, argument or environment variable
/// changes it.
///
/// 8 MiB accommodates book-length English (a typical novel is ~1.5 MiB / 200k
/// words) with headroom for multilingual prose and structured documents. Beyond
/// this bound, the underlying NLP provider's intermediate memory grows past
/// safe limits on a typical workstation (UDPipe's per-token allocations cross
/// ~1 GiB resident at this input size).
///
/// `Engine::annotate`, the only route from text to the parser, enforces it
/// and returns [`Error::InputTooLarge`] with `what = "input"` when it is
/// exceeded, so every `Engine` method inherits it. `FileSource` applies the
/// same limit to a file's size before reading it (`what = "file_source"`).
/// A provider's own `NlpProvider::parse` has no limit: calling it directly
/// bypasses this bound.
pub const MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;

// ---------------------------------------------------------------------------
// Provisioning
// ---------------------------------------------------------------------------

/// What a constructor is about to download, reported before the transfer
/// begins.
///
/// A first run fetches a model over a link whose speed matra cannot know.
/// Measured cold starts ran from 3 to 35 seconds with nothing on screen,
/// which is indistinguishable from a hung process, and the natural
/// response to a hung process makes it worse. The library writes to no
/// terminal, so it hands the facts to whoever does: the UDPipe
/// constructors that provision carry a `_with_notice` form that calls a
/// closure once per fetch, and only when the artifact is not already on
/// disk.
///
/// That is `Udpipe::english_with_notice`,
/// `Udpipe::from_config_with_notice` and `Engine::from_config_with_notice`.
/// `Model2Vec::potion_base_8m` fetches 30.2 MB across three artifacts
/// and has no notice form, so a semantic first run is still silent.
///
/// Facts rather than a sentence. The caller decides the wording, the
/// stream, and whether to say anything at all.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProvisionNotice {
    /// The artifact's filename, as it will appear in `destination`.
    pub artifact: String,
    /// The artifact's pinned size in bytes. Known before the fetch
    /// because the size is a constant in the source, checked against
    /// what arrives.
    pub bytes: u64,
    /// The directory the artifact is being fetched into.
    pub destination: PathBuf,
    /// The artifact's license, named for a reader, for example
    /// `CC BY-NC-SA 4.0 (non-commercial)`. A model is a separate work
    /// from matra and carries its own terms, which need not be matra's
    /// MIT. Pinned in the source beside the URL, so a change of model
    /// changes both.
    pub license: String,
    /// Where the full text of `license` is published.
    pub license_url: String,
}

// ---------------------------------------------------------------------------
// Embedding carrier (Tier 2)
// ---------------------------------------------------------------------------

/// A dense vector representation of a text, produced by an implementation
/// of the embed port's `Embedder` trait.
///
/// Model opinion, not verifiable structure: an embedding cannot be checked
/// against the source bytes, so it never appears as a field on [`Document`]
/// or any other type the deterministic pipeline returns (RFC-0010). Values
/// derived from embeddings carry their provenance (model identity, parameters)
/// in their own standalone types.
///
/// Serde treats the newtype transparently, so the wire form is the bare
/// array. Deliberately not `#[non_exhaustive]`: on a tuple struct that
/// attribute makes the constructor crate-private, and external `Embedder`
/// implementors must construct these values; RFC-0010 records the
/// departure from the convention.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Embedding(pub Vec<f32>);

/// Semantic-similarity clusters over a document's sentences: Tier 2
/// output, standing alone, never attached to [`Document`] (RFC-0010).
///
/// Clusters are connected components of the similarity graph whose edges
/// cleared `threshold`, so co-membership is transitive: two sentences can
/// share a cluster without sharing an edge, and co-membership must never
/// be read as pairwise similarity. The above-threshold edges travel in
/// each cluster precisely so a consumer can see which pairs actually
/// cleared the bar; a missing edge means no claim, not low similarity.
///
/// A sentence with no above-threshold edge appears in no cluster:
/// singletons are excluded by construction, so "not in any cluster" is a
/// meaningful consumer count, not an artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SemanticClusters {
    /// Identity of the model whose geometry produced the scores, as the
    /// embedder reports it (`Embedder::identity`; for a file-loaded model,
    /// a digest over its artifacts).
    pub identity: String,
    /// The caller-supplied similarity cutoff the edges cleared. matra
    /// knows no universal threshold; the literature's published values
    /// span 0.67 to 0.9 with no consensus.
    pub threshold: f32,
    /// The clusters, ordered by smallest member index; each cluster's
    /// members and edges are sorted, so output order is deterministic.
    pub clusters: Vec<SemanticCluster>,
}

/// One connected component of the above-threshold similarity graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SemanticCluster {
    /// Sentence indices (document order positions) in this component.
    pub members: Vec<usize>,
    /// Every pair whose cosine similarity cleared the threshold. Pairs of
    /// members without an edge here did not clear it individually; they
    /// share the cluster through the chain.
    pub edges: Vec<SemanticEdge>,
}

/// An above-threshold similarity between two sentences, `a < b`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SemanticEdge {
    /// Lower sentence index.
    pub a: usize,
    /// Higher sentence index.
    pub b: usize,
    /// Cosine similarity, in the model's geometry.
    pub score: f32,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// What an [`Error::InputTooLarge`] message says was too large: the bare
/// word `input` for the main gate, whose label is `"input"`, and the
/// label as a qualifier (`tfidf input`) for every other gate.
struct GateSubject<'a>(&'a str);

impl std::fmt::Display for GateSubject<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            "input" => f.write_str("input"),
            label => write!(f, "{label} input"),
        }
    }
}

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
    ///
    /// The message reads `{what} input too large: {actual} > limit
    /// {limit}`, except for the main gate: its label is `"input"`
    /// already, so it reads `input too large: {actual} > limit {limit}`.
    #[error("{} too large: {actual} > limit {limit}", GateSubject(.what))]
    InputTooLarge {
        /// The size cap that was exceeded.
        limit: usize,
        /// The actual size that exceeded the cap.
        actual: usize,
        /// Discriminator naming which gate fired, so consumers can route
        /// differently per gate. The labels in use: `"input"`,
        /// `"file_source"`, `"tfidf"`, `"textrank"`, `"rake"`, `"yake"`,
        /// `"semantic_clusters"`, `"udpipe_download"`,
        /// `"embedding_download"` and `"config_file"`.
        what: &'static str,
    },
    /// The document format has no registered decomposer in this build.
    /// Seen when analyzing a Pdf/Docx file without the relevant adapter.
    #[error("unsupported format: {0:?}")]
    UnsupportedFormat(Format),
    /// A caller violated a documented API contract (mismatched argument
    /// lengths, disagreeing dimensions, a non-finite parameter). Names
    /// the violation; fix the call site, not the input data.
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// File I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl Error {
    /// The stable kind string for this variant: the key a consumer
    /// branches on instead of matching against a message.
    ///
    /// The vocabulary is part of the public contract. A binding that
    /// cannot carry a Rust enum across (`DocumentError` has no serde
    /// form, because it wraps `std::io::Error`) materializes this string
    /// instead, so every language binding names a failure the same way.
    /// `spec/tests/corpus/items.json` pins the list.
    ///
    /// Exhaustive with no wildcard: a new variant fails to compile until
    /// someone names it here, which is what stops a new failure mode
    /// from quietly inheriting another one's key.
    pub fn kind(&self) -> &'static str {
        match self {
            Error::ModelNotFound(_) => "model_not_found",
            Error::ModelInvalid(_) => "model_invalid",
            Error::ParseFailed(_) => "parse_failed",
            Error::InputTooLarge { .. } => "input_too_large",
            Error::UnsupportedFormat(_) => "unsupported_format",
            Error::InvalidInput(_) => "invalid_input",
            Error::Io(_) => "io",
        }
    }
}

/// Result type for matra operations.
pub type Result<T> = std::result::Result<T, Error>;

// ---------------------------------------------------------------------------
// Core linguistic types
// ---------------------------------------------------------------------------

/// A parsed token carrying the full CoNLL-U annotation set.
///
/// One field per CoNLL-U column plus one derived convenience field.
/// Each column holds what the NLP provider emitted for it, converted
/// where the column has structure: column 6 (`features`) arrives as a
/// map rather than the pipe-separated string, so no consumer in any
/// language re-parses it. The UDPipe adapter cannot fill column 9
/// (`deps`), which its binding does not surface, and sets it to `_`.
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
    /// Morphological features (CoNLL-U column 6) as a map from feature
    /// name to value: `Mood=Ind|Tense=Pres` arrives as
    /// `{"Mood": "Ind", "Tense": "Pres"}`. A multi-valued feature keeps
    /// its value unsplit (`Case=Nom,Acc` is `{"Case": "Nom,Acc"}`), so
    /// matra exposes what the provider emitted without normalising it.
    /// Empty when the provider emitted none (the CoNLL-U placeholder
    /// `_`). Keys are an open set: a reader ignores a feature it does not
    /// know.
    pub features: std::collections::BTreeMap<String, String>,
    /// Head token id, 0 = root (CoNLL-U column 7).
    pub head: usize,
    /// Dependency relation to head (CoNLL-U column 8).
    pub dep: String,
    /// Enhanced dependency graph (CoNLL-U column 9). Always `_` from the
    /// UDPipe adapter, whose binding does not surface this column.
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
            features: std::collections::BTreeMap::new(),
            deps: String::new(),
            misc: String::new(),
            is_punct: false,
        }
    }

    /// Look up one morphological feature by exact key:
    /// `feat("Mood")` is `Some("Ind")` when `features` holds
    /// `Mood=Ind`. A view over [`Token::features`], which crosses FFI as
    /// a map, so it adds nothing to the wire (RFC-0009).
    pub fn feat(&self, key: &str) -> Option<&str> {
        self.features.get(key).map(String::as_str)
    }
}

/// Convert a CoNLL-U column 6 string into the [`Token::features`] map.
///
/// Splits on `|`, then each pair at its first `=`. A pair with no `=`
/// or an empty name is not a feature and is dropped; when a name repeats,
/// the first value wins. The empty string and the placeholder `_` carry
/// no pair, so both give an empty map. Values are kept as written, a
/// multi-valued `Nom,Acc` unsplit.
pub(crate) fn features_from_conllu(column: &str) -> std::collections::BTreeMap<String, String> {
    let mut features = std::collections::BTreeMap::new();
    for pair in column.split('|') {
        if let Some((name, value)) = pair.split_once('=')
            && !name.is_empty()
        {
            features
                .entry(name.to_string())
                .or_insert_with(|| value.to_string());
        }
    }
    features
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
    features: std::collections::BTreeMap<String, String>,
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
    /// Set the morphological features from a CoNLL-U column 6 string
    /// (`Mood=Ind|Tense=Pres`), converted into the [`Token::features`]
    /// map.
    pub fn feats(mut self, feats: String) -> Self {
        self.features = features_from_conllu(&feats);
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
            features: self.features,
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
/// language binding reads the same detection (RFC-0008).
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
/// (RFC-0008). The shapes were verified against live UDPipe parses:
/// `not` and `never` attach as `advmod` to the word they negate,
/// determiner `no` and `neither` attach as `det` to the noun they
/// negate, and `nor` attaches as `cc` to the conjunct it links.
/// Matching on the cue lemma plus the carrying relation keeps
/// precision, and it bounds recall to those shapes by decision:
/// `nothing` as a subject (`nsubj`) does not fire, pronominal
/// `neither` ("Neither of them came.", `nsubj` on the live model
/// rather than `det`) does not fire, and tokenizer-split `cannot`
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

/// One modal auxiliary in a sentence, referenced by token id.
///
/// Reports structure only: which token carries the modal, its lemma,
/// and the token it attaches to in the dependency graph. English
/// modals are ambiguous across readings (`must` is obligation in one
/// sentence and inference in the next) and resolving that needs
/// context matra does not model, so the epistemic/deontic/dynamic
/// category is the consumer's reading, not matra's.
///
/// Derived at [`Sentence`] construction from the dependency graph
/// (see [`Sentence::new`]) and serialized with the sentence, so every
/// language binding reads the same detection (RFC-0008).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Modal {
    /// Token id of the modal auxiliary.
    pub aux_id: usize,
    /// Lemma of the auxiliary, one of the ten-lemma closed class
    /// documented on [`Sentence::modals`].
    pub aux_lemma: String,
    /// Token id the auxiliary attaches to (`0` if it is the root).
    pub head_id: usize,
}

/// Modal auxiliary detection over a sentence's tokens.
///
/// The single Rust implementation behind [`Sentence::modals`]
/// (RFC-0008). The closed class is the ten lemmas the UD English
/// treebank tags `MD`, enumerated from live UDPipe parses (verified
/// 2026-08-21): `can`, `could`, `may`, `might`, `must`, `ought`,
/// `shall`, `should`, `will`, `would`. A class lemma matches when it
/// carries the `aux` relation or the `AUX` part of speech, because
/// the live model does not park every modal on `aux`: under VP
/// ellipsis ("Yes, you should.") the modal is promoted to root, and
/// with coordinated auxiliaries the first conjunct parses as root
/// with the second as `aux` ("He can and will succeed.") or `conj`
/// ("He can, and he will, succeed."), all verified live. The lemma
/// plus either signal keeps precision: the model's only other `AUX`
/// lemmas (`be`, `have`, `do`, `get`) are outside the class,
/// contracted forms fire because the model lemmatizes them into the
/// class (`wo` in `won't` carries lemma `will`, verified live), the
/// noun `will` does not fire because it never carries `aux` or
/// `AUX`, and `need`/`dare` are outside the class because the live
/// model tags them as main verbs (`VERB`, root), not modal
/// auxiliaries. In a multi-auxiliary chain (`might have been done`)
/// only the modal is in the class; `have` and `be` attach as
/// `aux`/`aux:pass` but are not modals, so the chain reports exactly
/// its modals, each with `head_id` pointing at the content verb.
fn detect_modals(tokens: &[Token]) -> Vec<Modal> {
    const MODAL_LEMMAS: [&str; 10] = [
        "can", "could", "may", "might", "must", "ought", "shall", "should", "will", "would",
    ];
    tokens
        .iter()
        .filter(|t| (t.dep == "aux" || t.pos == "AUX") && MODAL_LEMMAS.contains(&t.lemma.as_str()))
        .map(|t| Modal {
            aux_id: t.id,
            aux_lemma: t.lemma.clone(),
            head_id: t.head,
        })
        .collect()
}

/// The grammatical mood of a sentence's root clause, with the root's
/// token id.
///
/// Reports the `Mood` feature the parse assigns the root clause, read
/// off the root token itself or, when the root carries none, off its
/// first `cop`, `aux` or `aux:pass` child that does. The live model
/// parks finiteness on the auxiliary rather than the root in copular
/// ("The sky is blue.", root `ADJ`, `Mood=Ind` on the `cop`),
/// do-support ("He did leave.", root infinitive, `Mood=Ind` on the
/// `aux`) and passive ("The will was signed.", root participle,
/// `Mood=Ind` on the `aux:pass`) clauses, all verified live.
///
/// `mood` is the value as the provider emitted it (`Ind`, `Imp`, and
/// whatever else a model assigns), so a reader treats it as an open set.
/// Whether the clause is modalized is a separate fact: the modals that
/// attach to the root are in [`Sentence::modals`], each with its
/// `head_id`. What the mood commits the speaker to is the consumer's
/// reading, not matra's.
///
/// Derived at [`Sentence`] construction from the dependency graph (see
/// [`Sentence::new`]) and serialized with the sentence, so every
/// language binding reads the same detection (RFC-0008).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RootMood {
    /// Token id of the sentence root (the token with `head == 0`).
    pub root_id: usize,
    /// The root clause's `Mood` feature value, for example `Ind`.
    pub mood: String,
}

/// Root-clause mood detection over a sentence's tokens.
///
/// The single Rust implementation behind [`Sentence::root_mood`]. `None`
/// when the sentence has no root, or when neither the root nor any of
/// its finite-carrier children (`cop`, `aux`, `aux:pass`) carries a
/// `Mood` feature, as in a modalized clause whose modal carries only
/// `VerbForm=Fin`. The read is of the root clause only: a mood in a
/// subordinate clause does not reach it.
fn detect_root_mood(tokens: &[Token]) -> Option<RootMood> {
    const FINITE_CARRIER_DEPS: [&str; 3] = ["cop", "aux", "aux:pass"];
    let root = tokens.iter().find(|t| t.head == 0)?;
    let mood = root.feat("Mood").or_else(|| {
        tokens
            .iter()
            .filter(|t| t.head == root.id && FINITE_CARRIER_DEPS.contains(&t.dep.as_str()))
            .find_map(|t| t.feat("Mood"))
    })?;
    Some(RootMood {
        root_id: root.id,
        mood: mood.to_string(),
    })
}

/// One clausal complement in a sentence, referenced by token id: a verb
/// governing a clause (`ccomp`).
///
/// Reports structure only: the verb, the head of the complement clause,
/// and the verb's subject when the parse has one in the same sentence.
/// The construction fires regardless of which verb fills it (`show`,
/// `report`, `suggest`, `think`, `ensure` all parse identically),
/// because verbs taking a clausal complement are an open class: any list
/// matra shipped would be incomplete while looking authoritative. Which
/// verb lemmas count as reporting or evidential is the consumer's
/// lexicon (see [`Sentence::clausal_complements_in`]), and whether the
/// source is credible or the claim attributed rather than asserted is
/// the consumer's reading, not matra's.
///
/// Derived at [`Sentence`] construction from the dependency graph
/// (see [`Sentence::new`]) and serialized with the sentence, so every
/// language binding reads the same detection (RFC-0008).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClausalComplement {
    /// Token id of the verb governing the clausal complement.
    pub verb_id: usize,
    /// Lemma of that verb.
    pub verb_lemma: String,
    /// Token id of the head of the clausal complement (the token
    /// carrying the `ccomp` relation).
    pub ccomp_id: usize,
    /// Token id of the verb's subject (`nsubj`, `nsubj:pass`,
    /// `nsubjpass`), when the parse has one in this sentence.
    pub subject_id: Option<usize>,
    /// Lemma of that subject, when present.
    pub subject_lemma: Option<String>,
}

/// Clausal-complement detection over a sentence's tokens.
///
/// The single Rust implementation behind [`Sentence::clausal_complements`]
/// (RFC-0008). The shape was verified against live UDPipe parses
/// (2026-08-21): in "Smith reported that the effect vanished." the
/// complement head `vanished` attaches as `ccomp` to `reported`
/// (`VERB`, root) and `Smith` attaches as `nsubj` to `reported`. The
/// detector walks every `ccomp` arc whose head is a `VERB` and
/// records the verb, the complement head, and the verb's subject
/// child if it has one. The subject is optional by observed fact,
/// not caution: UDPipe splits "Smith et al. reported that ..." into
/// two sentences at the period in "et al." (verified live), leaving
/// the reporting verb with its `ccomp` but no `nsubj`, the
/// attribution stranded in the previous sentence. That is an
/// upstream segmentation defect this detector inherits and records
/// rather than papers over. A `ccomp` under a non-verb head ("I am
/// sure that it works.", `ccomp` on the adjective `sure`) is outside
/// the construction by decision: the detector reports verbal
/// complements only.
fn detect_clausal_complements(tokens: &[Token]) -> Vec<ClausalComplement> {
    const SUBJECT_DEPS: [&str; 3] = ["nsubj", "nsubj:pass", "nsubjpass"];
    tokens
        .iter()
        .filter(|c| c.dep == "ccomp")
        .filter_map(|c| {
            let verb = tokens.iter().find(|t| t.id == c.head && t.pos == "VERB")?;
            let subject = tokens
                .iter()
                .find(|t| t.head == verb.id && SUBJECT_DEPS.contains(&t.dep.as_str()));
            Some(ClausalComplement {
                verb_id: verb.id,
                verb_lemma: verb.lemma.clone(),
                ccomp_id: c.id,
                subject_id: subject.map(|s| s.id),
                subject_lemma: subject.map(|s| s.lemma.clone()),
            })
        })
        .collect()
}

/// One adverbial modifier attached to the sentence root, referenced
/// by token id.
///
/// Reports structure only: the `advmod` arc into the root, which is
/// where sentence-scope adverbs land ("Reportedly, the deal closed."
/// parses `reportedly` as `advmod` on the root `closed`, verified
/// live). The dependency graph does not distinguish sentence scope
/// from manner ("The team quickly shipped." puts `quickly` on the
/// same arc), and evidential adverbs are an open class, so matra
/// reports every root-attached adverbial and ships no lexicon: which
/// lemmas read as hearsay markers is the consumer's list (see
/// [`Sentence::root_adverbials_in`]), and what the marker does to
/// the claim is the consumer's reading, not matra's.
///
/// Derived at [`Sentence`] construction from the dependency graph
/// (see [`Sentence::new`]) and serialized with the sentence, so every
/// language binding reads the same detection (RFC-0008).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RootAdverbial {
    /// Token id of the adverbial modifier attached to the root.
    pub adv_id: usize,
    /// Lemma of the adverbial.
    pub adv_lemma: String,
}

/// Root-attached adverbial detection over a sentence's tokens.
///
/// The single Rust implementation behind [`Sentence::root_adverbials`]
/// (RFC-0008). Every token carrying `advmod` whose head is the root
/// is reported, whatever its part of speech: negation particles
/// ("not" is `PART` on the same arc) appear here and in
/// [`Sentence::negations`], and the overlap is intentional, because
/// both fields report arcs and neither assigns a reading. A sentence
/// with no root (malformed parse) reports none.
fn detect_root_adverbials(tokens: &[Token]) -> Vec<RootAdverbial> {
    let Some(root) = tokens.iter().find(|t| t.head == 0) else {
        return Vec::new();
    };
    tokens
        .iter()
        .filter(|t| t.dep == "advmod" && t.head == root.id)
        .map(|t| RootAdverbial {
            adv_id: t.id,
            adv_lemma: t.lemma.clone(),
        })
        .collect()
}

/// Which of the six Hearst (1992) lexico-syntactic patterns produced a
/// candidate hypernymy pair.
///
/// Each variant names a surface construction, not a semantic verdict:
/// the tag records which dependency shape matched, and whether the
/// hypernymy relation actually holds is the consumer's reading, not
/// matra's. Serialized in `snake_case` (`"such_as"`, `"and_other"`,
/// ...) so the tag crosses FFI as a plain string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum HearstPattern {
    /// `NP such as NP`: "animals such as dogs".
    SuchAs,
    /// `such NP as NP`: "such authors as Herrick".
    SuchNpAs,
    /// `NP, including NP`: "countries, including Canada".
    Including,
    /// `NP, especially NP`: "countries, especially France".
    Especially,
    /// `NP and other NP`: "temples and other buildings".
    AndOther,
    /// `NP or other NP`: "bruises or other injuries".
    OrOther,
}

/// One noun phrase in a Hearst pair, referenced by token id.
///
/// `head_id` is the syntactic head noun; `first_id..=last_id` is the
/// contiguous token range of that noun plus its adjacent nominal
/// modifiers (`det`, `amod`, `compound`, `nummod`, `flat`), with the
/// pattern's own marker words (`such`, `other`) excluded. Ids are
/// sentence-scoped, so provenance holds against
/// [`Sentence::tokens`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HearstSpan {
    /// Token id of the head noun.
    pub head_id: usize,
    /// Lemma of the head noun.
    pub head_lemma: String,
    /// Token id of the first token in the span.
    pub first_id: usize,
    /// Token id of the last token in the span.
    pub last_id: usize,
}

/// One candidate hypernymy pair from a Hearst pattern, as structure.
///
/// Reports the two spans and the construction that connected them.
/// It is a *candidate* by design: matra does not build a taxonomy or
/// assert the relation is true, it reports that the sentence used a
/// construction which conventionally signals one. Derived at the
/// annotate stage by the pipeline (the detector lives in
/// `matra::hearst`, outside the domain) and serialized with the
/// sentence, so every language binding reads the same detection
/// (RFC-0008).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HearstPair {
    /// The construction that matched.
    pub pattern: HearstPattern,
    /// The general term's span (the class).
    pub hypernym: HearstSpan,
    /// The specific term's span (the member).
    pub hyponym: HearstSpan,
}

/// Passive-voice detection over a sentence's tokens: the single Rust
/// implementation behind [`Sentence::passive`]. True when any token
/// carries `nsubj:pass`, `nsubjpass` or `aux:pass`.
fn detect_passive(tokens: &[Token]) -> bool {
    tokens
        .iter()
        .any(|t| t.dep == "nsubj:pass" || t.dep == "nsubjpass" || t.dep == "aux:pass")
}

/// Dependency-tree depth over a sentence's tokens: the single Rust
/// implementation behind [`Sentence::tree_depth`].
///
/// O(n) per sentence: builds a `HashMap<id, head>` once, then walks
/// each token's ancestor chain with depth memoization in a second
/// `HashMap<id, usize>`. Each token's depth is computed at most once;
/// the visited set inside each walk detects a cycle, and a cycle
/// anywhere makes the sentence's depth `None`, surfacing the malformed
/// parse rather than silently truncating it. There is no depth ceiling.
fn measure_tree_depth(tokens: &[Token]) -> Option<usize> {
    use std::collections::{HashMap, HashSet};

    // Build id -> head once. `0` head means root.
    let head_by_id: HashMap<usize, usize> = tokens.iter().map(|t| (t.id, t.head)).collect();

    // Memoize depth per token id. `None` marks a token on or rooted in
    // a cycle.
    let mut depth_by_id: HashMap<usize, Option<usize>> = HashMap::new();

    // Compute depth for one token. Walks up the head chain, using a
    // visited set to detect cycles in this single chain. On hit,
    // memoizes via the cache to amortize across tokens that share
    // ancestors.
    fn depth_of(
        id: usize,
        head_by_id: &HashMap<usize, usize>,
        depth_by_id: &mut HashMap<usize, Option<usize>>,
    ) -> Option<usize> {
        if let Some(&d) = depth_by_id.get(&id) {
            return d;
        }
        let mut visited: HashSet<usize> = HashSet::new();
        let mut chain: Vec<usize> = Vec::new();
        let mut cur = id;

        // Walk to root or to a memoized ancestor or to a cycle.
        let base_depth = loop {
            if !visited.insert(cur) {
                // Cycle detected: every token in the chain is malformed.
                for &cid in &chain {
                    depth_by_id.insert(cid, None);
                }
                return None;
            }
            let head = head_by_id.get(&cur).copied().unwrap_or(0);
            if head == 0 {
                // cur is root or its head is missing: depth contribution from this point is 0.
                chain.push(cur);
                break 0usize;
            }
            if let Some(&d) = depth_by_id.get(&head) {
                let Some(d) = d else {
                    // The ancestor sits on a cycle, so this chain does too.
                    chain.push(cur);
                    for &cid in &chain {
                        depth_by_id.insert(cid, None);
                    }
                    return None;
                };
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
            depth_by_id.insert(cid, Some(d));
        }
        Some(base_depth.saturating_add(n - 1))
    }

    let mut deepest = 0usize;
    for t in tokens {
        deepest = deepest.max(depth_of(t.id, &head_by_id, &mut depth_by_id)?);
    }
    Some(deepest)
}

/// One parsed sentence: its text plus its ordered tokens.
///
/// Invariants downstream code relies on:
/// - `tokens` are id-sorted ascending.
/// - Exactly one token has `head == 0` (the syntactic root).
/// - All `head` references point to another token in the same sentence
///   or to `0`.
/// - Following `head` references never loops back on itself.
///
/// Nothing validates these: [`Sentence::new`] trusts its caller, and the
/// `NlpProvider` contract states them for providers. The tree walks stay
/// safe on input that breaks the last one: [`Sentence::tree_depth`] is
/// `None` on a cycle and [`Sentence::subtree`] carries a visited set.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Sentence {
    /// Sentence text as the NLP provider reports it. The UDPipe adapter
    /// rebuilds it from the token forms, joined by a space unless a token
    /// carries `SpaceAfter=No` in `misc`, so its whitespace can differ
    /// from the source.
    pub text: String,
    /// CoNLL-U tokens in id-sorted order.
    pub tokens: Vec<Token>,
    /// Count of non-punctuation tokens, derived at construction.
    ///
    /// Counts linguistic tokens as the NLP provider identified them, so
    /// it can differ from a whitespace split of the same text.
    /// Readability (Flesch-Kincaid) splits on whitespace, as the
    /// formula's specification asks.
    pub word_count: usize,
    /// Maximum depth of the dependency tree, derived at construction:
    /// the longest path from any token to the root, the root at depth 0.
    ///
    /// `None` when following `head` references from some token loops
    /// (a malformed parse), so the failure is a value a reader must
    /// handle rather than a number that reads as a very deep tree. The
    /// walk carries a visited set and has no depth ceiling: a well-formed
    /// tree of any depth reports its true depth. `Some(0)` for a sentence
    /// with no tokens.
    pub tree_depth: Option<usize>,
    /// Whether the sentence contains a passive-voice construction (a
    /// token carrying `nsubj:pass`, `nsubjpass` or `aux:pass`), derived
    /// at construction.
    pub passive: bool,
    /// Negation cues derived from the dependency graph at construction
    /// (see [`Sentence::new`]). Serialized with the sentence so the
    /// detection crosses FFI as data (RFC-0008).
    pub negations: Vec<Negation>,
    /// Modal auxiliaries derived from the dependency graph at
    /// construction (see [`Sentence::new`]): tokens from the closed
    /// class `can`, `could`, `may`, `might`, `must`, `ought`,
    /// `shall`, `should`, `will`, `would`, matched by lemma on the
    /// `aux` relation or the `AUX` part of speech (the latter catches
    /// modals the model promotes to root or `conj` under VP ellipsis
    /// and coordination). Serialized with the sentence so the
    /// detection crosses FFI as data (RFC-0008).
    pub modals: Vec<Modal>,
    /// Clausal complements derived from the dependency graph at
    /// construction (see [`Sentence::new`]): each verb governing a
    /// clause (`ccomp`), with its subject when the parse has one. The
    /// construction is reported for every verb that fills it; which
    /// verbs count as reporting or evidential is the consumer's lexicon
    /// ([`Sentence::clausal_complements_in`]). Serialized with the
    /// sentence so the detection crosses FFI as data (RFC-0008).
    pub clausal_complements: Vec<ClausalComplement>,
    /// Adverbial modifiers attached to the root, derived at
    /// construction (see [`Sentence::new`]): the arc sentence-scope
    /// adverbs land on. Every root-attached `advmod` is reported;
    /// which lemmas read as evidential is the consumer's lexicon
    /// ([`Sentence::root_adverbials_in`]). Serialized with the
    /// sentence so the detection crosses FFI as data (RFC-0008).
    pub root_adverbials: Vec<RootAdverbial>,
    /// Candidate hypernymy pairs from the six Hearst (1992) patterns.
    ///
    /// Unlike the other derived fields, this one is not computed by
    /// [`Sentence::new`]: the detector lives in `matra::hearst`, a
    /// module outside the domain (EP-0007 M5 boundary), so the pipeline
    /// fills the field at the annotate stage. A hand-built `Sentence`
    /// carries an empty vector until the caller runs the detector.
    /// Serialized with the sentence so the detection crosses FFI as
    /// data (RFC-0008).
    pub hearst_pairs: Vec<HearstPair>,
    /// The root clause's grammatical mood with the root's token id,
    /// derived at construction (see [`RootMood`]). `None` when the
    /// sentence has no root or its root clause carries no `Mood`
    /// feature. Serialized with the sentence so the detection crosses
    /// FFI as data (RFC-0008).
    pub root_mood: Option<RootMood>,
}

impl Sentence {
    /// Construct a `Sentence` from a text string and a token vector.
    ///
    /// The caller is responsible for upholding the invariants documented
    /// on [`Sentence`]; this constructor does not validate them. Derived
    /// fields (`word_count`, `tree_depth`, `passive`, `negations`,
    /// `modals`, `clausal_complements`, `root_adverbials`, `root_mood`)
    /// are computed here and reflect `tokens` as passed in; mutating
    /// `tokens` afterwards does not recompute them, and keeping them
    /// consistent is part of the same caller contract.
    /// `hearst_pairs` starts empty and is filled at the annotate stage
    /// by the pipeline, because its detector lives outside the domain
    /// (see the field's documentation).
    pub fn new(text: String, tokens: Vec<Token>) -> Self {
        let word_count = tokens.iter().filter(|t| !t.is_punct).count();
        let tree_depth = measure_tree_depth(&tokens);
        let passive = detect_passive(&tokens);
        let negations = detect_negations(&tokens);
        let modals = detect_modals(&tokens);
        let clausal_complements = detect_clausal_complements(&tokens);
        let root_adverbials = detect_root_adverbials(&tokens);
        let root_mood = detect_root_mood(&tokens);
        Self {
            text,
            tokens,
            word_count,
            tree_depth,
            passive,
            negations,
            modals,
            clausal_complements,
            root_adverbials,
            hearst_pairs: Vec::new(),
            root_mood,
        }
    }

    /// The clausal complements whose verb lemma is in the
    /// caller-supplied lexicon.
    ///
    /// Verbs taking a clausal complement are an open class, so matra
    /// ships no default lexicon: an incomplete list that looks
    /// authoritative is worse than none. The structural detection
    /// ([`Sentence::clausal_complements`]) crosses FFI as data; this is a
    /// Rust-side convenience view over it, per RFC-0008's criterion
    /// (derivations cross as fields, views over crossing data stay
    /// methods). Non-Rust consumers filter the field by `verb_lemma` the
    /// same way.
    pub fn clausal_complements_in(&self, lexicon: &[&str]) -> Vec<&ClausalComplement> {
        self.clausal_complements
            .iter()
            .filter(|r| lexicon.contains(&r.verb_lemma.as_str()))
            .collect()
    }

    /// The root-attached adverbials whose lemma is in the
    /// caller-supplied lexicon.
    ///
    /// Evidential adverbs are an open class, so matra ships no default
    /// lexicon; see [`Sentence::clausal_complements_in`] for the
    /// reasoning and the RFC-0008 classification. Non-Rust consumers
    /// filter [`Sentence::root_adverbials`] by `adv_lemma` the same way.
    pub fn root_adverbials_in(&self, lexicon: &[&str]) -> Vec<&RootAdverbial> {
        self.root_adverbials
            .iter()
            .filter(|a| lexicon.contains(&a.adv_lemma.as_str()))
            .collect()
    }

    /// Tokens excluding punctuation.
    pub fn content_tokens(&self) -> Vec<&Token> {
        self.tokens.iter().filter(|t| !t.is_punct).collect()
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
// Document output -- what the annotate and compose stages fill
// ---------------------------------------------------------------------------

/// The paragraph-level metric record: what `Engine::compose` measures
/// over one paragraph.
///
/// Each slot gates on its own condition and is `None` when the
/// paragraph did not meet it (or when the metric suite has not run), so
/// no outer `Option` wraps the record.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ParagraphMetrics {
    /// Flesch-Kincaid grade level, when the paragraph holds more than
    /// ten words.
    pub readability_grade: Option<f64>,
    /// Content-word ratio, when the paragraph holds a word.
    pub lexical_density: Option<f64>,
    /// Brotli compression ratio (a redundancy proxy), when the paragraph
    /// holds more than fifty words and is within the compression size
    /// bound.
    pub compression_ratio: Option<f64>,
}

/// One paragraph of prose. `Engine::annotate` attaches its sentences and
/// `Engine::compose`, which runs the metric suite, fills its
/// [`ParagraphMetrics`].
///
/// `in_blockquote = true` paragraphs are skipped by both stages: they keep
/// no sentences, and their metric slots stay `None`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Paragraph {
    /// Paragraph text as the decomposer cut it from the input: trimmed at
    /// both ends, for markdown with its lines rejoined by `\n` (so `\r\n`
    /// line endings arrive as `\n`), and for a blockquote with the `>`
    /// markers removed. Not a slice of the input.
    pub text: String,
    /// Whether the paragraph is inside a blockquote (never parsed, never
    /// measured).
    pub in_blockquote: bool,
    /// Sentences produced by parsing this paragraph (attached by
    /// `Engine::annotate`).
    pub sentences: Vec<Sentence>,
    /// The paragraph's metrics, filled by `Engine::compose`.
    pub metrics: ParagraphMetrics,
}

impl Paragraph {
    /// Construct a new `Paragraph` with empty sentences and an empty
    /// metric record. `Engine::annotate` attaches the sentences and
    /// `Engine::compose` fills the record.
    pub fn new(text: String, in_blockquote: bool) -> Self {
        Self {
            text,
            in_blockquote,
            sentences: Vec::new(),
            metrics: ParagraphMetrics::default(),
        }
    }

    /// Total word count across all sentences.
    pub fn word_count(&self) -> usize {
        self.sentences.iter().map(|s| s.word_count).sum()
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

/// The document-level metric record: what `Engine::compose` measures
/// over the whole document.
///
/// The counts are what the command line's table prints and what a
/// binding would otherwise rebuild, materialized so they cross FFI as
/// data (methods do not). The ratios gate on their own conditions and
/// are `None` when unmet, so no outer `Option` wraps the record. Before
/// the metric suite runs, every count is `0` and every ratio `None`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DocumentMetrics {
    /// Non-punctuation tokens across every sentence.
    pub total_words: usize,
    /// Sentences across every paragraph.
    pub total_sentences: usize,
    /// Paragraphs across every section, blockquotes included.
    pub paragraph_count: usize,
    /// Mean sentence length in words, when the document holds a
    /// sentence.
    pub mean_sentence_length: Option<f64>,
    /// Sample standard deviation of sentence length in words, when the
    /// document holds at least two sentences (the sample deviation of
    /// one value is undefined).
    pub sentence_length_std: Option<f64>,
    /// Vocabulary type-token ratio over lemmas, when the document holds
    /// a non-punctuation token.
    ///
    /// A raw ratio with no length correction: it falls as a document
    /// grows, because repetition accumulates, so two documents of
    /// different lengths are not comparable on it.
    pub vocabulary_ttr: Option<f64>,
    /// Share of non-punctuation tokens that are nouns ending in a
    /// nominalizing suffix, when the document holds a non-punctuation
    /// token.
    pub nominalization_ratio: Option<f64>,
    /// Fraction of sentences containing a passive-voice construction,
    /// when the document holds a sentence.
    pub passive_ratio: Option<f64>,
}

/// The version of the document schema this build emits, as
/// `"major.minor"`: the value of [`Provenance::schema`] and nowhere else.
///
/// A minor adds optional fields only, which a reader that ignores fields
/// it does not recognize never notices. A major is anything else: a
/// field removed, renamed or retyped, or a field whose meaning changes.
/// A reader rejects a major it does not know. The published JSON Schema
/// for each major is in `spec/schema/`.
pub const SCHEMA_VERSION: &str = "2.0";

/// Who produced a result: a model-backed component named by what it is,
/// which model it ran and that model's digest and licence.
///
/// The parser reports its own, so a consumer sees which model produced a
/// parse rather than discovering a change of model as a shift in the
/// numbers. A component loaded from a model matra does not pin knows the
/// model's digest but not its name or licence, and leaves those `None`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Identity {
    /// The component, for example `udpipe`.
    pub name: String,
    /// The model's name, when the component runs a model matra pins,
    /// for example `english-ewt-ud-2.5-191206`.
    pub model: Option<String>,
    /// SHA-256 of the model artifact, lowercase hex, when the component
    /// runs a model.
    pub sha256: Option<String>,
    /// The model's licence, named for a reader, when it is known. A model
    /// is a separate work from matra and carries its own terms.
    pub license: Option<String>,
}

impl Identity {
    /// Construct an identity from its four facts.
    pub fn new(
        name: impl Into<String>,
        model: Option<String>,
        sha256: Option<String>,
        license: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            model,
            sha256,
            license,
        }
    }
}

/// The input a [`Document`] was analysed from: where it came from, how
/// many bytes it held, and their digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProvenanceSource {
    /// The path the input was read from, `None` for in-memory text.
    ///
    /// In JSON a path that is not valid UTF-8 has its undecodable bytes
    /// replaced with U+FFFD, since a JSON string cannot carry them; the
    /// Rust value keeps the path exact.
    #[serde(serialize_with = "serialize_path_lossy")]
    pub path: Option<PathBuf>,
    /// The input's length in bytes.
    pub bytes: usize,
    /// SHA-256 of the input's bytes, lowercase hex, computed once when
    /// the document is annotated.
    pub sha256: String,
}

impl ProvenanceSource {
    /// Construct the record of one input.
    pub fn new(path: Option<PathBuf>, bytes: usize, sha256: impl Into<String>) -> Self {
        Self {
            path,
            bytes,
            sha256: sha256.into(),
        }
    }
}

/// A path as a JSON string, lossily where the path is not UTF-8. serde's
/// own form refuses such a path, which would fail the whole document
/// over a name.
fn serialize_path_lossy<S: serde::Serializer>(
    path: &Option<PathBuf>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    match path {
        Some(p) => serializer.serialize_some(&p.to_string_lossy()),
        None => serializer.serialize_none(),
    }
}

/// Where a [`Document`] came from: the matra build and schema version
/// that produced it, the parser that read it, the format it was read as,
/// and the input itself.
///
/// No timestamp: matra's outputs are deterministic, and a timestamp would
/// make two identical analyses differ.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Provenance {
    /// The matra version that produced the result, for example `0.3.0`.
    pub matra: String,
    /// The schema version of the result, `"major.minor"`
    /// ([`SCHEMA_VERSION`]). The one place a schema version appears.
    pub schema: String,
    /// The parser that produced the sentences and tokens.
    pub parser: Identity,
    /// The format the input was read as.
    pub format: Format,
    /// The input.
    pub source: ProvenanceSource,
}

impl Provenance {
    /// The provenance of a result this build produces: `matra` and
    /// `schema` are this build's own, the rest is what the caller names.
    pub fn new(parser: Identity, format: Format, source: ProvenanceSource) -> Self {
        Self {
            matra: env!("CARGO_PKG_VERSION").to_string(),
            schema: SCHEMA_VERSION.to_string(),
            parser,
            format,
            source,
        }
    }

    /// The provenance of a hand-built document in a unit test: a parser
    /// named `test`, plain text, an empty in-memory input.
    #[cfg(test)]
    pub(crate) fn for_test() -> Self {
        Self::new(
            Identity::new("test", None, None, None),
            Format::PlainText,
            ProvenanceSource::new(None, 0, ""),
        )
    }
}

/// The full analysis output. `Engine::annotate` builds the section tree,
/// attaches each paragraph's parsed sentences and records the
/// provenance; `Engine::compose` fills the metric records.
///
/// Paragraphs live in sections (single source of truth). The document's
/// aggregates are a record, [`DocumentMetrics`], rather than methods, so
/// they cross FFI as data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Document {
    /// Where the document came from and what produced it.
    pub provenance: Provenance,
    /// Section tree (the single source of truth for paragraph ownership).
    pub sections: Vec<Section>,
    /// The document's metrics, filled by `Engine::compose`.
    pub metrics: DocumentMetrics,
}

impl Document {
    /// Construct a new `Document` from its provenance and a section tree,
    /// with an empty metric record; `Engine::compose` fills it in.
    pub fn new(provenance: Provenance, sections: Vec<Section>) -> Self {
        Self {
            provenance,
            sections,
            metrics: DocumentMetrics::default(),
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
    /// The selected sentence's text, as on [`Sentence::text`].
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
///
/// Serialized in the format vocabulary every surface shares:
/// `markdown`, `plain_text`, `pdf`, `docx`. A reader treats the set as
/// open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

impl Format {
    /// The format a path's extension implies.
    ///
    /// The mapping lives here, on the type it produces, because two
    /// callers need it and a second copy is a second answer waiting to
    /// happen: the file source assigns a format to a file on disk, and
    /// the command line assigns one to the name given for stdin. Both
    /// read this function, so `notes.md` cannot mean markdown on one
    /// route and plain text on the other.
    ///
    /// An extension this build does not know, and a path carrying no
    /// extension at all, are [`Format::PlainText`]. The format selects a
    /// decomposer, and plain text is the decomposer that reads anything.
    ///
    /// ```
    /// # use matra::domain::Format;
    /// assert_eq!(Format::from_path("notes.md"), Format::Markdown);
    /// assert_eq!(Format::from_path("notes"), Format::PlainText);
    /// ```
    pub fn from_path(path: impl AsRef<Path>) -> Format {
        match path.as_ref().extension().and_then(|e| e.to_str()) {
            Some("md" | "markdown") => Format::Markdown,
            Some("pdf") => Format::Pdf,
            Some("docx") => Format::Docx,
            _ => Format::PlainText,
        }
    }
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
        self.entries
            .iter()
            .map(|e| e.analysis.metrics.total_words)
            .sum()
    }

    /// Fraction of sentences across all entries containing a passive-
    /// voice construction. Returns `0.0` when the corpus has no
    /// sentences.
    pub fn passive_ratio(&self) -> f64 {
        let total: usize = self
            .entries
            .iter()
            .map(|e| e.analysis.metrics.total_sentences)
            .sum();
        if total == 0 {
            return 0.0;
        }
        let passive: usize = self
            .entries
            .iter()
            .map(|e| e.analysis.sentences().filter(|s| s.passive).count())
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
            .filter_map(|p| p.metrics.readability_grade)
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

    /// The wire form RFC-0010 commits to: a newtype serializes as the
    /// bare array, not an object wrapping one.
    #[test]
    fn embedding_serializes_as_bare_array() {
        let e = Embedding(vec![1.0, -0.5, 0.25]);
        let json = serde_json::to_string(&e).expect("serialize");
        assert_eq!(json, "[1.0,-0.5,0.25]");
        let back: Embedding = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, e);
    }

    fn make_token(id: usize, text: &str, pos: &str, dep: &str, head: usize) -> Token {
        Token {
            id,
            text: text.to_string(),
            lemma: text.to_lowercase(),
            pos: pos.to_string(),
            xpos: String::new(),
            features: std::collections::BTreeMap::new(),
            head,
            dep: dep.to_string(),
            deps: String::new(),
            misc: String::new(),
            is_punct: pos == "PUNCT",
        }
    }

    /// [`make_token`] with a feats string, for fixtures where the
    /// morphology is load-bearing (Mood on roots, VerbForm on
    /// auxiliaries). Lemma defaults to the lowercased text, so pass
    /// the lemma as `text` when the two differ.
    fn make_token_feats(
        id: usize,
        text: &str,
        pos: &str,
        dep: &str,
        head: usize,
        feats: &str,
    ) -> Token {
        let mut tok = make_token(id, text, pos, dep, head);
        tok.features = features_from_conllu(feats);
        tok
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
        assert_eq!(sent.word_count, 2);
    }

    #[test]
    fn passive_detects_passive() {
        assert!(passive_sentence().passive);
        assert!(!active_sentence().passive);
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
    fn negation_no_fire_on_pronominal_neither() {
        // "Neither of them came." Pronominal `neither` heads the
        // subject itself (DET, `nsubj` on the live model) instead of
        // attaching as `det` to a noun it negates. Out of scope by the
        // same decision that excludes `nothing` as a subject: a cue
        // fires only on its carrying relation, and `nsubj` is not one.
        // This test records the recall boundary; widening it means
        // changing detect_negations deliberately, not by drift.
        let sent = Sentence::new(
            "Neither of them came.".to_string(),
            vec![
                make_token(1, "Neither", "DET", "nsubj", 4),
                make_token(2, "of", "ADP", "case", 3),
                make_token(3, "them", "PRON", "nmod", 1),
                make_token(4, "came", "VERB", "root", 0),
                make_token(5, ".", "PUNCT", "punct", 4),
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
    fn negations_cross_the_wire() {
        // The field crosses the wire as data (RFC-0008).
        let sent = Sentence::new(
            "It was never reviewed.".to_string(),
            vec![make_token(1, "never", "ADV", "advmod", 0)],
        );
        let json = serde_json::to_value(&sent).unwrap();
        assert_eq!(json["negations"][0]["cue_lemma"], "never");
    }

    // Modal fixtures mirror live UDPipe parses (verified 2026-08-21):
    // every member of the closed class is AUX/`aux` with xpos `MD` and
    // `VerbForm=Fin`; the verb it scopes is `VerbForm=Inf` (or a
    // participle in a chain) and carries no `Mood`. Indicative roots
    // carry `Mood=Ind`; imperative roots carry `Mood=Imp`.

    /// A minimal modalized clause: `aux` with the given lemma
    /// attached to an infinitive root.
    fn modal_clause(lemma: &str) -> Sentence {
        Sentence::new(
            format!("They {lemma} leave."),
            vec![
                make_token(1, "They", "PRON", "nsubj", 3),
                make_token_feats(2, lemma, "AUX", "aux", 3, "VerbForm=Fin"),
                make_token_feats(3, "leave", "VERB", "root", 0, "VerbForm=Inf"),
                make_token(4, ".", "PUNCT", "punct", 3),
            ],
        )
    }

    #[test]
    fn modal_every_closed_class_lemma_is_found() {
        for lemma in [
            "can", "could", "may", "might", "must", "ought", "shall", "should", "will", "would",
        ] {
            let sent = modal_clause(lemma);
            assert_eq!(sent.modals.len(), 1, "lemma {lemma:?}");
            assert_eq!(sent.modals[0].aux_id, 2, "lemma {lemma:?}");
            assert_eq!(sent.modals[0].aux_lemma, lemma);
            assert_eq!(sent.modals[0].head_id, 3, "lemma {lemma:?}");
            assert_eq!(
                sent.root_mood, None,
                "the modal carries no Mood, lemma {lemma:?}"
            );
        }
    }

    #[test]
    fn modal_chain_reports_its_modal_with_the_content_verb_head() {
        // "It might have been done." The potential surface form as a
        // multi-auxiliary chain: `might`, `have`, `been` all attach to
        // `done`. Only `might` is in the closed class; the chain
        // reports exactly its modals, and the head is the content
        // verb, not the next auxiliary.
        let sent = Sentence::new(
            "It might have been done.".to_string(),
            vec![
                make_token(1, "It", "PRON", "nsubj:pass", 5),
                make_token_feats(2, "might", "AUX", "aux", 5, "VerbForm=Fin"),
                make_token_feats(3, "have", "AUX", "aux", 5, "VerbForm=Inf"),
                make_token_feats(4, "been", "AUX", "aux:pass", 5, "Tense=Past|VerbForm=Part"),
                make_token_feats(
                    5,
                    "done",
                    "VERB",
                    "root",
                    0,
                    "Tense=Past|VerbForm=Part|Voice=Pass",
                ),
                make_token(6, ".", "PUNCT", "punct", 5),
            ],
        );
        assert_eq!(sent.modals.len(), 1);
        assert_eq!(sent.modals[0].aux_id, 2);
        assert_eq!(sent.modals[0].aux_lemma, "might");
        assert_eq!(sent.modals[0].head_id, 5);
        assert_eq!(sent.root_mood, None, "participle root carries no Mood");
    }

    #[test]
    fn modal_coordinated_clauses_report_both() {
        // "He can stay, and she may leave." Two clauses, two modals,
        // each attached to its own verb.
        let sent = Sentence::new(
            "He can stay, and she may leave.".to_string(),
            vec![
                make_token(1, "He", "PRON", "nsubj", 3),
                make_token_feats(2, "can", "AUX", "aux", 3, "VerbForm=Fin"),
                make_token_feats(3, "stay", "VERB", "root", 0, "VerbForm=Inf"),
                make_token(4, ",", "PUNCT", "punct", 8),
                make_token(5, "and", "CCONJ", "cc", 8),
                make_token(6, "she", "PRON", "nsubj", 8),
                make_token_feats(7, "may", "AUX", "aux", 8, "VerbForm=Fin"),
                make_token_feats(8, "leave", "VERB", "conj", 3, "VerbForm=Inf"),
                make_token(9, ".", "PUNCT", "punct", 3),
            ],
        );
        let found: Vec<(&str, usize)> = sent
            .modals
            .iter()
            .map(|m| (m.aux_lemma.as_str(), m.head_id))
            .collect();
        assert_eq!(found, vec![("can", 3), ("may", 8)]);
    }

    #[test]
    fn modal_promoted_to_root_under_vp_ellipsis_is_found() {
        // "Yes, you should." VP ellipsis promotes the modal to root
        // (verified live), so the `aux` relation is absent and the
        // `AUX` part of speech carries the match. head_id is 0, the
        // documented root convention. A clause whose root is a modal
        // is modalized, not a bare assertion.
        let sent = Sentence::new(
            "Yes, you should.".to_string(),
            vec![
                make_token(1, "Yes", "INTJ", "discourse", 4),
                make_token(2, ",", "PUNCT", "punct", 4),
                make_token(3, "you", "PRON", "nsubj", 4),
                make_token_feats(4, "should", "AUX", "root", 0, "VerbForm=Fin"),
                make_token(5, ".", "PUNCT", "punct", 4),
            ],
        );
        assert_eq!(sent.modals.len(), 1);
        assert_eq!(sent.modals[0].aux_lemma, "should");
        assert_eq!(sent.modals[0].head_id, 0);
        assert_eq!(sent.root_mood, None, "a root modal carries no Mood");
    }

    #[test]
    fn modal_coordinated_auxiliaries_report_both() {
        // "He can and will succeed." The live model parses the first
        // coordinated auxiliary as root and the second as `aux` on
        // the shared verb, so the first is found by its `AUX` part
        // of speech and the second by its relation.
        let sent = Sentence::new(
            "He can and will succeed.".to_string(),
            vec![
                make_token(1, "He", "PRON", "nsubj", 2),
                make_token_feats(2, "can", "AUX", "root", 0, "VerbForm=Fin"),
                make_token(3, "and", "CCONJ", "cc", 5),
                make_token_feats(4, "will", "AUX", "aux", 5, "VerbForm=Fin"),
                make_token_feats(5, "succeed", "VERB", "conj", 2, "VerbForm=Inf"),
                make_token(6, ".", "PUNCT", "punct", 2),
            ],
        );
        let found: Vec<(&str, usize)> = sent
            .modals
            .iter()
            .map(|m| (m.aux_lemma.as_str(), m.head_id))
            .collect();
        assert_eq!(found, vec![("can", 0), ("will", 5)]);
        assert_eq!(sent.root_mood, None);
    }

    #[test]
    fn modal_second_coordinated_auxiliary_as_conj_is_found() {
        // "He can, and he will, succeed." With full clausal
        // coordination the live model parses the first modal as root
        // and the second as `conj`, so both are found by the `AUX`
        // part of speech and neither carries `aux`.
        let sent = Sentence::new(
            "He can, and he will, succeed.".to_string(),
            vec![
                make_token(1, "He", "PRON", "nsubj", 2),
                make_token_feats(2, "can", "AUX", "root", 0, "VerbForm=Fin"),
                make_token(3, ",", "PUNCT", "punct", 6),
                make_token(4, "and", "CCONJ", "cc", 6),
                make_token(5, "he", "PRON", "nsubj", 6),
                make_token_feats(6, "will", "AUX", "conj", 2, "VerbForm=Fin"),
                make_token(7, ",", "PUNCT", "punct", 8),
                make_token_feats(8, "succeed", "VERB", "conj", 2, "Mood=Ind|VerbForm=Fin"),
                make_token(9, ".", "PUNCT", "punct", 2),
            ],
        );
        let found: Vec<(&str, usize)> = sent
            .modals
            .iter()
            .map(|m| (m.aux_lemma.as_str(), m.head_id))
            .collect();
        assert_eq!(found, vec![("can", 0), ("will", 2)]);
        assert_eq!(sent.root_mood, None, "a root modal carries no Mood");
    }

    #[test]
    fn root_mood_of_a_copular_clause_rides_the_cop() {
        // "The sky is blue." The live model roots the predicate ADJ
        // and parks Mood=Ind on the `cop` child, so the detector reads
        // the mood off the copula, and reports the root's id.
        let sent = Sentence::new(
            "The sky is blue.".to_string(),
            vec![
                make_token(1, "The", "DET", "det", 2),
                make_token(2, "sky", "NOUN", "nsubj", 4),
                make_token_feats(
                    3,
                    "is",
                    "AUX",
                    "cop",
                    4,
                    "Mood=Ind|Number=Sing|Person=3|Tense=Pres|VerbForm=Fin",
                ),
                make_token_feats(4, "blue", "ADJ", "root", 0, "Degree=Pos"),
                make_token(5, ".", "PUNCT", "punct", 4),
            ],
        );
        assert!(sent.modals.is_empty(), "the copula be is outside the class");
        assert_eq!(mood_of(&sent), Some((4, "Ind")));
    }

    #[test]
    fn root_mood_of_do_support_rides_the_aux() {
        // "He did leave." Do-support roots the infinitive and parks
        // Mood=Ind on the `aux` child, whose lemma is outside the
        // modal class.
        let sent = Sentence::new(
            "He did leave.".to_string(),
            vec![
                make_token(1, "He", "PRON", "nsubj", 3),
                make_token_feats(2, "do", "AUX", "aux", 3, "Mood=Ind|Tense=Past|VerbForm=Fin"),
                make_token_feats(3, "leave", "VERB", "root", 0, "VerbForm=Inf"),
                make_token(4, ".", "PUNCT", "punct", 3),
            ],
        );
        assert!(sent.modals.is_empty());
        assert_eq!(mood_of(&sent), Some((3, "Ind")));
    }

    #[test]
    fn modal_contraction_fires_on_its_lemma() {
        // "They won't stop." The tokenizer splits the contraction and
        // the model lemmatizes `wo` to `will` (verified live), so the
        // lemma match fires; the `not` is the negation primitive's
        // finding, not this one's.
        let sent = Sentence::new(
            "They won't stop.".to_string(),
            vec![
                make_token(1, "They", "PRON", "nsubj", 4),
                make_token_feats(2, "will", "AUX", "aux", 4, "VerbForm=Fin"),
                make_token(3, "not", "PART", "advmod", 4),
                make_token_feats(4, "stop", "VERB", "root", 0, "VerbForm=Inf"),
                make_token(5, ".", "PUNCT", "punct", 4),
            ],
        );
        assert_eq!(sent.modals.len(), 1);
        assert_eq!(sent.modals[0].aux_lemma, "will");
        assert_eq!(sent.negations.len(), 1, "the split not still fires");
    }

    #[test]
    fn modal_no_false_positive_on_lexical_will() {
        // "The will was signed yesterday." The noun `will` shares the
        // lemma but never the `aux` relation.
        let sent = Sentence::new(
            "The will was signed yesterday.".to_string(),
            vec![
                make_token(1, "The", "DET", "det", 2),
                make_token(2, "will", "NOUN", "nsubj:pass", 4),
                make_token_feats(
                    3,
                    "was",
                    "AUX",
                    "aux:pass",
                    4,
                    "Mood=Ind|Tense=Past|VerbForm=Fin",
                ),
                make_token_feats(
                    4,
                    "signed",
                    "VERB",
                    "root",
                    0,
                    "Tense=Past|VerbForm=Part|Voice=Pass",
                ),
                make_token(5, "yesterday", "NOUN", "obl:tmod", 4),
                make_token(6, ".", "PUNCT", "punct", 4),
            ],
        );
        assert!(sent.modals.is_empty());
        // In the indicative passive, Mood=Ind rides the `aux:pass`
        // child of the participle root.
        assert_eq!(mood_of(&sent), Some((4, "Ind")));
    }

    #[test]
    fn a_modalized_directive_has_no_root_mood() {
        // "You must complete the form by Friday." The directive
        // surface form with a modal: `must` is reported, and neither
        // the infinitive root nor the modal carries a Mood. Whether
        // `must` is obligation or inference here is the consumer's
        // reading.
        let sent = Sentence::new(
            "You must complete the form by Friday.".to_string(),
            vec![
                make_token(1, "You", "PRON", "nsubj", 3),
                make_token_feats(2, "must", "AUX", "aux", 3, "VerbForm=Fin"),
                make_token_feats(3, "complete", "VERB", "root", 0, "VerbForm=Inf"),
                make_token(4, "the", "DET", "det", 5),
                make_token(5, "form", "NOUN", "obj", 3),
                make_token(6, "by", "ADP", "case", 7),
                make_token(7, "Friday", "PROPN", "obl", 3),
                make_token(8, ".", "PUNCT", "punct", 3),
            ],
        );
        assert_eq!(sent.modals.len(), 1);
        assert_eq!(sent.modals[0].aux_lemma, "must");
        assert_eq!(sent.root_mood, None);
    }

    #[test]
    fn root_mood_reads_the_indicative_root() {
        // "The committee approved it." The root carries Mood=Ind and no
        // modal attaches.
        let sent = Sentence::new(
            "The committee approved it.".to_string(),
            vec![
                make_token(1, "The", "DET", "det", 2),
                make_token(2, "committee", "NOUN", "nsubj", 3),
                make_token_feats(
                    3,
                    "approved",
                    "VERB",
                    "root",
                    0,
                    "Mood=Ind|Tense=Past|VerbForm=Fin",
                ),
                make_token(4, "it", "PRON", "obj", 3),
                make_token(5, ".", "PUNCT", "punct", 3),
            ],
        );
        assert!(sent.modals.is_empty());
        assert_eq!(mood_of(&sent), Some((3, "Ind")));
    }

    #[test]
    fn root_mood_reads_the_imperative_root() {
        // "Complete the form by Friday." The bare directive surface
        // form: Mood=Imp on the root, with no modal involved.
        let sent = Sentence::new(
            "Complete the form by Friday.".to_string(),
            vec![
                make_token_feats(1, "Complete", "VERB", "root", 0, "Mood=Imp|VerbForm=Fin"),
                make_token(2, "the", "DET", "det", 3),
                make_token(3, "form", "NOUN", "obj", 1),
                make_token(4, "by", "ADP", "case", 5),
                make_token(5, "Friday", "PROPN", "obl", 1),
                make_token(6, ".", "PUNCT", "punct", 1),
            ],
        );
        assert!(sent.modals.is_empty());
        assert_eq!(mood_of(&sent), Some((1, "Imp")));
    }

    #[test]
    fn subordinate_modal_is_reported_and_leaves_the_root_mood() {
        // "I know it might rain." The detector reads the root clause:
        // `know` carries Mood=Ind and `might` attaches to the
        // subordinate `rain`, so the root mood is the matrix's while
        // the modal is still reported with its own head. The consumer
        // sees both facts and does its own reading.
        let sent = Sentence::new(
            "I know it might rain.".to_string(),
            vec![
                make_token(1, "I", "PRON", "nsubj", 2),
                make_token_feats(
                    2,
                    "know",
                    "VERB",
                    "root",
                    0,
                    "Mood=Ind|Tense=Pres|VerbForm=Fin",
                ),
                make_token(3, "it", "PRON", "nsubj", 5),
                make_token_feats(4, "might", "AUX", "aux", 5, "VerbForm=Fin"),
                make_token(5, "rain", "NOUN", "ccomp", 2),
                make_token(6, ".", "PUNCT", "punct", 5),
            ],
        );
        assert_eq!(sent.modals.len(), 1);
        assert_eq!(sent.modals[0].aux_lemma, "might");
        assert_eq!(sent.modals[0].head_id, 5);
        assert_eq!(mood_of(&sent), Some((2, "Ind")));
    }

    /// The root mood as `(root_id, mood)`, for terse assertions.
    fn mood_of(sent: &Sentence) -> Option<(usize, &str)> {
        sent.root_mood
            .as_ref()
            .map(|m| (m.root_id, m.mood.as_str()))
    }

    #[test]
    fn modals_and_root_mood_cross_the_wire() {
        // The fields cross the wire as data (RFC-0008): the modal with
        // its lemma, and an absent root mood as null rather than a
        // missing key.
        let sent = modal_clause("might");
        let json = serde_json::to_value(&sent).unwrap();
        assert_eq!(json["modals"][0]["aux_lemma"], "might");
        assert!(json["root_mood"].is_null());
        assert!(json.as_object().unwrap().contains_key("root_mood"));
        let indicative = Sentence::new(
            "It rains.".to_string(),
            vec![
                make_token(1, "It", "PRON", "expl", 2),
                make_token_feats(2, "rains", "VERB", "root", 0, "Mood=Ind|VerbForm=Fin"),
                make_token(3, ".", "PUNCT", "punct", 2),
            ],
        );
        let json = serde_json::to_value(&indicative).unwrap();
        assert_eq!(
            json["root_mood"],
            serde_json::json!({"root_id": 2, "mood": "Ind"})
        );
    }

    #[test]
    fn features_cross_the_wire_as_a_map() {
        let tok = token_with_feats("Mood=Ind|Tense=Pres");
        let json = serde_json::to_value(&tok).unwrap();
        assert_eq!(
            json["features"],
            serde_json::json!({"Mood": "Ind", "Tense": "Pres"})
        );
        assert!(json.get("feats").is_none(), "no raw string beside the map");
        let empty = serde_json::to_value(token_with_feats("_")).unwrap();
        assert_eq!(empty["features"], serde_json::json!({}));
    }

    // Reporting and root-adverbial fixtures mirror live UDPipe parses
    // (verified 2026-08-21): the complement head attaches as `ccomp`
    // to the reporting verb, the subject as `nsubj` (or `nsubj:pass`)
    // to the same verb, and a sentence-scope adverb as `advmod` to
    // the root.

    #[test]
    fn clausal_complement_self_attribution() {
        // "We show that the method works."
        let sent = Sentence::new(
            "We show that the method works.".to_string(),
            vec![
                make_token(1, "We", "PRON", "nsubj", 2),
                make_token(2, "show", "VERB", "root", 0),
                make_token(3, "that", "SCONJ", "mark", 6),
                make_token(4, "the", "DET", "det", 5),
                make_token(5, "method", "NOUN", "nsubj", 6),
                make_token(6, "works", "VERB", "ccomp", 2),
                make_token(7, ".", "PUNCT", "punct", 2),
            ],
        );
        assert_eq!(sent.clausal_complements.len(), 1);
        let r = &sent.clausal_complements[0];
        assert_eq!(r.verb_id, 2);
        assert_eq!(r.verb_lemma, "show");
        assert_eq!(r.ccomp_id, 6);
        // The complement's own subject ("method", nsubj of token 6)
        // is not the matrix subject; only the verb's child qualifies.
        assert_eq!(r.subject_id, Some(1));
        assert_eq!(r.subject_lemma.as_deref(), Some("we"));
    }

    #[test]
    fn clausal_complement_other_attribution() {
        // "Smith reported that the effect vanished."
        let sent = Sentence::new(
            "Smith reported that the effect vanished.".to_string(),
            vec![
                make_token(1, "Smith", "PROPN", "nsubj", 2),
                make_token(2, "reported", "VERB", "root", 0),
                make_token(3, "that", "SCONJ", "mark", 6),
                make_token(4, "the", "DET", "det", 5),
                make_token(5, "effect", "NOUN", "nsubj", 6),
                make_token(6, "vanished", "VERB", "ccomp", 2),
                make_token(7, ".", "PUNCT", "punct", 2),
            ],
        );
        assert_eq!(sent.clausal_complements.len(), 1);
        let r = &sent.clausal_complements[0];
        assert_eq!((r.verb_id, r.ccomp_id), (2, 6));
        assert_eq!(r.verb_lemma, "reported");
        assert_eq!(r.subject_id, Some(1));
    }

    #[test]
    fn clausal_complement_impersonal() {
        // "These results suggest that the mechanism is shared."
        // The complement is passive; its `nsubj:pass` belongs to the
        // complement head, not to the reporting verb.
        let sent = Sentence::new(
            "These results suggest that the mechanism is shared.".to_string(),
            vec![
                make_token(1, "These", "DET", "det", 2),
                make_token(2, "results", "NOUN", "nsubj", 3),
                make_token(3, "suggest", "VERB", "root", 0),
                make_token(4, "that", "SCONJ", "mark", 8),
                make_token(5, "the", "DET", "det", 6),
                make_token(6, "mechanism", "NOUN", "nsubj:pass", 8),
                make_token(7, "is", "AUX", "aux:pass", 8),
                make_token(8, "shared", "VERB", "ccomp", 3),
                make_token(9, ".", "PUNCT", "punct", 3),
            ],
        );
        assert_eq!(sent.clausal_complements.len(), 1);
        let r = &sent.clausal_complements[0];
        assert_eq!((r.verb_id, r.ccomp_id), (3, 8));
        assert_eq!(r.subject_id, Some(2));
        assert_eq!(r.subject_lemma.as_deref(), Some("results"));
    }

    #[test]
    fn clausal_complement_survives_upstream_split_with_no_subject() {
        // Known upstream defect, recorded not fixed: UDPipe splits
        // "Smith et al. reported that the effect vanished." into two
        // sentences at the period in "et al." (verified live,
        // 2026-08-21). The reporting verb keeps its `ccomp` but its
        // attribution is stranded in the previous sentence, so the
        // construction reports with no subject. This test pins the
        // detector's behaviour on that second fragment.
        let sent = Sentence::new(
            "reported that the effect vanished.".to_string(),
            vec![
                make_token(1, "reported", "VERB", "root", 0),
                make_token(2, "that", "SCONJ", "mark", 5),
                make_token(3, "the", "DET", "det", 4),
                make_token(4, "effect", "NOUN", "nsubj", 5),
                make_token(5, "vanished", "VERB", "ccomp", 1),
                make_token(6, ".", "PUNCT", "punct", 1),
            ],
        );
        assert_eq!(sent.clausal_complements.len(), 1);
        let r = &sent.clausal_complements[0];
        assert_eq!((r.verb_id, r.ccomp_id), (1, 5));
        assert_eq!(r.subject_id, None);
        assert_eq!(r.subject_lemma, None);
    }

    #[test]
    fn clausal_complement_no_fire_without_ccomp() {
        // "Smith reported a similar finding." takes a plain object,
        // not a clausal complement; the construction is absent.
        let sent = Sentence::new(
            "Smith reported a similar finding.".to_string(),
            vec![
                make_token(1, "Smith", "PROPN", "nsubj", 2),
                make_token(2, "reported", "VERB", "root", 0),
                make_token(3, "a", "DET", "det", 5),
                make_token(4, "similar", "ADJ", "amod", 5),
                make_token(5, "finding", "NOUN", "obj", 2),
                make_token(6, ".", "PUNCT", "punct", 2),
            ],
        );
        assert!(sent.clausal_complements.is_empty());
        assert!(active_sentence().clausal_complements.is_empty());
    }

    #[test]
    fn clausal_complement_no_fire_on_non_verb_head() {
        // "I am sure that it works." parses the `ccomp` under the
        // adjective `sure`. The plan defines the reporting
        // construction as verbal, so a non-VERB head is outside it
        // by decision, not omission.
        let sent = Sentence::new(
            "I am sure that it works.".to_string(),
            vec![
                make_token(1, "I", "PRON", "nsubj", 3),
                make_token(2, "am", "AUX", "cop", 3),
                make_token(3, "sure", "ADJ", "root", 0),
                make_token(4, "that", "SCONJ", "mark", 6),
                make_token(5, "it", "PRON", "nsubj", 6),
                make_token(6, "works", "VERB", "ccomp", 3),
                make_token(7, ".", "PUNCT", "punct", 3),
            ],
        );
        assert!(sent.clausal_complements.is_empty());
    }

    #[test]
    fn root_adverbial_detects_sentence_scope_advmod() {
        // "Reportedly, the deal closed."
        let sent = Sentence::new(
            "Reportedly, the deal closed.".to_string(),
            vec![
                make_token(1, "Reportedly", "ADV", "advmod", 5),
                make_token(2, ",", "PUNCT", "punct", 5),
                make_token(3, "the", "DET", "det", 4),
                make_token(4, "deal", "NOUN", "nsubj", 5),
                make_token(5, "closed", "VERB", "root", 0),
                make_token(6, ".", "PUNCT", "punct", 5),
            ],
        );
        assert_eq!(sent.root_adverbials.len(), 1);
        assert_eq!(sent.root_adverbials[0].adv_id, 1);
        assert_eq!(sent.root_adverbials[0].adv_lemma, "reportedly");
    }

    #[test]
    fn root_adverbial_reports_manner_adverbs_too() {
        // "The team quickly shipped." puts `quickly` on the same
        // root-attached `advmod` arc as `reportedly`. The dependency
        // graph does not distinguish sentence scope from manner, so
        // the detector reports the arc and the caller's lexicon
        // decides the evidential reading (EP-0007 M4: construction, not
        // category).
        let sent = Sentence::new(
            "The team quickly shipped.".to_string(),
            vec![
                make_token(1, "The", "DET", "det", 2),
                make_token(2, "team", "NOUN", "nsubj", 4),
                make_token(3, "quickly", "ADV", "advmod", 4),
                make_token(4, "shipped", "VERB", "root", 0),
                make_token(5, ".", "PUNCT", "punct", 4),
            ],
        );
        assert_eq!(sent.root_adverbials.len(), 1);
        assert_eq!(sent.root_adverbials[0].adv_lemma, "quickly");
        // The caller's lexicon is where the evidential reading lives.
        assert!(
            sent.root_adverbials_in(&["reportedly", "allegedly"])
                .is_empty()
        );
    }

    #[test]
    fn root_adverbial_ignores_non_root_attachment() {
        // "She says he ran quickly." attaches `quickly` to `ran`
        // (the complement head), not the root, so no root adverbial;
        // the reporting construction on `says` still fires.
        let sent = Sentence::new(
            "She says he ran quickly.".to_string(),
            vec![
                make_token(1, "She", "PRON", "nsubj", 2),
                make_token(2, "says", "VERB", "root", 0),
                make_token(3, "he", "PRON", "nsubj", 4),
                make_token(4, "ran", "VERB", "ccomp", 2),
                make_token(5, "quickly", "ADV", "advmod", 4),
                make_token(6, ".", "PUNCT", "punct", 2),
            ],
        );
        assert!(sent.root_adverbials.is_empty());
        assert_eq!(sent.clausal_complements.len(), 1);
    }

    #[test]
    fn root_adverbial_overlaps_negation_by_design() {
        // "The plan is not ready." carries `not` as `advmod` on the
        // root, so it appears in both `negations` and
        // `root_adverbials`. Both report arcs; neither assigns a
        // reading; the overlap is intentional.
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
        assert_eq!(sent.root_adverbials.len(), 1);
        assert_eq!(sent.root_adverbials[0].adv_id, 4);
    }

    #[test]
    fn clausal_complements_in_filters_by_caller_lexicon() {
        // matra ships no evidential lexicon (open class); the caller
        // supplies one and the filter is a view over crossing data.
        let sent = Sentence::new(
            "Smith reported that the effect vanished.".to_string(),
            vec![
                make_token(1, "Smith", "PROPN", "nsubj", 2),
                make_token(2, "reported", "VERB", "root", 0),
                make_token(3, "that", "SCONJ", "mark", 6),
                make_token(4, "the", "DET", "det", 5),
                make_token(5, "effect", "NOUN", "nsubj", 6),
                make_token(6, "vanished", "VERB", "ccomp", 2),
                make_token(7, ".", "PUNCT", "punct", 2),
            ],
        );
        // make_token lemmatizes to the lowercased text, so the
        // caller's lexicon here uses the surface-derived lemma.
        assert_eq!(
            sent.clausal_complements_in(&["reported", "suggest"]).len(),
            1
        );
        assert!(sent.clausal_complements_in(&["say", "claim"]).is_empty());
    }

    #[test]
    fn clausal_complements_and_root_adverbials_cross_the_wire() {
        // The fields cross the wire as data (RFC-0008).
        let sent = Sentence::new(
            "Reportedly, it works.".to_string(),
            vec![
                make_token(1, "Reportedly", "ADV", "advmod", 3),
                make_token(2, "it", "PRON", "nsubj", 3),
                make_token(3, "works", "VERB", "root", 0),
                make_token(4, ".", "PUNCT", "punct", 3),
            ],
        );
        let json = serde_json::to_value(&sent).unwrap();
        assert_eq!(json["root_adverbials"][0]["adv_lemma"], "reportedly");
        assert!(json["clausal_complements"].as_array().unwrap().is_empty());
    }

    /// The derived counts cross the wire beside the tokens, so a
    /// binding reads them rather than recounting.
    #[test]
    fn sentence_counts_cross_the_wire() {
        let json = serde_json::to_value(passive_sentence()).unwrap();
        assert_eq!(json["word_count"], 7);
        assert_eq!(json["passive"], true);
        assert_eq!(json["tree_depth"], 2);
    }

    #[test]
    fn tree_depth_computes_max() {
        let sent = active_sentence();
        assert!(sent.tree_depth.is_some_and(|d| d > 0));
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
        assert_eq!(sent.tree_depth, Some(2));
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
        assert_eq!(sent.word_count, 0);
        assert!(!sent.passive);
        assert_eq!(sent.tree_depth, Some(0));
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
        assert_eq!(sent.tree_depth, Some(24));
    }

    #[test]
    fn tree_depth_1000_chain_returns_999_in_bounded_time() {
        // Verifies O(n) complexity: a 1000-token chain returns 999, fast.
        // The pre-fix O(n^2) impl would do ~10^6 inner finds; this is well
        // under 50ms on commodity hardware.
        let tokens: Vec<Token> = (1..=1000)
            .map(|i| make_token(i, "x", "NOUN", "dep", if i == 1 { 0 } else { i - 1 }))
            .collect();
        let start = std::time::Instant::now();
        let depth = measure_tree_depth(&tokens);
        let elapsed = start.elapsed();
        assert_eq!(depth, Some(999));
        assert_eq!(straight_chain(1000).tree_depth, Some(999));
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
                Document::new(Provenance::for_test(), Vec::new()),
            )),
            Err(DocumentError::new(
                Some(PathBuf::from("b.md")),
                Error::ParseFailed("bad".to_string()),
            )),
            Ok(CorpusEntry::new(
                None,
                Document::new(Provenance::for_test(), Vec::new()),
            )),
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

    /// The whole extension table, in one place, because it is the one
    /// place. Both the file source and the command line read it, so a
    /// change here changes both routes at once and this test is what
    /// says which change was intended.
    #[test]
    fn format_from_path_reads_the_extension() {
        for (path, expected) in [
            ("notes.md", Format::Markdown),
            ("notes.markdown", Format::Markdown),
            ("report.pdf", Format::Pdf),
            ("report.docx", Format::Docx),
            ("notes.txt", Format::PlainText),
            ("notes.rs", Format::PlainText),
            ("/tmp/deep/notes.md", Format::Markdown),
            ("archive.md.gz", Format::PlainText),
        ] {
            assert_eq!(Format::from_path(path), expected, "{path}");
        }
    }

    /// No extension, and no name at all, are plain text rather than an
    /// error: the format picks a decomposer, and every byte sequence has
    /// one that will read it.
    #[test]
    fn format_from_path_falls_back_to_plain_text() {
        assert_eq!(Format::from_path("README"), Format::PlainText);
        assert_eq!(Format::from_path("<stdin>"), Format::PlainText);
        assert_eq!(Format::from_path(".gitignore"), Format::PlainText);
        assert_eq!(Format::from_path(""), Format::PlainText);
    }

    #[test]
    fn tree_depth_is_none_on_a_cycle() {
        // A -> B -> A is a cycle. The depth is `None`, a value a reader
        // must handle, rather than a sentinel number that reads as a
        // very deep tree. The magic-< 20 ceiling before the visited set
        // silently truncated to 20, protective by accident, not by
        // design (and silently wrong on legitimate deep parses).
        let sent = Sentence::new(
            "cyclic".to_string(),
            vec![
                make_token(1, "A", "NOUN", "dep", 2),
                make_token(2, "B", "NOUN", "dep", 1),
            ],
        );
        assert_eq!(sent.tree_depth, None);
    }

    /// A token rooted in a cycle, reached after the cycle was memoized,
    /// still makes the sentence's depth `None`: the cycle is not hidden
    /// behind a well-formed chain that leads into it.
    #[test]
    fn tree_depth_is_none_when_a_chain_leads_into_a_cycle() {
        let sent = Sentence::new(
            "cyclic tail".to_string(),
            vec![
                make_token(1, "A", "NOUN", "dep", 2),
                make_token(2, "B", "NOUN", "dep", 1),
                make_token(3, "C", "NOUN", "dep", 1),
            ],
        );
        assert_eq!(sent.tree_depth, None);
    }

    #[test]
    fn provenance_carries_this_builds_versions_and_the_format_vocabulary() {
        let p = Provenance::for_test();
        assert_eq!(p.schema, SCHEMA_VERSION);
        assert_eq!(p.matra, env!("CARGO_PKG_VERSION"));
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["format"], "plain_text");
        assert!(json["source"]["path"].is_null());
        assert_eq!(
            serde_json::to_value(Format::Markdown).unwrap(),
            serde_json::json!("markdown")
        );
    }

    /// A path that is not UTF-8 serializes lossily rather than failing
    /// the whole document over its name.
    #[cfg(unix)]
    #[test]
    fn a_non_utf8_source_path_serializes_lossily() {
        use std::os::unix::ffi::OsStrExt;
        let path = PathBuf::from(std::ffi::OsStr::from_bytes(b"bad\xffname.md"));
        let source = ProvenanceSource::new(Some(path), 3, "abc");
        let json = serde_json::to_value(&source).expect("serializes");
        assert_eq!(json["path"], "bad\u{fffd}name.md");
    }

    /// The main gate's label is `"input"`, and the message once read
    /// "input input too large" because the format put the label in
    /// front of the word it already is. Every other gate keeps its label
    /// as a qualifier.
    #[test]
    fn input_too_large_names_the_main_gate_once() {
        let main_gate = Error::InputTooLarge {
            limit: 8,
            actual: 9,
            what: "input",
        };
        assert_eq!(main_gate.to_string(), "input too large: 9 > limit 8");

        let extractor_gate = Error::InputTooLarge {
            limit: 2000,
            actual: 2001,
            what: "tfidf",
        };
        assert_eq!(
            extractor_gate.to_string(),
            "tfidf input too large: 2001 > limit 2000"
        );
    }

    /// The kind vocabulary is a contract every crust reads, so it is
    /// pinned here as well as in `spec/tests/corpus/items.json`: one
    /// string per variant, all seven distinct.
    #[test]
    fn error_kind_names_every_variant_distinctly() {
        let cases: [(Error, &str); 7] = [
            (
                Error::ModelNotFound(PathBuf::from("m.udpipe")),
                "model_not_found",
            ),
            (
                Error::ModelInvalid("truncated".to_string()),
                "model_invalid",
            ),
            (Error::ParseFailed("no parse".to_string()), "parse_failed"),
            (
                Error::InputTooLarge {
                    limit: 1,
                    actual: 2,
                    what: "input",
                },
                "input_too_large",
            ),
            (Error::UnsupportedFormat(Format::Pdf), "unsupported_format"),
            (
                Error::InvalidInput("bad argument".to_string()),
                "invalid_input",
            ),
            (
                Error::Io(std::io::Error::from(std::io::ErrorKind::NotFound)),
                "io",
            ),
        ];

        let mut seen: Vec<&'static str> = Vec::new();
        for (error, expected) in &cases {
            assert_eq!(error.kind(), *expected, "{error}");
            assert!(!seen.contains(expected), "duplicate kind {expected}");
            seen.push(expected);
        }
    }
}
