//! Domain types. The core model that everything else depends on.
//! Dependencies are bounded to serde, thiserror, and std.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Resource bounds
// ---------------------------------------------------------------------------

/// Every bound matra holds its work to, in one place: the caps on what it
/// reads, the caps each outcome method checks before it runs, and the
/// minimum a paragraph needs before a metric is measured on it.
///
/// Published so a caller can size its work before the call rather than
/// learn the bound from a refusal: split a long input under
/// [`limits::MAX_INPUT_BYTES`], or chunk a document's sentences under the
/// cap of the method it selects ([`limits::MAX_TFIDF_SENTENCES`] for
/// `SummaryMethod::TfIdf`, [`limits::MAX_RAKE_TOKENS`] for
/// `KeyphraseMethod::Rake`, and so on). Each method cap is named for the
/// method a caller names, as the selectors are.
///
/// Constants, not settings: no configuration, argument or environment
/// variable changes any of them. They are this release's values and may
/// change in any 0.x release, so read them from here rather than copying
/// the number. A cap that is exceeded returns [`Error::InputTooLarge`],
/// whose `limit` detail is the constant here and whose `what` detail
/// names the gate. A paragraph under a minimum, or a paragraph over
/// [`limits::MAX_COMPRESSION_PARAGRAPH_BYTES`], is not an error: its metric slot
/// stays `None`.
///
/// Each cap is defined once and read where it applies. Two caps with the
/// same value stay two constants where their costs differ, so tuning one
/// does not move the other.
pub mod limits {
    /// The most text one input may hold, in bytes: 8 MiB.
    ///
    /// The gate whose `what` is `"input"`. `Engine::analyze` and
    /// `Engine::analyze_all` check it on every `Input` (text, bytes, a
    /// file, blocks), on the bytes handed in and again on the analysed
    /// text a reader produces from them; reading a file checks it on the
    /// file's size before reading (`"file_source"`), and the command line
    /// on stdin as the bytes arrive.
    ///
    /// 8 MiB holds book-length English (a typical novel is ~1.5 MiB,
    /// 200k words) with headroom. Beyond it the parser's intermediate
    /// memory grows past safe limits on a typical workstation (UDPipe's
    /// per-token allocations cross ~1 GiB resident at this size).
    pub const MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;

    /// The largest configuration file read, in bytes: 64 KiB.
    ///
    /// The gate whose `what` is `"config_file"`, checked on the file's
    /// size before any read. A config file is small by construction;
    /// anything larger is a mistake or an attack, and either way it is
    /// not read into memory first and rejected second.
    pub const MAX_CONFIG_FILE_BYTES: usize = 64 * 1024;

    /// The most bytes read from any one model download: 64 MiB.
    ///
    /// The gates whose `what` is `"udpipe_download"` (the parsing model)
    /// and `"embedding_download"` (each embedding artifact). One number,
    /// because it answers one question: how much a redirected or
    /// misbehaving server can make the process hold. The pinned artifacts
    /// are 16.3 MB and at most 30.2 MB, so this is headroom for a later
    /// version that still costs a bounded read. The read stops one byte
    /// past it.
    pub const MAX_DOWNLOAD_BYTES: usize = 64 * 1024 * 1024;

    /// The most sentences a TF-IDF summary ranks: 2,000.
    ///
    /// The gate whose `what` is `"tfidf"`. TF-IDF scores each sentence in
    /// linear time; the working set is the per-sentence term counts, about
    /// 60k entries (~2 MiB) at this size. It shares the value with
    /// [`MAX_TEXTRANK_SENTENCES`] but not the cost model, so it is a
    /// separate constant: tuning TextRank does not move it.
    pub const MAX_TFIDF_SENTENCES: usize = 2000;

    /// The most sentences a TextRank summary ranks: 2,000.
    ///
    /// The gate whose `what` is `"textrank"`. TextRank builds a dense
    /// sentence similarity matrix, quadratic in sentences: about 32 MB of
    /// `f64` at this size, the ceiling for unattended use.
    pub const MAX_TEXTRANK_SENTENCES: usize = 2000;

    /// The most sentences, or vectors, clustered: TextRank's bound.
    ///
    /// The gate whose `what` is `"clusters"`, checked on a document's
    /// sentences before anything is embedded and on the vectors
    /// themselves. Clustering builds the same quadratic graph TextRank
    /// does, so it is defined as [`MAX_TEXTRANK_SENTENCES`] rather than as
    /// a second copy of its value, and the two cannot drift apart.
    pub const MAX_CLUSTER_SENTENCES: usize = MAX_TEXTRANK_SENTENCES;

    /// The most tokens RAKE extracts keyphrases from: 200,000, counted
    /// across every sentence, punctuation included.
    ///
    /// The gate whose `what` is `"rake"`. RAKE's co-occurrence map grows
    /// with tokens, not sentences (a chat log can hold 50k one-token
    /// sentences), so the cap is on tokens: at this size the candidate map
    /// holds at most ~50k entries, about 3 MiB.
    pub const MAX_RAKE_TOKENS: usize = 200_000;

    /// The most tokens YAKE extracts keyphrases from: 200,000, counted as
    /// RAKE's are.
    ///
    /// The gate whose `what` is `"yake"`. YAKE's one- to three-word
    /// candidate map is bounded by token count: at this size at most ~600k
    /// entries, about 40 MiB. Its own constant, because its cost model is
    /// not RAKE's.
    pub const MAX_YAKE_TOKENS: usize = 200_000;

    /// The largest paragraph whose compression ratio is measured, in
    /// bytes: 256 KiB.
    ///
    /// Above it `compression_ratio` stays `None` rather than failing, to
    /// bound worst-case CPU on adversarial input. It matches the
    /// compressor's 256 KiB window, so one paragraph never costs more
    /// than one window's work.
    pub const MAX_COMPRESSION_PARAGRAPH_BYTES: usize = 256 * 1024;

    /// The fewest words a paragraph needs for a readability grade: 11
    /// (more than 10), counted by whitespace.
    ///
    /// Below it `readability_grade` stays `None`.
    pub const MIN_READABILITY_WORDS: usize = 11;

    /// The fewest words a paragraph needs for a compression ratio: 51
    /// (more than 50), counted by whitespace.
    ///
    /// Below it `compression_ratio` stays `None`.
    pub const MIN_COMPRESSION_WORDS: usize = 51;
}

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
/// terminal, so it hands the facts to whoever does: the constructors that
/// provision carry a `_with_notice` form that calls a closure once per
/// fetch, and only when the artifact is not already on disk.
///
/// That is `Engine::from_config_with_notice` for the parsing model, and
/// `Model2Vec::from_config_with_notice` for the embedding model, whose
/// three files are fetched and verified as one set and so are one notice.
///
/// Facts rather than a sentence. The caller decides the wording, the
/// stream, and whether to say anything at all.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ProvisionNotice {
    /// What is being fetched: the artifact's filename as it will appear
    /// in `destination`, or, for a model fetched as a set of files, the
    /// model's name (`potion-base-8M`).
    pub artifact: String,
    /// The artifact's pinned size in bytes, the set's total for a model
    /// fetched as a set. Known before the fetch because it is a constant
    /// in the source; the pinned digest decides whether what arrives is
    /// trusted.
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

/// What an [`Error::UnsupportedFormat`] message says could not be read,
/// and what would read it.
struct FormatSubject<'a>(Option<&'a Format>, Option<&'static str>);

impl std::fmt::Display for FormatSubject<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.0, self.1) {
            (Some(format), Some(feature)) => write!(
                f,
                "unsupported format: {}; enable the `{feature}` feature to read it",
                format.wire_name()
            ),
            (Some(format), None) => write!(
                f,
                "unsupported format: {}; no feature of this matra version reads it",
                format.wire_name()
            ),
            (None, _) => f.write_str(
                "unsupported format: the input is not UTF-8 text and carries no signature matra recognises",
            ),
        }
    }
}

/// All errors matra can produce. Matchable, not opaque.
///
/// Every channel reads a failure the same way: a stable [`kind`](Error::kind)
/// to branch on, a message for a person, and [`details`](Error::details),
/// every dynamic value in the message as named data. The serde form is
/// those three, `{kind, message, details}`, and is what the Python
/// binding and the JSON carry, so no reader has to parse a message.
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
        /// `"clusters"`, `"udpipe_download"`,
        /// `"embedding_download"` and `"config_file"`.
        what: &'static str,
    },
    /// The input is in a format this build cannot read: a format matra
    /// recognises but has no reader for here (`pdf`, `docx`), or bytes
    /// that are not UTF-8 text and carry no signature matra recognises.
    /// A text file with an extension matra does not know is not this
    /// error: it reads as plain text.
    #[error("{}", FormatSubject(.format.as_ref(), *.feature))]
    UnsupportedFormat {
        /// The format recognised, or `None` when the bytes are neither
        /// UTF-8 text nor a signature matra recognises.
        format: Option<Format>,
        /// The Cargo feature that would read `format`, when one exists.
        /// No feature of this version reads `pdf` or `docx`, so it is
        /// `None` for both today.
        feature: Option<&'static str>,
    },
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
    /// cannot carry a Rust enum across carries this string instead, in
    /// the serde form and on the Python exception, so every language
    /// names a failure the same way. `spec/tests/corpus/items.json` pins
    /// the list.
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
            Error::UnsupportedFormat { .. } => "unsupported_format",
            Error::InvalidInput(_) => "invalid_input",
            Error::Io(_) => "io",
        }
    }

    /// Every dynamic value in this error's message, as named data, so a
    /// caller acts on a failure without parsing its message.
    ///
    /// The keys per kind:
    ///
    /// | Kind | Keys |
    /// |---|---|
    /// | `model_not_found` | `path` |
    /// | `model_invalid`, `parse_failed`, `invalid_input` | `reason` |
    /// | `input_too_large` | `what`, `limit`, `actual` |
    /// | `unsupported_format` | `format` when one was recognised, `feature` when one would read it |
    /// | `io` | `io_kind` and `reason`; `path` when the failure concerns a path matra named; `url` and `retryable` when a download did not arrive |
    ///
    /// A key that does not apply is absent, never empty. A key, once a
    /// kind carries it, keeps appearing in later versions. `reason` is the
    /// variant's own text: the message a payload `String` holds, or the
    /// operating system's account of an I/O failure. `io_kind` names the
    /// I/O failure's class (`not_found`, `permission_denied`,
    /// `timed_out`, ...; [`std::io::ErrorKind`] in Rust), a set a reader
    /// treats as open. `retryable` is `true` when the same call can be made
    /// again as it is and may succeed: a download that did not arrive
    /// left nothing behind. Its absence makes no claim either way.
    ///
    /// ```
    /// # use matra::domain::{DetailValue, Error};
    /// let err = Error::InputTooLarge { limit: 8, actual: 9, what: "input" };
    /// let details = err.details();
    /// assert_eq!(details.get("limit"), Some(&DetailValue::Integer(8)));
    /// assert_eq!(details.get("what"), Some(&DetailValue::Text("input".into())));
    /// ```
    pub fn details(&self) -> ErrorDetails {
        let details = ErrorDetails::default();
        match self {
            Error::ModelNotFound(path) => details.text("path", path.to_string_lossy()),
            Error::ModelInvalid(reason)
            | Error::ParseFailed(reason)
            | Error::InvalidInput(reason) => details.text("reason", reason.as_str()),
            Error::InputTooLarge {
                limit,
                actual,
                what,
            } => details
                .text("what", *what)
                .integer("limit", *limit)
                .integer("actual", *actual),
            Error::UnsupportedFormat { format, feature } => {
                let details = match format {
                    Some(format) => details.text("format", format.wire_name()),
                    None => details,
                };
                match feature {
                    Some(feature) => details.text("feature", *feature),
                    None => details,
                }
            }
            Error::Io(error) => {
                let details = details.text("io_kind", io_kind_name(error.kind()));
                let context = error
                    .get_ref()
                    .and_then(|inner| inner.downcast_ref::<IoContext>());
                let details = match context {
                    Some(context) => context.add_to(details),
                    None => details,
                };
                details.text("reason", error.to_string())
            }
        }
    }
}

/// `{kind, message, details}`: the [`kind`](Error::kind), the `Display`
/// text, and the [`details`](Error::details). There is no `Deserialize`:
/// an error is reported, never read back into a value.
impl Serialize for Error {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Error", 3)?;
        state.serialize_field("kind", self.kind())?;
        state.serialize_field("message", &self.to_string())?;
        state.serialize_field("details", &self.details())?;
        state.end()
    }
}

/// Result type for matra operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Every dynamic value in an [`Error`]'s message, by name: what
/// [`Error::details`] returns.
///
/// Serializes as an object, its keys in the order the error names them.
/// The keys each kind carries are listed on [`Error::details`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ErrorDetails {
    entries: Vec<(&'static str, DetailValue)>,
}

impl ErrorDetails {
    /// The value named `key`, or `None` when this error carries no such
    /// value.
    pub fn get(&self, key: &str) -> Option<&DetailValue> {
        self.entries
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| value)
    }

    /// Every value with its name, in the order the error names them.
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &DetailValue)> + '_ {
        self.entries.iter().map(|(name, value)| (*name, value))
    }

    /// How many values there are.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn text(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.entries.push((key, DetailValue::Text(value.into())));
        self
    }

    fn integer(mut self, key: &'static str, value: usize) -> Self {
        // Lossless wherever Rust runs: no supported target has a `usize`
        // wider than 64 bits.
        let value = u64::try_from(value).unwrap_or(u64::MAX);
        self.entries.push((key, DetailValue::Integer(value)));
        self
    }

    fn flag(mut self, key: &'static str, value: bool) -> Self {
        self.entries.push((key, DetailValue::Bool(value)));
        self
    }
}

impl Serialize for ErrorDetails {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.entries.len()))?;
        for (key, value) in &self.entries {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

/// One value in an [`ErrorDetails`].
///
/// Serializes as the bare value: a string, a number or a boolean.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum DetailValue {
    /// Text: a path, a URL, a gate's label, a format's name, a reason.
    Text(String),
    /// A whole number, never negative: a size or a limit.
    Integer(u64),
    /// Yes or no: whether a retry may succeed.
    Bool(bool),
}

/// What matra knows about an I/O failure it raised itself, beyond the
/// operating system's kind: the path or the URL it concerns, and whether
/// the same call may succeed when made again.
///
/// It travels inside the [`std::io::Error`] that [`Error::Io`] wraps,
/// which keeps the variant's payload a plain `std::io::Error`, and
/// [`Error::details`] finds it there. Its `Display` is the whole message.
#[derive(Debug)]
pub(crate) struct IoContext {
    message: String,
    path: Option<PathBuf>,
    url: Option<String>,
    retryable: bool,
}

impl IoContext {
    /// A failure concerning `path`, whose message is `message`.
    pub(crate) fn at_path(
        kind: std::io::ErrorKind,
        path: &Path,
        message: impl Into<String>,
    ) -> std::io::Error {
        std::io::Error::new(
            kind,
            IoContext {
                message: message.into(),
                path: Some(path.to_path_buf()),
                url: None,
                retryable: false,
            },
        )
    }

    /// A filesystem `operation` on `path` that failed with `error`,
    /// keeping its kind: `cannot {operation} {path}: {error}`.
    ///
    /// `io error: No such file or directory (os error 2)` was the whole
    /// message for a missing input, with the path one line away in a
    /// variable; this is the one place the sentence that names both is
    /// written.
    pub(crate) fn cannot(operation: &str, path: &Path, error: &std::io::Error) -> std::io::Error {
        IoContext::at_path(
            error.kind(),
            path,
            format!("cannot {operation} {}: {error}", path.display()),
        )
    }

    /// A download from `url` that did not arrive, whose message is
    /// `message`. `retryable` says whether the same call may succeed when
    /// made again: true for a failure in the transfer, false where the
    /// message names a remedy that a retry cannot supply.
    ///
    /// Only the two provisioners download, and the embedder downloads
    /// nothing on `wasm32`, so a build with neither has no caller.
    #[cfg_attr(
        not(any(
            feature = "udpipe",
            all(feature = "model2vec", not(target_arch = "wasm32"))
        )),
        allow(dead_code)
    )]
    pub(crate) fn download(
        kind: std::io::ErrorKind,
        url: &str,
        message: impl Into<String>,
        retryable: bool,
    ) -> std::io::Error {
        std::io::Error::new(
            kind,
            IoContext {
                message: message.into(),
                path: None,
                url: Some(url.to_string()),
                retryable,
            },
        )
    }

    fn add_to(&self, details: ErrorDetails) -> ErrorDetails {
        let details = match &self.path {
            Some(path) => details.text("path", path.to_string_lossy()),
            None => details,
        };
        match &self.url {
            Some(url) => details
                .text("url", url.as_str())
                .flag("retryable", self.retryable),
            None => details,
        }
    }
}

impl std::fmt::Display for IoContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for IoContext {}

/// The `io_kind` detail: an I/O failure's class in snake case.
///
/// Written out rather than derived from `Debug`, which promises no
/// stable text. A kind not named here is `other`; the operating system's
/// account is in `reason` either way.
fn io_kind_name(kind: std::io::ErrorKind) -> &'static str {
    use std::io::ErrorKind;
    match kind {
        ErrorKind::NotFound => "not_found",
        ErrorKind::PermissionDenied => "permission_denied",
        ErrorKind::AlreadyExists => "already_exists",
        ErrorKind::Unsupported => "unsupported",
        ErrorKind::InvalidInput => "invalid_input",
        ErrorKind::InvalidData => "invalid_data",
        ErrorKind::TimedOut => "timed_out",
        ErrorKind::NotConnected => "not_connected",
        ErrorKind::ConnectionRefused => "connection_refused",
        ErrorKind::ConnectionReset => "connection_reset",
        ErrorKind::UnexpectedEof => "unexpected_eof",
        ErrorKind::Interrupted => "interrupted",
        ErrorKind::NotADirectory => "not_a_directory",
        ErrorKind::IsADirectory => "is_a_directory",
        ErrorKind::StorageFull => "storage_full",
        _ => "other",
    }
}

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
    /// Where the token's surface form lies in the analysed text, in
    /// UTF-8 bytes. The parser reports it against the paragraph text it
    /// was given, and `Engine::annotate` maps it into the analysed text.
    /// In a markdown paragraph whose text dropped inline markup, a span
    /// can cover markup inside the token (`foo**bar**` is one token whose
    /// span runs from the `f` to the last `r`), and a token read from an
    /// escape or an entity (`&amp;`) covers all of it.
    pub span: Span,
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
            span: Span::default(),
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
    span: Span,
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
    /// Set where the token lies in the text it was parsed from, in UTF-8
    /// bytes. Defaults to the empty span at `0`.
    pub fn span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }
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
            span: self.span,
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
    /// Where the sentence lies in the analysed text, in UTF-8 bytes:
    /// from the start of its first token to the end of its last, derived
    /// at construction from the token spans. The authority on where the
    /// sentence came from; `text` is normalized.
    pub span: Span,
    /// Sentence text as the NLP provider reports it, normalized. The
    /// UDPipe adapter rebuilds it from the token forms, joined by a space
    /// unless a token carries `SpaceAfter=No` in `misc`, so its
    /// whitespace can differ from the source; `span` locates the source.
    pub text: String,
    /// CoNLL-U tokens in id-sorted order.
    ///
    /// Read as empty when the JSON has no `tokens` key, which is how
    /// `matra analyze --json --depth sentence` writes a sentence, so a cut
    /// document reads back into this type. Empty then means "not written",
    /// not "no tokens": the sentence's derived fields still describe them.
    #[serde(default)]
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
    /// fields (`span`, `word_count`, `tree_depth`, `passive`, `negations`,
    /// `modals`, `clausal_complements`, `root_adverbials`, `root_mood`)
    /// are computed here and reflect `tokens` as passed in; mutating
    /// `tokens` afterwards does not recompute them, and keeping them
    /// consistent is part of the same caller contract.
    /// `hearst_pairs` starts empty and is filled at the annotate stage
    /// by the pipeline, because its detector lives outside the domain
    /// (see the field's documentation).
    pub fn new(text: String, tokens: Vec<Token>) -> Self {
        let span = match (tokens.first(), tokens.last()) {
            (Some(first), Some(last)) => Span::new(first.span.byte_start, last.span.byte_end),
            _ => Span::default(),
        };
        let word_count = tokens.iter().filter(|t| !t.is_punct).count();
        let tree_depth = measure_tree_depth(&tokens);
        let passive = detect_passive(&tokens);
        let negations = detect_negations(&tokens);
        let modals = detect_modals(&tokens);
        let clausal_complements = detect_clausal_complements(&tokens);
        let root_adverbials = detect_root_adverbials(&tokens);
        let root_mood = detect_root_mood(&tokens);
        Self {
            span,
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

/// What kind of block a [`Paragraph`] was read from.
///
/// Serialized as `paragraph`, `quote` and `list_item`. A reader treats
/// the set as open: a later reader can add a kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ParagraphKind {
    /// An ordinary paragraph, and every paragraph of plain text and of
    /// block input.
    Paragraph,
    /// A paragraph inside a block quote, at any depth, a list item
    /// inside a quote included. Quoted text is not the author's own, so
    /// a quote is never parsed and never measured: it keeps no sentences
    /// and its metric slots stay `None`.
    Quote,
    /// The text of a list item outside any block quote.
    ListItem,
}

/// One paragraph of prose. `Engine::annotate` attaches its sentences and
/// `Engine::compose`, which runs the metric suite, fills its
/// [`ParagraphMetrics`].
///
/// A [`ParagraphKind::Quote`] paragraph is skipped by both stages: it
/// keeps no sentences, and its metric slots stay `None`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Paragraph {
    /// Where the paragraph lies in the analysed text, in UTF-8 bytes,
    /// with trailing whitespace excluded. For markdown it covers the
    /// block quote and list markers on the paragraph's lines, so every
    /// byte of the input that is not whitespace lies in exactly one
    /// heading, paragraph or skipped span.
    pub span: Span,
    /// The paragraph's text, normalized: for markdown, the inline text
    /// the parser reads, with the markup removed (emphasis markers, link
    /// and image syntax, the backticks of a code span, block quote and
    /// list markers), soft line breaks as `\n`, escapes and entities
    /// decoded; for plain text and block input, a verbatim slice. `span`
    /// is the authority on where it came from.
    pub text: String,
    /// The kind of block the paragraph was read from.
    pub kind: ParagraphKind,
    /// Sentences produced by parsing this paragraph (attached by
    /// `Engine::annotate`).
    ///
    /// Read as empty when the JSON has no `sentences` key, as
    /// `matra analyze --json --depth paragraph` writes a paragraph; the
    /// paragraph's metrics still describe them.
    #[serde(default)]
    pub sentences: Vec<Sentence>,
    /// The paragraph's metrics, filled by `Engine::compose`.
    pub metrics: ParagraphMetrics,
}

impl Paragraph {
    /// Construct a new `Paragraph` with empty sentences and an empty
    /// metric record. `Engine::annotate` attaches the sentences and
    /// `Engine::compose` fills the record.
    pub fn new(text: String, kind: ParagraphKind, span: Span) -> Self {
        Self {
            span,
            text,
            kind,
            sentences: Vec::new(),
            metrics: ParagraphMetrics::default(),
        }
    }

    /// Total word count across all sentences.
    ///
    /// Saturates at `usize::MAX` rather than overflowing: the counts are
    /// whatever the sentences carry, and a document read back from JSON
    /// can carry any, so a sum past the largest count reads as that.
    pub fn word_count(&self) -> usize {
        self.sentences
            .iter()
            .fold(0usize, |total, s| total.saturating_add(s.word_count))
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
    /// Section heading, if any, its text normalized as a paragraph's is.
    /// `None` for the section holding what precedes the first heading,
    /// and for plain text (which produces one heading-less section).
    pub heading: Option<String>,
    /// Heading depth (0 with no heading, 1+ for a heading: markdown `#`
    /// is 1, `##` is 2, and a block input heading keeps its level).
    pub level: usize,
    /// Where the heading lies in the analysed text, in UTF-8 bytes,
    /// markers included (`## Title` from the first `#`); `None` with no
    /// heading.
    pub span: Option<Span>,
    /// Paragraphs in document order.
    pub paragraphs: Vec<Paragraph>,
}

impl Section {
    /// Construct a new section with the given heading, level, heading
    /// span and paragraphs.
    pub fn new(
        heading: Option<String>,
        level: usize,
        span: Option<Span>,
        paragraphs: Vec<Paragraph>,
    ) -> Self {
        Self {
            heading,
            level,
            span,
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

/// Where a result came from: the matra build and schema version that
/// produced it, the parser that read the document, the format it was read
/// as, and the input itself.
///
/// A [`Document`] carries it as recorded when the document was read. A
/// result derived from a document ([`Summary`], [`Keyphrases`],
/// [`Clusters`]) carries a copy of the document's, plus the `method` that
/// ran and the `parameters` that shaped it; on a document both are
/// `None`, and absent from the JSON.
///
/// No timestamp: matra's outputs are deterministic, and a timestamp would
/// make two identical analyses differ.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// The method that produced a derived result, for example `tfidf`:
    /// `None` on a document. A reader treats the set as open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// The parameters that shaped a derived result: `None` on a
    /// document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Parameters>,
}

/// The parameters that shaped a derived result, as it ran: only those
/// the method takes are `Some`, and only those appear in the JSON.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Parameters {
    /// How many sentences a summary keeps, or how many phrases keyphrase
    /// extraction returns at most.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n: Option<usize>,
    /// The cosine similarity an edge between two sentences cleared to
    /// join them in a cluster.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
    /// The embedder whose vectors were clustered, as it reports itself
    /// (`Embedder::identity`): its name, its model, the digest over the
    /// model's artifacts and its licence, so the scores stay attributable
    /// to one geometry and the model's terms travel with them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedder: Option<Identity>,
}

impl Parameters {
    /// The parameters of a method that takes a count.
    pub(crate) fn count(n: usize) -> Self {
        Self {
            n: Some(n),
            ..Self::default()
        }
    }

    /// The parameters of clustering: the threshold and the embedder.
    pub(crate) fn clustering(threshold: f64, embedder: Identity) -> Self {
        Self {
            threshold: Some(threshold),
            embedder: Some(embedder),
            ..Self::default()
        }
    }
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
            method: None,
            parameters: None,
        }
    }

    /// This provenance, copied onto a result derived from the document it
    /// describes, with the method that ran and its parameters.
    pub(crate) fn derived(&self, method: &str, parameters: Parameters) -> Self {
        Self {
            method: Some(method.to_string()),
            parameters: Some(parameters),
            ..self.clone()
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
/// attaches each paragraph's parsed sentences, records what the reader
/// skipped and records the provenance; `Engine::compose` fills the metric
/// records.
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
    ///
    /// Read as empty when the JSON has no `sections` key, as
    /// `matra analyze --json --depth document` writes a document; its
    /// metrics still describe them.
    #[serde(default)]
    pub sections: Vec<Section>,
    /// The document's metrics, filled by `Engine::compose`.
    pub metrics: DocumentMetrics,
    /// What the reader saw and did not analyse (a table, a code block,
    /// front matter), in document order. Never silently dropped.
    pub skipped: Vec<Skipped>,
}

impl Document {
    /// Construct a new `Document` from its provenance, a section tree and
    /// what the reader skipped, with an empty metric record;
    /// `Engine::compose` fills it in.
    pub fn new(provenance: Provenance, sections: Vec<Section>, skipped: Vec<Skipped>) -> Self {
        Self {
            provenance,
            sections,
            metrics: DocumentMetrics::default(),
            skipped,
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
// Outcomes: what the outcome functions derive from a document
// ---------------------------------------------------------------------------

/// The sentences that best represent a document, from `matra::summarize`.
///
/// `provenance` is the document's, with the method that ranked the
/// sentences and the `n` it was asked for. Each method scores on its own
/// scale, so a score compares with another score from the same method
/// only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Summary {
    /// The document's provenance, plus `method` and `parameters`.
    pub provenance: Provenance,
    /// At most `n` sentences, in document order.
    pub sentences: Vec<SummarySentence>,
}

/// One sentence a summary keeps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SummarySentence {
    /// The sentence's position among the document's sentences
    /// ([`Document::sentences`]), counting from 0.
    pub index: usize,
    /// Where the sentence lies in the analysed text, as on
    /// [`Sentence::span`].
    pub span: Span,
    /// The sentence's text, as on [`Sentence::text`].
    pub text: String,
    /// The method's score for it, higher is more representative.
    pub score: f64,
}

impl SummarySentence {
    /// The sentence at `index` of `sentence`, with its score.
    pub(crate) fn new(index: usize, sentence: &Sentence, score: f64) -> Self {
        Self {
            index,
            span: sentence.span,
            text: sentence.text.clone(),
            score,
        }
    }
}

/// The phrases that best name what a document is about, from
/// `matra::keyphrases`.
///
/// `provenance` is the document's, with the method that ranked the
/// phrases and the `n` it was asked for. Each method scores on its own
/// scale, so a score compares with another score from the same method
/// only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Keyphrases {
    /// The document's provenance, plus `method` and `parameters`.
    pub provenance: Provenance,
    /// At most `n` phrases, highest score first. Equal scores are ordered
    /// the same way on every run.
    pub phrases: Vec<Keyphrase>,
}

/// One ranked keyphrase and where it occurs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Keyphrase {
    /// The phrase: the lowercased lemmas of its words, joined by a space.
    pub phrase: String,
    /// The method's score for it, higher is more relevant.
    pub score: f64,
    /// Every stretch of the analysed text the phrase was found in, in
    /// document order: from its first word's start to its last word's
    /// end, so a word the method passes over (a stop word between two
    /// content words) lies inside the span.
    pub occurrences: Vec<Span>,
}

/// Sentences of a document grouped by the similarity of their
/// embeddings, from `matra::clusters`: model opinion, standing alone,
/// never attached to [`Document`] (RFC-0010).
///
/// `provenance` is the document's, with the method and the `threshold`
/// and `embedder` that shaped the result, so the scores cannot be
/// attributed to a model that did not produce them.
///
/// A cluster is a connected component of the graph whose edges are the
/// sentence pairs with a cosine similarity of at least `threshold`, so
/// membership is transitive: two sentences can share a cluster without
/// sharing an edge, and co-membership must never be read as pairwise
/// similarity. `edges` lists every pair that cleared the bar; a pair not
/// listed is no claim, not low similarity.
///
/// A sentence with no edge appears in no cluster: singletons are excluded
/// by construction, so "not in any cluster" is a meaningful count.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Clusters {
    /// The document's provenance, plus `method` and `parameters`.
    pub provenance: Provenance,
    /// The clusters, ordered by their smallest member.
    pub clusters: Vec<Cluster>,
    /// Every pair of sentences whose similarity cleared the threshold,
    /// ordered by `a`, then `b`. Both ends of an edge are members of the
    /// same cluster.
    pub edges: Vec<SimilarityEdge>,
}

/// One connected component of the above-threshold similarity graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Cluster {
    /// The sentence indices ([`Document::sentences`] positions) in the
    /// component, ascending.
    pub members: Vec<usize>,
}

impl Cluster {
    /// A cluster of `members`, ascending.
    pub(crate) fn new(members: Vec<usize>) -> Self {
        Self { members }
    }
}

/// A similarity between two sentences that cleared the threshold,
/// `a < b`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SimilarityEdge {
    /// The lower sentence index.
    pub a: usize,
    /// The higher sentence index.
    pub b: usize,
    /// Their cosine similarity, in the embedder's geometry.
    pub score: f64,
}

impl SimilarityEdge {
    /// The edge between sentences `a < b`.
    pub(crate) fn new(a: usize, b: usize, score: f64) -> Self {
        Self { a, b, score }
    }
}

// ---------------------------------------------------------------------------
// Source / corpus types
// ---------------------------------------------------------------------------

/// A document's format: how its bytes are read.
///
/// `Pdf` and `Docx` are recognised, by extension and by their bytes, but
/// no reader for either ships, so analysing one returns
/// [`Error::UnsupportedFormat`].
///
/// Serialized in the format vocabulary every surface shares:
/// `markdown`, `plain_text`, `blocks`, `pdf`, `docx`. A reader treats the
/// set as open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Format {
    /// Markdown (CommonMark, with tables, strikethrough, task lists and
    /// front matter).
    Markdown,
    /// Plain text: paragraphs separated by blank lines.
    PlainText,
    /// Headings and paragraphs the caller supplies: the JSON array whose
    /// schema is `spec/schema/v2/blocks.schema.json`, or [`Block`]s.
    Blocks,
    /// PDF (recognised; no reader ships).
    Pdf,
    /// DOCX (recognised; no reader ships).
    Docx,
}

impl Format {
    /// The format's name in the shared vocabulary, as it serializes.
    pub(crate) fn wire_name(&self) -> &'static str {
        match self {
            Format::Markdown => "markdown",
            Format::PlainText => "plain_text",
            Format::Blocks => "blocks",
            Format::Pdf => "pdf",
            Format::Docx => "docx",
        }
    }

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
    /// extension at all, are [`Format::PlainText`]: the format selects a
    /// reader, and plain text reads any UTF-8 text. No extension implies
    /// [`Format::Blocks`]; a caller names it. When the bytes are read, a
    /// PDF or DOCX signature outranks the extension.
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

/// One heading or paragraph a caller supplies, for a document matra has
/// no reader for: text extracted from a PDF, a CMS, a transcript.
///
/// Input only, and opaque: built by [`Block::heading`] and
/// [`Block::paragraph`], handed to `Input::blocks`, and never returned.
/// The analysed text of block input is the blocks' texts joined by a
/// blank line (`"\n\n"`), in order, and every span in the result points
/// into that text. The same blocks as JSON are [`Format::Blocks`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block(pub(crate) BlockContent);

/// What a [`Block`] holds. Crate-private, so the block vocabulary can
/// grow without a breaking change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BlockContent {
    /// A heading opening a section.
    Heading {
        /// Its level, 1 or more.
        level: u8,
        /// Its text.
        text: String,
    },
    /// A paragraph of prose.
    Paragraph {
        /// Its text.
        text: String,
    },
}

impl Block {
    /// A heading at `level` opening a new section: 1 for a top-level
    /// heading, 2 below it, and so on. A level of 0 is refused with
    /// [`Error::InvalidInput`] when the input is analysed, since 0 is the
    /// level of a section with no heading.
    pub fn heading(level: u8, text: impl Into<String>) -> Block {
        Block(BlockContent::Heading {
            level,
            text: text.into(),
        })
    }

    /// A paragraph of prose, parsed into sentences.
    pub fn paragraph(text: impl Into<String>) -> Block {
        Block(BlockContent::Paragraph { text: text.into() })
    }
}

/// A stretch of the analysed text, in UTF-8 bytes: `byte_start`
/// inclusive, `byte_end` exclusive, so
/// `&text[span.byte_start..span.byte_end]` is the stretch.
///
/// For markdown and plain text the analysed text is the input itself;
/// for block input it is the blocks' texts joined by a blank line.
/// Bytes, not characters: a caller in a language that indexes strings by
/// character converts. The unit is in the field names because a byte
/// span used to slice a character-indexed string (a Python `str`) is
/// right on ASCII and silently wrong after the first character outside
/// it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Span {
    /// The first byte.
    pub byte_start: usize,
    /// One past the last byte.
    pub byte_end: usize,
}

impl Span {
    /// The span from byte `byte_start` (inclusive) to byte `byte_end`
    /// (exclusive).
    pub fn new(byte_start: usize, byte_end: usize) -> Self {
        Self {
            byte_start,
            byte_end,
        }
    }
}

/// What kind of construct a reader saw and did not analyse.
///
/// Each names the construct, never a judgment about it. Serialized in
/// snake case; a reader treats the set as open, since a later reader can
/// add a kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SkippedKind {
    /// A table.
    Table,
    /// A fenced or indented code block.
    Code,
    /// A front matter block (YAML between `---` lines, or TOML between
    /// `+++` lines) at the very start of the document.
    FrontMatter,
    /// An HTML block.
    Html,
    /// A thematic break (`---`, `***`, `___`).
    ThematicBreak,
    /// A link reference definition (`[label]: https://example.com`).
    LinkDefinition,
}

/// One construct the reader saw and did not analyse, with where it lies.
///
/// Skipping is specified behaviour reported beside a correct result, not
/// an error: a table is not prose, so it is not parsed, but the result
/// says it was there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Skipped {
    /// What the construct is.
    pub kind: SkippedKind,
    /// Where it lies in the analysed text, in UTF-8 bytes, with trailing
    /// whitespace excluded.
    pub span: Span,
}

impl Skipped {
    /// A construct of `kind` at `span`.
    pub fn new(kind: SkippedKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// A per-document failure: the error plus the path it occurred at, if
/// the document came from disk.
///
/// `path` is `Option` because in-memory text has no path. This closes a
/// real hole: the previous corpus surface fabricated an empty `PathBuf`
/// for a path-less document, which collided with a genuinely empty path.
///
/// Serializes as the error's own form plus the path:
/// `{path, kind, message, details}`, with `path` null for an input that
/// never came from disk, and written lossily where it is not UTF-8.
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

impl Serialize for DocumentError {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;

        /// The path through the lossy form [`ProvenanceSource`] uses.
        struct LossyPath<'a>(&'a Option<PathBuf>);
        impl Serialize for LossyPath<'_> {
            fn serialize<S: serde::Serializer>(
                &self,
                serializer: S,
            ) -> std::result::Result<S::Ok, S::Error> {
                serialize_path_lossy(self.0, serializer)
            }
        }

        let mut state = serializer.serialize_struct("DocumentError", 4)?;
        state.serialize_field("path", &LossyPath(&self.path))?;
        state.serialize_field("kind", self.error.kind())?;
        state.serialize_field("message", &self.error.to_string())?;
        state.serialize_field("details", &self.error.details())?;
        state.end()
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
            span: Default::default(),
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

    /// The message says what could not be read and what would read it:
    /// a feature when one exists, and plainly that none does otherwise.
    #[test]
    fn unsupported_format_names_the_format_and_the_remedy() {
        let pdf = Error::UnsupportedFormat {
            format: Some(Format::Pdf),
            feature: None,
        };
        assert_eq!(
            pdf.to_string(),
            "unsupported format: pdf; no feature of this matra version reads it"
        );
        let with_feature = Error::UnsupportedFormat {
            format: Some(Format::Docx),
            feature: Some("docx"),
        };
        assert_eq!(
            with_feature.to_string(),
            "unsupported format: docx; enable the `docx` feature to read it"
        );
        let undecodable = Error::UnsupportedFormat {
            format: None,
            feature: None,
        };
        assert!(undecodable.to_string().contains("not UTF-8 text"));
    }

    /// The name a message uses is the name the format serializes as, so
    /// the two vocabularies cannot drift.
    #[test]
    fn format_wire_name_is_its_serialized_name() {
        for format in [
            Format::Markdown,
            Format::PlainText,
            Format::Blocks,
            Format::Pdf,
            Format::Docx,
        ] {
            assert_eq!(
                serde_json::to_value(&format).unwrap(),
                serde_json::json!(format.wire_name())
            );
        }
    }

    /// A sentence's span runs from its first token's start to its last
    /// token's end, and is empty at 0 with no tokens.
    #[test]
    fn sentence_span_is_derived_from_its_tokens() {
        let tok = |id: usize, start: usize, end: usize| {
            Token::builder(
                id,
                "w".to_string(),
                "w".to_string(),
                "X".to_string(),
                0,
                "root".to_string(),
            )
            .span(Span::new(start, end))
            .build()
        };
        let sentence = Sentence::new("a b".to_string(), vec![tok(1, 4, 5), tok(2, 6, 9)]);
        assert_eq!(sentence.span, Span::new(4, 9));
        assert_eq!(
            Sentence::new(String::new(), Vec::new()).span,
            Span::default()
        );
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
            (
                Error::UnsupportedFormat {
                    format: Some(Format::Pdf),
                    feature: None,
                },
                "unsupported_format",
            ),
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

    /// Each limit is published and the docs state its number, so every
    /// value is pinned here: a change fails this test and has to be made
    /// on purpose, with the docs that state it.
    #[test]
    fn the_limits_hold_their_published_values() {
        use super::limits::*;
        assert_eq!(MAX_INPUT_BYTES, 8_388_608);
        assert_eq!(MAX_CONFIG_FILE_BYTES, 65_536);
        assert_eq!(MAX_DOWNLOAD_BYTES, 67_108_864);
        assert_eq!(MAX_TFIDF_SENTENCES, 2000);
        assert_eq!(MAX_TEXTRANK_SENTENCES, 2000);
        assert_eq!(MAX_CLUSTER_SENTENCES, MAX_TEXTRANK_SENTENCES);
        assert_eq!(MAX_RAKE_TOKENS, 200_000);
        assert_eq!(MAX_YAKE_TOKENS, 200_000);
        assert_eq!(MAX_COMPRESSION_PARAGRAPH_BYTES, 262_144);
        assert_eq!(MIN_READABILITY_WORDS, 11);
        assert_eq!(MIN_COMPRESSION_WORDS, 51);
    }

    /// The details of `error` as a JSON object, for comparing whole.
    fn details_json(error: &Error) -> serde_json::Value {
        serde_json::to_value(error.details()).expect("details serialize")
    }

    /// Every dynamic value in a message is also a detail, so a reader
    /// never parses the message (EPR-0006). One case per variant, and per
    /// shape a variant takes, each built with values distinct enough
    /// that a detail cannot match by accident.
    #[test]
    fn every_dynamic_value_in_a_message_is_a_detail() {
        use serde_json::json;
        let cases: Vec<(Error, serde_json::Value)> = vec![
            (
                Error::ModelNotFound(PathBuf::from("models/m.udpipe")),
                json!({ "path": "models/m.udpipe" }),
            ),
            (
                Error::ModelInvalid("truncated at byte 7".to_string()),
                json!({ "reason": "truncated at byte 7" }),
            ),
            (
                Error::ParseFailed("udpipe panicked: boom".to_string()),
                json!({ "reason": "udpipe panicked: boom" }),
            ),
            (
                Error::InputTooLarge {
                    limit: 2000,
                    actual: 2001,
                    what: "tfidf",
                },
                json!({ "what": "tfidf", "limit": 2000, "actual": 2001 }),
            ),
            (
                Error::UnsupportedFormat {
                    format: Some(Format::Pdf),
                    feature: None,
                },
                json!({ "format": "pdf" }),
            ),
            (
                Error::UnsupportedFormat {
                    format: Some(Format::Docx),
                    feature: Some("docx"),
                },
                json!({ "format": "docx", "feature": "docx" }),
            ),
            (
                Error::UnsupportedFormat {
                    format: None,
                    feature: None,
                },
                json!({}),
            ),
            (
                Error::InvalidInput("threshold must be finite".to_string()),
                json!({ "reason": "threshold must be finite" }),
            ),
            (
                Error::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "denied",
                )),
                json!({ "io_kind": "permission_denied", "reason": "denied" }),
            ),
            (
                Error::Io(IoContext::at_path(
                    std::io::ErrorKind::NotFound,
                    Path::new("/models/x"),
                    "cannot read /models/x: gone",
                )),
                json!({
                    "io_kind": "not_found",
                    "path": "/models/x",
                    "reason": "cannot read /models/x: gone",
                }),
            ),
            (
                Error::Io(IoContext::download(
                    std::io::ErrorKind::TimedOut,
                    "https://models.example/m",
                    "download https://models.example/m: timeout",
                    true,
                )),
                json!({
                    "io_kind": "timed_out",
                    "url": "https://models.example/m",
                    "retryable": true,
                    "reason": "download https://models.example/m: timeout",
                }),
            ),
        ];
        for (error, expected) in &cases {
            assert_eq!(details_json(error), *expected, "{error}");
        }
    }

    /// The I/O payload carries what the message says without changing
    /// the message: an `Io` built with a context reads exactly as one
    /// built from the text alone.
    #[test]
    fn an_io_context_does_not_change_the_message() {
        let plain = Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "cannot read /m: gone",
        ));
        let with_context = Error::Io(IoContext::at_path(
            std::io::ErrorKind::NotFound,
            Path::new("/m"),
            "cannot read /m: gone",
        ));
        assert_eq!(plain.to_string(), with_context.to_string());
        assert_eq!(plain.kind(), with_context.kind());
    }

    /// The serde form is `{kind, message, details}`, and a
    /// `DocumentError`'s adds `path`, null for input never on disk.
    #[test]
    fn errors_serialize_as_kind_message_and_details() {
        let error = Error::InputTooLarge {
            limit: 8,
            actual: 9,
            what: "input",
        };
        assert_eq!(
            serde_json::to_value(&error).unwrap(),
            serde_json::json!({
                "kind": "input_too_large",
                "message": "input too large: 9 > limit 8",
                "details": { "what": "input", "limit": 8, "actual": 9 },
            })
        );

        let on_disk = DocumentError::new(
            Some(PathBuf::from("essay.md")),
            Error::ParseFailed("boom".to_string()),
        );
        assert_eq!(
            serde_json::to_value(&on_disk).unwrap(),
            serde_json::json!({
                "path": "essay.md",
                "kind": "parse_failed",
                "message": "parse failed: boom",
                "details": { "reason": "boom" },
            })
        );
        let in_memory = DocumentError::new(None, Error::ParseFailed("boom".to_string()));
        assert!(serde_json::to_value(&in_memory).unwrap()["path"].is_null());
    }

    /// A path that is not UTF-8 serializes lossily, as a document's
    /// source path does, rather than failing the error over its name.
    #[cfg(unix)]
    #[test]
    fn a_non_utf8_document_error_path_serializes_lossily() {
        use std::os::unix::ffi::OsStrExt;
        let path = PathBuf::from(std::ffi::OsStr::from_bytes(b"bad\xffname.md"));
        let err = DocumentError::new(Some(path), Error::ParseFailed("boom".to_string()));
        let json = serde_json::to_value(&err).expect("serializes");
        assert_eq!(json["path"], "bad\u{fffd}name.md");
    }

    /// The `io_kind` names are written out, so they cannot drift with
    /// `ErrorKind`'s `Debug` text, and a kind not named is `other`.
    #[test]
    fn io_kinds_are_named_in_snake_case() {
        use std::io::ErrorKind;
        for (kind, name) in [
            (ErrorKind::NotFound, "not_found"),
            (ErrorKind::PermissionDenied, "permission_denied"),
            (ErrorKind::TimedOut, "timed_out"),
            (ErrorKind::NotConnected, "not_connected"),
            (ErrorKind::NotADirectory, "not_a_directory"),
            (ErrorKind::Other, "other"),
            (ErrorKind::WouldBlock, "other"),
        ] {
            assert_eq!(io_kind_name(kind), name, "{kind:?}");
        }
    }
}
