#![doc = include_str!("../README.md")]

#[cfg(feature = "cli")]
pub mod cli;
pub mod config;
// Without `udpipe` the build ships no parser, so no engine is built and no
// reader runs: the readers are unreachable there, and still compile, which
// is what boundary rule 6 asks of that build.
#[cfg_attr(not(feature = "udpipe"), allow(dead_code))]
mod decompose;
mod detect;
mod digest;
pub mod domain;
pub mod embed;
mod extraction;
mod hearst;
mod metrics;
mod nlp;
mod source;
mod stopwords;
mod structure;

// The published schemas, as the test suites check against them: one copy,
// shared with the runners in tests/.
#[cfg(test)]
#[path = "../tests/document_schema/mod.rs"]
mod document_schema;

use std::path::{Path, PathBuf};

use decompose::Decomposer;
pub use domain::Block;
use domain::limits::MAX_INPUT_BYTES;
use domain::{Document, Format, ParagraphKind};
pub use extraction::{
    KeyphraseMethod, SummaryMethod, keyphrases, keyphrases_with, summarize, summarize_with,
};
use nlp::NlpProvider;
use source::Source;

/// Reject an input or an analysed text of `len` bytes over
/// [`MAX_INPUT_BYTES`].
///
/// Returns `Error::InputTooLarge { what: "input", .. }` so consumers can
/// distinguish the bound check from per-extractor caps (which use distinct
/// `what` labels). `Engine::annotate` runs this gate, and annotate is the
/// only route from text to the parser.
fn check_input_size(len: usize) -> domain::Result<()> {
    if len > MAX_INPUT_BYTES {
        return Err(domain::Error::InputTooLarge {
            limit: MAX_INPUT_BYTES,
            actual: len,
            what: "input",
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The input surface: Input, Inputs -> Engine
// ---------------------------------------------------------------------------

/// One document to analyze: text, bytes, a file, or blocks.
///
/// Opaque and lazy. Constructing one reads nothing and cannot fail; the
/// file is read, the format detected and the size cap applied when
/// [`Engine::analyze`] or [`Engine::analyze_all`] reaches it, so a file
/// that cannot be read is one failed item of a stream, not an abort.
///
/// How the format is decided, from strongest to weakest evidence: a PDF
/// or DOCX signature in the bytes, the format the caller names, the file
/// name's extension, and plain text. A recognised format no reader ships
/// for fails with [`domain::Error::UnsupportedFormat`], and so do bytes
/// that are not UTF-8 text.
///
/// Every constructor reaches the parser through the same step, so the
/// [`MAX_INPUT_BYTES`] cap holds for all of them.
#[derive(Debug, Clone)]
pub struct Input(InputKind);

#[derive(Debug, Clone)]
enum InputKind {
    Text {
        text: String,
        format: Format,
    },
    Bytes {
        data: Vec<u8>,
        format: Option<Format>,
    },
    File {
        path: PathBuf,
        format: Option<Format>,
    },
    Blocks(Vec<Block>),
}

impl Input {
    /// Text in a format the caller names. The text is the analysed text:
    /// every span in the result points into it.
    ///
    /// ```
    /// let input = matra::Input::text("# Notes\n\nThe river rose.", matra::domain::Format::Markdown);
    /// ```
    pub fn text(text: impl Into<String>, format: Format) -> Input {
        Input(InputKind::Text {
            text: text.into(),
            format,
        })
    }

    /// Bytes from anywhere: a caller that fetches its own documents hands
    /// them in here. With `format` `None` the format is detected from the
    /// bytes (a PDF or DOCX signature) and is otherwise plain text; a
    /// signature outranks a format the caller names.
    pub fn bytes(data: impl Into<Vec<u8>>, format: Option<Format>) -> Input {
        Input(InputKind::Bytes {
            data: data.into(),
            format,
        })
    }

    /// The file at `path`, read when the input is analysed. Its format is
    /// detected from its bytes, then its extension. A symlink is refused,
    /// and a directory fails with [`domain::Error::InvalidInput`] naming
    /// [`Inputs::dir`], which reads one. The path is recorded in the
    /// result's `provenance.source.path`.
    pub fn file(path: impl AsRef<Path>) -> Input {
        Input(InputKind::File {
            path: path.as_ref().to_path_buf(),
            format: None,
        })
    }

    /// The file at `path`, read as `format` in place of the format its
    /// extension implies: [`Input::file`], with the caller naming the
    /// format. A JSON file of blocks is the case it exists for, since no
    /// extension implies [`Format::Blocks`].
    ///
    /// A PDF or DOCX signature in the bytes still outranks the caller, as
    /// it does for [`Input::bytes`]: a file's bytes do not lie about being
    /// a PDF the way its name can. Everything else about the file is
    /// [`Input::file`]'s: read when analysed, a symlink refused, the path
    /// recorded in `provenance.source.path`.
    ///
    /// ```
    /// use matra::Input;
    /// use matra::domain::Format;
    /// let input = Input::file_as("notes.json", Format::Blocks);
    /// ```
    pub fn file_as(path: impl AsRef<Path>, format: Format) -> Input {
        Input(InputKind::File {
            path: path.as_ref().to_path_buf(),
            format: Some(format),
        })
    }

    /// Headings and paragraphs the caller supplies, for a format matra
    /// has no reader for. The analysed text is the blocks' texts joined
    /// by a blank line (`"\n\n"`), in order.
    ///
    /// ```
    /// use matra::{Block, Input};
    /// let input = Input::blocks([
    ///     Block::heading(1, "Field notes"),
    ///     Block::paragraph("The river rose overnight."),
    /// ]);
    /// ```
    pub fn blocks(blocks: impl IntoIterator<Item = Block>) -> Input {
        Input(InputKind::Blocks(blocks.into_iter().collect()))
    }

    /// The path a file input names, which a failed item carries.
    fn path(&self) -> Option<&Path> {
        match &self.0 {
            InputKind::File { path, .. } => Some(path),
            InputKind::Text { .. } | InputKind::Bytes { .. } | InputKind::Blocks(_) => None,
        }
    }
}

/// The documents in a directory: an iterator of [`Input::file`], one per
/// regular file, sorted by path. Not recursive; symlinks are skipped.
///
/// [`Inputs::dir`] lists the directory up front, and a listing failure is
/// its `Err`; each file is read only when its `Input` is analysed, so a
/// file that cannot be read is one failed item, not an aborted walk.
///
/// ```no_run
/// # fn example(engine: &matra::Engine) -> matra::domain::Result<()> {
/// for item in engine.analyze_all(matra::Inputs::dir("docs/")?) {
///     match item {
///         Ok(doc) => println!("{:?}: {} words", doc.provenance.source.path, doc.metrics.total_words),
///         Err(e) => eprintln!("{e}"),
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct Inputs {
    paths: std::vec::IntoIter<PathBuf>,
}

impl Inputs {
    /// The regular files in the directory at `path`.
    ///
    /// # Errors
    ///
    /// [`domain::Error::Io`] when the path does not exist, cannot be
    /// listed, or is a symlink (refused, as a file symlink is);
    /// [`domain::Error::InvalidInput`] naming [`Input::file`] when it is
    /// not a directory.
    pub fn dir(path: impl AsRef<Path>) -> domain::Result<Inputs> {
        let path = path.as_ref();
        let file_type = std::fs::symlink_metadata(path)
            .map_err(|e| domain::Error::Io(domain::IoContext::cannot("read", path, &e)))?
            .file_type();
        if file_type.is_symlink() {
            return Err(domain::Error::Io(domain::IoContext::at_path(
                std::io::ErrorKind::Unsupported,
                path,
                format!("refusing to read symlink: {}", path.display()),
            )));
        }
        if !file_type.is_dir() {
            return Err(domain::Error::InvalidInput {
                reason: "not_a_directory",
                message: format!(
                    "{} is not a directory; read a file with Input::file",
                    path.display()
                ),
            });
        }
        let paths = source::directory::DirectorySource.candidate_paths(path)?;
        Ok(Inputs {
            paths: paths.into_iter(),
        })
    }
}

impl Iterator for Inputs {
    type Item = Input;

    fn next(&mut self) -> Option<Input> {
        self.paths.next().map(Input::file)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.paths.size_hint()
    }
}

impl ExactSizeIterator for Inputs {}

/// The assembled pipeline: the parser, plus the readers this build ships.
///
/// Construction is the wiring step; after it, analysis is
/// `fetch -> detect -> read -> structure -> parse -> measure`, each step
/// a crate-private module hiding one decision. Variation lives in the
/// data ([`Input`]'s constructors, the format), not in the function
/// namespace. What a caller does with the [`Document`] it returns is an
/// outcome function's job ([`summarize`], [`keyphrases`], [`clusters`]),
/// and those need no engine.
///
/// The parser is UDPipe with its pinned English model, so an engine is
/// built only in a build with the `udpipe` feature: [`Engine::new`],
/// [`Engine::from_config`] and [`Engine::from_config_with_notice`].
///
/// An `Engine` is `Send` but not `Sync`: the model holds C-side state
/// its parse path mutates, so an engine moves to another thread but is
/// not shared between two. Build one on each worker thread, or serialize
/// calls behind a `Mutex`.
pub struct Engine {
    nlp: Box<dyn NlpProvider>,
    readers: decompose::Decomposers,
}

impl Engine {
    /// Wire an engine around a parser, with the readers this build ships
    /// (markdown, plain text and block input). Crate-private: the parser
    /// trait is, so the one parser outside the tests is UDPipe, wired by
    /// the public constructors.
    #[cfg(any(feature = "udpipe", test))]
    pub(crate) fn with_parser(nlp: Box<dyn NlpProvider>) -> Self {
        Self {
            nlp,
            readers: standard_readers(),
        }
    }

    /// Analyze one document: read it, parse each paragraph, and measure
    /// the result.
    ///
    /// # Errors
    ///
    /// [`domain::Error::InputTooLarge`] over [`MAX_INPUT_BYTES`];
    /// [`domain::Error::UnsupportedFormat`] for a format no reader ships
    /// for, or bytes that are not UTF-8; [`domain::Error::InvalidInput`]
    /// for a directory named as a file and for block input that does not
    /// parse; [`domain::Error::Io`] when a file cannot be read; and
    /// [`domain::Error::ParseFailed`] when the parser fails.
    pub fn analyze(&self, input: Input) -> domain::Result<Document> {
        let mut doc = self.annotate(input)?;
        self.compose(&mut doc);
        Ok(doc)
    }

    /// Analyze many documents, lazily and in order.
    ///
    /// Nothing is read or parsed until the returned iterator is pulled,
    /// and each pull runs one input through [`Engine::analyze`]. A
    /// failure is an `Err` item carrying the input's path when it has
    /// one, and the stream goes on, so partitioning is the caller's:
    ///
    /// ```no_run
    /// # fn example(engine: &matra::Engine) -> matra::domain::Result<()> {
    /// let (documents, failures): (Vec<_>, Vec<_>) = engine
    ///     .analyze_all(matra::Inputs::dir("docs/")?)
    ///     .partition(Result::is_ok);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// The returned iterator borrows the engine and is not `Send`: an
    /// `Engine` is `Send` without `Sync`, so a borrow of it cannot cross
    /// threads and the stream stays on the thread that holds the engine.
    pub fn analyze_all<I>(
        &self,
        inputs: I,
    ) -> impl Iterator<Item = std::result::Result<Document, domain::DocumentError>>
    where
        I: IntoIterator<Item = Input>,
    {
        inputs.into_iter().map(|input| {
            let path = input.path().map(Path::to_path_buf);
            self.analyze(input)
                .map_err(|e| domain::DocumentError::new(path, e))
        })
    }

    /// Read and parse one document into an unmeasured [`Document`]:
    /// fetch its bytes, detect its format, read it into blocks with their
    /// spans, build the section tree, and parse each paragraph, every
    /// metric slot still `None`.
    ///
    /// This is the only route from text to the parser, which is what
    /// makes the input size cap a property of the pipeline rather than of
    /// each entry point: no input or analysed text over
    /// [`MAX_INPUT_BYTES`] reaches [`NlpProvider::parse`].
    ///
    /// Crate-private, with [`Engine::compose`]: a caller analyzes. A
    /// public parse-only call would publish a second shape of `Document`,
    /// every metric empty, under the same name (EPR-0006).
    ///
    /// Each paragraph is parsed on its own (never a whole document joined
    /// and matched back), and the token and sentence spans the parser
    /// reports against the paragraph's text are mapped here, the one
    /// place it is parsed, into the analysed text. A quote is not parsed.
    ///
    /// Structure materializes here (RFC-0008): derived facts whose
    /// detectors live outside the domain are filled onto each parsed
    /// sentence at this choke point. Today that is
    /// [`domain::Sentence::hearst_pairs`], computed by
    /// [`hearst::hypernymy_pairs`]; sentence-level facts whose
    /// detectors live in the domain are computed by `Sentence::new`
    /// inside the provider.
    ///
    /// The provenance is recorded here too, once: the parser's
    /// [`NlpProvider::identity`], the format, and the input's path and
    /// the analysed text's length and SHA-256.
    ///
    /// # Errors
    ///
    /// [`domain::Error::InputTooLarge`] over the cap;
    /// [`domain::Error::UnsupportedFormat`] for a format no reader ships
    /// for, or bytes that are not UTF-8; [`domain::Error::InvalidInput`]
    /// for a directory named as a file and for block input that does not
    /// parse; [`domain::Error::Io`] when a file cannot be read; and
    /// whatever the parser returns.
    pub(crate) fn annotate(&self, input: Input) -> domain::Result<Document> {
        let path = input.path().map(Path::to_path_buf);
        let (format, input_text, read) = match input.0 {
            InputKind::Text { text, format } => {
                check_input_size(text.len())?;
                let read = self.reader(&format)?.decompose(&text)?;
                (format, text, read)
            }
            InputKind::Bytes { data, format } => self.read_bytes(data, format.as_ref(), None)?,
            InputKind::File { path, format } => {
                let fetched = source::file::FileSource
                    .read(&path)?
                    .into_iter()
                    .next()
                    .ok_or_else(|| {
                        domain::Error::Io(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "source returned no documents",
                        ))
                    })?;
                self.read_bytes(fetched.bytes, format.as_ref(), Some(&fetched.path))?
            }
            InputKind::Blocks(blocks) => (
                Format::Blocks,
                String::new(),
                decompose::blocks::assemble(&blocks)?,
            ),
        };
        let text = read.text.unwrap_or(input_text);
        check_input_size(text.len())?;

        let provenance = domain::Provenance::new(
            self.nlp.identity(),
            format,
            domain::ProvenanceSource::new(path, text.len(), digest::sha256_hex(text.as_bytes())),
        );
        let (sections, maps) = structure::sections(read.blocks);
        let mut doc = Document::new(provenance, sections, read.skipped);
        for (para, map) in doc.paragraphs_mut().zip(&maps) {
            if para.kind == ParagraphKind::Quote {
                continue;
            }
            let mut sentences = self.nlp.parse(&para.text)?;
            for sentence in &mut sentences {
                sentence.span = map.map(sentence.span);
                for token in &mut sentence.tokens {
                    token.span = map.map(token.span);
                }
                sentence.hearst_pairs = hearst::hypernymy_pairs(&sentence.tokens);
            }
            para.sentences = sentences;
        }
        Ok(doc)
    }

    /// Detect the format of `bytes`, decode them, and read them: the
    /// shared half of byte and file input.
    fn read_bytes(
        &self,
        bytes: Vec<u8>,
        declared: Option<&Format>,
        name: Option<&Path>,
    ) -> domain::Result<(Format, String, decompose::Read)> {
        check_input_size(bytes.len())?;
        let format = detect::format_of(&bytes, declared, name);
        let reader = self.reader(&format)?;
        let text = detect::decode(bytes)?;
        let read = reader.decompose(&text)?;
        Ok((format, text, read))
    }

    /// The reader for `format`, or the error naming what would read it.
    fn reader(&self, format: &Format) -> domain::Result<&dyn Decomposer> {
        self.readers
            .get(format)
            .ok_or_else(|| domain::Error::UnsupportedFormat {
                format: Some(format.clone()),
                feature: reader_feature(format),
            })
    }

    /// Run the metric suite over an annotated document. Total: metrics
    /// read what is attached and skip what is not; there is no failure
    /// path.
    pub(crate) fn compose(&self, doc: &mut Document) {
        let suite = metrics::default_suite();
        metrics::run_suite(doc, &suite);
    }
}

#[cfg(feature = "udpipe")]
impl Engine {
    /// The standard pipeline: UDPipe with its pinned English model, plus
    /// the readers this build ships (markdown, plain text and block
    /// input).
    ///
    /// The model is located the way every matra surface locates it,
    /// from the environment alone: `MATRA_MODEL_DIR`, else the `models`
    /// directory of the data root (`MATRA_DATA_DIR`, else
    /// `$XDG_DATA_HOME/matra`, else `~/.local/share/matra`), else an
    /// existing `~/.matra/models`. It is downloaded there on first use
    /// and verified against a digest pinned in the source. No
    /// configuration file is read: a caller who wants one passes it,
    /// with `Engine::from_config(&Config::resolve()?)`.
    ///
    /// ```no_run
    /// let engine = matra::Engine::new()?;
    /// # Ok::<(), matra::domain::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`domain::Error::InvalidInput`] when the environment names no
    /// home directory at all, and whatever [`Engine::from_config`]
    /// returns.
    pub fn new() -> domain::Result<Engine> {
        let cfg = config::Config::from_sources(|key| std::env::var(key).ok(), None)?;
        Engine::from_config(&cfg)
    }

    /// The standard pipeline over the model directory a resolved
    /// [`config::Config`] names: [`Engine::new`], with the locations
    /// the caller resolved.
    ///
    /// ```no_run
    /// let cfg = matra::config::Config::resolve()?;
    /// let engine = matra::Engine::from_config(&cfg)?;
    /// # Ok::<(), matra::domain::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`domain::Error::Io`] when the model directory cannot be created
    /// or written, or the download fails; [`domain::Error::ModelInvalid`]
    /// when the downloaded bytes fail the pinned digest or do not load.
    pub fn from_config(cfg: &config::Config) -> domain::Result<Engine> {
        let nlp = nlp::udpipe::Udpipe::from_config(cfg)?;
        Ok(Engine::with_parser(Box::new(nlp)))
    }

    /// [`Engine::from_config`], with `notice` called once before each
    /// model fetch and not at all when the model is already on disk.
    ///
    /// The route from an application that renders to the adapter that
    /// provisions. A first run downloads 16 MB, and the library writes
    /// to no terminal, so the facts travel out as a
    /// [`domain::ProvisionNotice`] and the caller decides the wording.
    ///
    /// ```no_run
    /// let cfg = matra::config::Config::resolve()?;
    /// let engine = matra::Engine::from_config_with_notice(&cfg, |n| {
    ///     eprintln!("fetching {} into {}", n.artifact, n.destination.display());
    /// })?;
    /// # Ok::<(), matra::domain::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Whatever [`Engine::from_config`] returns.
    pub fn from_config_with_notice(
        cfg: &config::Config,
        notice: impl FnMut(&domain::ProvisionNotice),
    ) -> domain::Result<Engine> {
        let nlp = nlp::udpipe::Udpipe::from_config_with_notice(cfg, notice)?;
        Ok(Engine::with_parser(Box::new(nlp)))
    }
}

/// The reader this build ships for `format`, or `None` for a format that
/// is recognised but has no reader.
///
/// The match is deliberately exhaustive with no wildcard: adding a
/// `Format` variant fails compilation here, so a new format is a
/// conscious registration decision rather than a silent
/// `Error::UnsupportedFormat` at run time.
#[cfg(any(feature = "udpipe", test))]
fn default_reader(format: &Format) -> Option<Box<dyn Decomposer>> {
    match format {
        Format::Markdown => Some(Box::new(decompose::markdown::MarkdownDecomposer)),
        Format::PlainText => Some(Box::new(decompose::plain::PlainTextDecomposer)),
        Format::Blocks => Some(Box::new(decompose::blocks::BlocksDecomposer)),
        Format::Pdf | Format::Docx => None,
    }
}

/// The Cargo feature that would add a reader for `format`, for the
/// `UnsupportedFormat` message. No feature of this version reads PDF or
/// DOCX. Exhaustive for the same reason as [`default_reader`]: a reader
/// behind a feature is a decision to record here.
fn reader_feature(format: &Format) -> Option<&'static str> {
    match format {
        Format::Markdown | Format::PlainText | Format::Blocks | Format::Pdf | Format::Docx => None,
    }
}

/// The reader table this build ships: markdown, plain text and block
/// input. Lives in the composition root because it is the only place
/// that names every adapter (boundary rule 7).
#[cfg(any(feature = "udpipe", test))]
fn standard_readers() -> decompose::Decomposers {
    let mut table = decompose::Decomposers::new();
    for format in [
        Format::Markdown,
        Format::PlainText,
        Format::Blocks,
        Format::Pdf,
        Format::Docx,
    ] {
        if let Some(reader) = default_reader(&format) {
            table = table.with(format, reader);
        }
    }
    table
}

/// The sentences of `doc` grouped by the similarity of their embeddings:
/// each sentence is embedded with `embedder`, and two sentences whose
/// cosine similarity is at least `threshold` are joined by an edge. A
/// cluster is a connected component of that graph (see
/// [`domain::Clusters`] for what co-membership does and does not mean).
///
/// The embedder is the caller's, always: matra builds none inside an
/// outcome call, because building the default one downloads a model.
/// [`embed::Embedder`] is the one port open to outside implementers. The
/// result's provenance is the document's, with the method (`cosine`),
/// the `threshold` and the embedder's
/// [`identity`](embed::Embedder::identity) recorded, so the scores
/// cannot be attributed to a model that did not produce them. matra knows
/// no universal threshold; published values span 0.67 to 0.9.
///
/// This is the one place both halves meet, the embedding port and the
/// clustering over its vectors, which is why it lives in the composition
/// root (boundary rule 7). Sentence text reaching the embedder already
/// passed the pipeline's size cap when the document was analysed.
///
/// ```no_run
/// # #[cfg(feature = "model2vec")]
/// # fn example(doc: &matra::domain::Document) -> matra::domain::Result<()> {
/// let cfg = matra::config::Config::resolve()?;
/// let embedder = matra::embed::model2vec::Model2Vec::from_config(&cfg)?;
/// let found = matra::clusters(doc, &embedder, 0.85)?;
/// println!("{} clusters", found.clusters.len());
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// [`domain::Error::InputTooLarge`] (`what` = `"clusters"`) when the
/// document has more than 2,000 sentences, checked before anything is
/// embedded; [`domain::Error::InvalidInput`] when `threshold` is not
/// finite, or the embedder breaks its contract (the wrong number of
/// vectors, vectors of different lengths, a non-finite value); and
/// whatever the embedder returns.
pub fn clusters(
    doc: &Document,
    embedder: &dyn embed::Embedder,
    threshold: f64,
) -> domain::Result<domain::Clusters> {
    let texts: Vec<&str> = doc.sentences().map(|s| s.text.as_str()).collect();
    // The count cap fires before the expensive stage, not after it: an
    // over-cap document must not run a full embedding pass just to be
    // rejected by the clustering gate. The same check inside
    // `extraction::cluster` still guards the vectors themselves.
    if texts.len() > domain::limits::MAX_CLUSTER_SENTENCES {
        return Err(domain::Error::InputTooLarge {
            limit: domain::limits::MAX_CLUSTER_SENTENCES,
            actual: texts.len(),
            what: "clusters",
        });
    }
    if !threshold.is_finite() {
        return Err(domain::Error::InvalidInput {
            reason: "threshold_not_finite",
            message: format!("threshold must be finite, got {threshold}"),
        });
    }
    let embeddings = embedder.embed(&texts)?;
    if embeddings.len() != texts.len() {
        return Err(domain::Error::InvalidInput {
            reason: "embedder_vector_count",
            message: format!(
                "embedder returned {} for {}, violating its contract",
                counted(embeddings.len(), "vector"),
                counted(texts.len(), "sentence"),
            ),
        });
    }
    let (clusters, edges) = extraction::cluster(&embeddings, threshold)?;
    Ok(domain::Clusters {
        provenance: doc.provenance.derived(
            CLUSTERS_METHOD,
            domain::Parameters::clustering(threshold, embedder.identity()),
        ),
        clusters,
        edges,
    })
}

/// `n` with `noun`, singular for one: "1 vector", "2 vectors".
fn counted(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// The method [`clusters`] records: connected components of the graph of
/// sentence pairs whose cosine similarity clears the threshold.
const CLUSTERS_METHOD: &str = "cosine";

// ---------------------------------------------------------------------------
// The Python binding (behind the "python" feature flag)
// ---------------------------------------------------------------------------

// A driving adapter like src/cli/, held to boundary rule 7: it uses the
// public surface above and never a port or an adapter.
#[cfg(feature = "python")]
mod python;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nlp::NlpProvider;
    use domain::{Section, Span};

    /// An engine moves to a worker thread, so a server can build one per
    /// worker or hold one behind a `Mutex`. A reader or parser that is
    /// not `Send` fails this at compile time.
    #[test]
    fn an_engine_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Engine>();
    }

    /// Minimal NlpProvider that returns no sentences. Lets us test composition-root
    /// gates (input size, etc.) without requiring a real NLP backend.
    struct EmptyNlp;
    impl NlpProvider for EmptyNlp {
        fn parse(&self, _text: &str) -> domain::Result<Vec<domain::Sentence>> {
            Ok(Vec::new())
        }
        fn identity(&self) -> domain::Identity {
            domain::Identity::new("empty", None, None, None)
        }
    }

    /// Splits the input on `.` and returns one [`Sentence`] per non-empty
    /// piece. Tokens are not constructed (the pipeline tests below only
    /// look at sentence text and counts). Lets us assert the
    /// per-paragraph parse contract without a real NLP backend.
    struct DotSplitNlp;
    impl NlpProvider for DotSplitNlp {
        fn parse(&self, text: &str) -> domain::Result<Vec<domain::Sentence>> {
            Ok(text
                .split('.')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| domain::Sentence::new(s.to_string(), Vec::new()))
                .collect())
        }
        fn identity(&self) -> domain::Identity {
            domain::Identity::new("dot-split", None, None, None)
        }
    }

    /// One sentence per paragraph, one token per whitespace-separated
    /// word, each token carrying its span in the text it was given, as
    /// the provider contract asks. Lets the span tests observe the
    /// engine mapping those spans into the analysed text.
    struct WordNlp;
    impl NlpProvider for WordNlp {
        fn parse(&self, text: &str) -> domain::Result<Vec<domain::Sentence>> {
            let mut tokens = Vec::new();
            let mut at = 0;
            for word in text.split_whitespace() {
                let start = at + text[at..].find(word).unwrap_or(0);
                let end = start + word.len();
                tokens.push(
                    domain::Token::builder(
                        tokens.len() + 1,
                        word.to_string(),
                        word.to_lowercase(),
                        "X".to_string(),
                        0,
                        "dep".to_string(),
                    )
                    .span(Span::new(start, end))
                    .build(),
                );
                at = end;
            }
            if tokens.is_empty() {
                return Ok(Vec::new());
            }
            Ok(vec![domain::Sentence::new(text.to_string(), tokens)])
        }
        fn identity(&self) -> domain::Identity {
            domain::Identity::new("words", None, None, None)
        }
    }

    use std::sync::atomic::{AtomicUsize, Ordering};

    /// DotSplitNlp that counts parse calls and records the largest text
    /// it was handed, through a handle the test keeps. Refuses any text
    /// containing the marker. Lets the law tests observe exactly when
    /// (and with what) the pipeline reaches the parser, even though the
    /// engine owns the provider.
    struct ObservableNlp {
        calls: std::sync::Arc<AtomicUsize>,
        max_len_seen: std::sync::Arc<AtomicUsize>,
    }
    impl ObservableNlp {
        fn new() -> (
            Self,
            std::sync::Arc<AtomicUsize>,
            std::sync::Arc<AtomicUsize>,
        ) {
            let calls = std::sync::Arc::new(AtomicUsize::new(0));
            let max_len = std::sync::Arc::new(AtomicUsize::new(0));
            (
                Self {
                    calls: calls.clone(),
                    max_len_seen: max_len.clone(),
                },
                calls,
                max_len,
            )
        }
    }
    impl NlpProvider for ObservableNlp {
        fn parse(&self, text: &str) -> domain::Result<Vec<domain::Sentence>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.max_len_seen.fetch_max(text.len(), Ordering::SeqCst);
            if text.contains("POISON") {
                return Err(domain::Error::ParseFailed("poisoned".to_string()));
            }
            DotSplitNlp.parse(text)
        }
        fn identity(&self) -> domain::Identity {
            domain::Identity::new("observable", None, None, None)
        }
    }

    fn law_engine() -> Engine {
        Engine::with_parser(Box::new(DotSplitNlp))
    }

    /// Structural fingerprint for law equality: serialized documents plus
    /// stringified errors. `Document` has no `PartialEq`; serde is the
    /// comparison it does have.
    fn fingerprint(
        results: Vec<std::result::Result<Document, domain::DocumentError>>,
    ) -> Vec<String> {
        results
            .into_iter()
            .map(|r| match r {
                Ok(doc) => format!("ok:{}", serde_json::to_string(&doc).unwrap()),
                Err(e) => format!("err:{e}"),
            })
            .collect()
    }

    /// One way to build each `Input` constructor from the same text: the
    /// laws below are stated once and run for every one of them. The
    /// file lives in `dir`, which the caller keeps alive.
    fn constructors(text: &str, dir: &Path) -> Vec<(&'static str, Input)> {
        let file = dir.join(format!("{}.txt", text.len()));
        std::fs::write(&file, text).unwrap();
        vec![
            ("text", Input::text(text, Format::PlainText)),
            ("bytes", Input::bytes(text.as_bytes(), None)),
            ("file", Input::file(&file)),
            ("file_as", Input::file_as(&file, Format::PlainText)),
            (
                "blocks",
                Input::blocks(text.split("\n\n").map(Block::paragraph)),
            ),
        ]
    }

    #[test]
    fn law_l1_analyze_all_commutes_with_chain() {
        let engine = law_engine();
        let dir = tempfile::tempdir().unwrap();
        let firsts = constructors("First stream. Has two.", dir.path());
        let seconds = constructors("Second stream here.", dir.path());
        for ((name, a), (_, b)) in firsts.into_iter().zip(seconds) {
            let chained: Vec<_> = engine
                .analyze_all([a.clone()].into_iter().chain([b.clone()]))
                .collect();
            let separate: Vec<_> = engine
                .analyze_all([a])
                .chain(engine.analyze_all([b]))
                .collect();
            assert_eq!(fingerprint(chained), fingerprint(separate), "{name}");
        }
    }

    #[test]
    fn law_l2_analyze_all_of_empty_is_empty() {
        let engine = law_engine();
        assert_eq!(engine.analyze_all(std::iter::empty()).count(), 0);
    }

    #[test]
    fn law_l3_analyze_all_of_one_is_analyze() {
        let engine = law_engine();
        let dir = tempfile::tempdir().unwrap();
        for (name, input) in constructors("One document. Two sentences.", dir.path()) {
            let path = input.path().map(Path::to_path_buf);
            let streamed: Vec<_> = engine.analyze_all([input.clone()]).collect();
            let direct = vec![
                engine
                    .analyze(input)
                    .map_err(|e| domain::DocumentError::new(path, e)),
            ];
            assert_eq!(fingerprint(streamed), fingerprint(direct), "{name}");
        }
    }

    #[test]
    fn law_l4_analyze_is_annotate_then_compose() {
        let engine = law_engine();
        let dir = tempfile::tempdir().unwrap();
        for (name, input) in constructors("Composed of parts. Checked by law.", dir.path()) {
            let via_analyze = engine.analyze(input.clone()).unwrap();
            let via_stages = {
                let mut doc = engine.annotate(input).unwrap();
                engine.compose(&mut doc);
                doc
            };
            assert_eq!(
                serde_json::to_string(&via_analyze).unwrap(),
                serde_json::to_string(&via_stages).unwrap(),
                "{name}"
            );
        }
    }

    #[test]
    fn law_l5_partition_documents_plus_errors_equals_inputs() {
        let (nlp, _, _) = ObservableNlp::new();
        let engine = Engine::with_parser(Box::new(nlp));
        let dir = tempfile::tempdir().unwrap();
        let mut inputs: Vec<Input> = constructors("Good document one.", dir.path())
            .into_iter()
            .map(|(_, input)| input)
            .collect();
        let good = inputs.len();
        inputs.extend(
            constructors("This one is POISON and fails.", dir.path())
                .into_iter()
                .map(|(_, input)| input),
        );
        inputs.push(Input::file(dir.path().join("lost.md")));
        let n = inputs.len();
        let (documents, errors): (Vec<_>, Vec<_>) =
            engine.analyze_all(inputs).partition(Result::is_ok);
        assert_eq!(documents.len() + errors.len(), n);
        assert_eq!(documents.len(), good);
        assert_eq!(errors.len(), good + 1);
    }

    /// A failed read never reaches the parser, whatever the reason: a
    /// missing file, a directory named as a file, bytes that are not
    /// text, a signature no reader ships for. Each is an `Err` item, the
    /// file's carrying its path.
    #[test]
    fn law_l6_a_failed_read_never_reaches_the_parser() {
        let (nlp, calls, _) = ObservableNlp::new();
        let engine = Engine::with_parser(Box::new(nlp));
        let dir = tempfile::tempdir().unwrap();
        let ghost = dir.path().join("ghost.md");
        let inputs = vec![
            Input::file(&ghost),
            Input::file(dir.path()),
            Input::bytes(vec![0xff, 0xfe, b'x'], None),
            Input::bytes(b"%PDF-1.7".to_vec(), Some(Format::PlainText)),
            Input::text("[{\"kind\": \"table\"}]", Format::Blocks),
        ];
        let out: Vec<_> = engine.analyze_all(inputs).collect();
        assert_eq!(out.len(), 5);
        assert!(out.iter().all(Result::is_err));
        let first = out[0].as_ref().unwrap_err();
        assert_eq!(first.path.as_deref(), Some(ghost.as_path()));
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "a failed read must not reach the parser"
        );

        engine
            .annotate(Input::text("Control.", Format::PlainText))
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1, "counter is live");
    }

    /// The size cap holds for every constructor: on the bytes handed in,
    /// on a file's size, and on the analysed text a reader produces.
    #[test]
    fn law_l7_no_oversized_input_reaches_the_parser() {
        let (nlp, _, max_len) = ObservableNlp::new();
        let engine = Engine::with_parser(Box::new(nlp));
        let dir = tempfile::tempdir().unwrap();
        let oversized = "a".repeat(MAX_INPUT_BYTES + 1);
        for (name, input) in constructors(&oversized, dir.path()) {
            let out: Vec<_> = engine.analyze_all([input.clone()]).collect();
            assert!(
                matches!(
                    out[0].as_ref().unwrap_err().error,
                    domain::Error::InputTooLarge { .. }
                ),
                "{name} through the stream"
            );
            assert!(
                matches!(
                    engine.annotate(input),
                    Err(domain::Error::InputTooLarge { .. })
                ),
                "{name} through annotate"
            );
        }
        // Blocks under the cap one by one that join to more than it.
        let half = "b".repeat(MAX_INPUT_BYTES / 2 + 1);
        let joined = Input::blocks([Block::paragraph(half.clone()), Block::paragraph(half)]);
        assert!(matches!(
            engine.annotate(joined),
            Err(domain::Error::InputTooLarge { .. })
        ));
        assert!(
            max_len.load(Ordering::SeqCst) <= MAX_INPUT_BYTES,
            "the parser never saw text over the cap"
        );
    }

    // ------------------------------------------------------------------
    // L8, the accounting law
    // ------------------------------------------------------------------

    /// L8: every byte of the analysed text that is not whitespace lies in
    /// exactly one heading span, paragraph span or skipped span. `Err`
    /// names the first byte that does not, with its count.
    fn accounting(text: &str, doc: &Document) -> std::result::Result<(), String> {
        let mut owners = vec![0u8; text.len()];
        let spans = doc
            .sections
            .iter()
            .filter_map(|s| s.span)
            .chain(doc.paragraphs().map(|p| p.span))
            .chain(doc.skipped.iter().map(|s| s.span));
        for span in spans {
            if span.byte_start > span.byte_end || span.byte_end > text.len() {
                return Err(format!("span {span:?} is outside the text"));
            }
            for owner in &mut owners[span.byte_start..span.byte_end] {
                *owner = owner.saturating_add(1);
            }
        }
        for (at, c) in text.char_indices() {
            if !c.is_whitespace() && owners[at] != 1 {
                let mut from = at.saturating_sub(20);
                while !text.is_char_boundary(from) {
                    from -= 1;
                }
                let mut to = (at + 20).min(text.len());
                while !text.is_char_boundary(to) {
                    to += 1;
                }
                return Err(format!(
                    "byte {at} ({c:?}) lies in {} spans, near {:?}",
                    owners[at],
                    &text[from..to]
                ));
            }
        }
        Ok(())
    }

    fn accounting_engine() -> Engine {
        Engine::with_parser(Box::new(EmptyNlp))
    }

    /// L8 over every conformance fixture, in the format it names.
    #[test]
    fn law_l9_holds_over_every_conformance_fixture() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("spec/tests");
        let engine = accounting_engine();
        let mut checked = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let fixture: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let input = fixture["input"].as_str().unwrap();
            let format: Format = serde_json::from_value(fixture["format"].clone()).unwrap();
            let doc = engine.annotate(Input::text(input, format)).unwrap();
            accounting(input, &doc).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            checked += 1;
        }
        assert!(checked > 0, "no fixtures in {}", dir.display());
    }

    /// A small deterministic generator (xorshift64), so the property test
    /// needs no dependency and a failure names the seed that found it.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
            items[usize::try_from(self.next() % items.len() as u64).unwrap()]
        }
    }

    const MARKDOWN_PIECES: &[&str] = &[
        "# Heading",
        "## References",
        "### Deep heading ###",
        "#hashtag opens this line",
        "Plain prose with **bold**, *em* and `code`.",
        "Text with [a link](https://x.test) and ![alt](i.png) &amp; \\* and <br> html.",
        "> quoted line\n> more quote",
        "> one\n>\n> two",
        ">",
        "> # quoted heading",
        "> - quoted item",
        "- item one\n- item two\n  continued",
        "1. first\n\n2. second",
        "1) paren item",
        "+ plus item",
        "  * star item",
        "- [ ] open task\n- [x] done task",
        "-",
        "1.",
        "- > quote in a list",
        "- ```\n  code in a list\n  ```",
        "| a | b |\n|---|---|\n| 1 | 2 |",
        "```rust\nfn main() {}\n```",
        "~~~\ntilde fence\n~~~",
        "    indented code",
        "<div>\nhtml block\n</div>",
        "<!-- a comment -->",
        "---",
        "***",
        "___",
        "[x]: https://example.com \"title\"",
        "> [y]: /quoted-def",
        "Setext heading\n===",
        "Another setext\n---",
        "line one\r\nline two",
        "café, naïve, 日本語 and an emoji 🙂",
        "~~struck~~ through",
        "trailing hard break  \nnext line",
        "a line ending in a backslash\\\nnext",
        "[^1] footnote-looking text",
        "\t\ttabbed text",
        "*",
        "> > nested quote",
    ];

    const SEPARATORS: &[&str] = &["\n\n", "\n", "\r\n\r\n", "\n\n\n", "\n \n"];

    fn generated_markdown(rng: &mut Rng) -> String {
        let mut doc = String::new();
        if rng.next().is_multiple_of(5) {
            doc.push_str(rng.pick(&["---\ntitle: x\n---\n\n", "+++\ntitle = 1\n+++\n\n"]));
        }
        let pieces = 1 + rng.next() % 8;
        for i in 0..pieces {
            if i > 0 {
                doc.push_str(rng.pick(SEPARATORS));
            }
            doc.push_str(rng.pick(MARKDOWN_PIECES));
        }
        if rng.next().is_multiple_of(2) {
            doc.push('\n');
        }
        doc
    }

    /// Markup fragments with no structure imposed, so the generator also
    /// reaches the malformed input a reader is total over.
    const SOUP: &[&str] = &[
        "#", "# ", "## ", ">", "> ", "-", "- ", "*", "* ", "+", "1.", "1)", "|", " | ", "`", "```",
        "~~~", "[", "]", "(", ")", "![", "<", "</", "&", "&amp;", "\\", "_", "__", "=", "===",
        "---", "+++", "***", ":", "\"", "[x]: ", "word", "text", "é", " ", "  ", "    ", "\t",
        "\n", "\n", "\n\n", "\r\n",
    ];

    fn generated_soup(rng: &mut Rng) -> String {
        let pieces = 1 + rng.next() % 40;
        (0..pieces).map(|_| rng.pick(SOUP)).collect()
    }

    /// L8 as a property: thousands of generated markdown and plain text
    /// documents, half built from constructs the readers recognise and
    /// the edge cases they once got wrong, half from markup fragments in
    /// any order.
    #[test]
    fn law_l9_holds_over_generated_documents() {
        let engine = accounting_engine();
        for seed in 1..=6000_u64 {
            let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
            let markdown = if seed % 2 == 0 {
                generated_markdown(&mut rng)
            } else {
                generated_soup(&mut rng)
            };
            let doc = engine
                .annotate(Input::text(markdown.clone(), Format::Markdown))
                .unwrap();
            accounting(&markdown, &doc)
                .unwrap_or_else(|e| panic!("seed {seed}, markdown {markdown:?}: {e}"));
            let doc = engine
                .annotate(Input::text(markdown.clone(), Format::PlainText))
                .unwrap();
            accounting(&markdown, &doc)
                .unwrap_or_else(|e| panic!("seed {seed}, plain text {markdown:?}: {e}"));
        }
    }

    // ------------------------------------------------------------------
    // Spans
    // ------------------------------------------------------------------

    /// The parser reports spans against the paragraph's text; the engine
    /// maps them into the analysed text, through the markup the reader
    /// removed.
    #[test]
    fn token_and_sentence_spans_point_into_the_analysed_text() {
        let engine = Engine::with_parser(Box::new(WordNlp));
        let md = "# Title\n\nSome **bold** and `code` here.\n\n- item text";
        let doc = engine.analyze(Input::text(md, Format::Markdown)).unwrap();
        let tokens: Vec<&str> = doc
            .tokens()
            .map(|t| &md[t.span.byte_start..t.span.byte_end])
            .collect();
        assert_eq!(
            tokens,
            ["Some", "bold", "and", "code", "here.", "item", "text"]
        );
        let sentence = doc.sentences().next().unwrap();
        assert_eq!(
            &md[sentence.span.byte_start..sentence.span.byte_end],
            "Some **bold** and `code` here."
        );
    }

    #[test]
    fn spans_count_bytes_not_characters() {
        let engine = Engine::with_parser(Box::new(WordNlp));
        let text = "Café naïve.\n\nÜber.";
        let doc = engine
            .analyze(Input::text(text, Format::PlainText))
            .unwrap();
        for token in doc.tokens() {
            assert_eq!(
                &text[token.span.byte_start..token.span.byte_end],
                token.text
            );
        }
        let second = doc.paragraphs().nth(1).unwrap();
        assert_eq!(second.span, Span::new(15, 21));
    }

    /// Whitespace-separated words with their spans, a sentence ending at
    /// each word that ends in `.`, so one paragraph holds several
    /// sentences whose spans do not start at the paragraph's start.
    struct SentenceNlp;
    impl NlpProvider for SentenceNlp {
        fn parse(&self, text: &str) -> domain::Result<Vec<domain::Sentence>> {
            let mut sentences = Vec::new();
            let mut tokens: Vec<domain::Token> = Vec::new();
            let mut at = 0;
            for word in text.split_whitespace() {
                let start = at + text[at..].find(word).unwrap_or(0);
                at = start + word.len();
                tokens.push(
                    domain::Token::builder(
                        tokens.len() + 1,
                        word.to_string(),
                        word.to_lowercase(),
                        "X".to_string(),
                        0,
                        "dep".to_string(),
                    )
                    .span(Span::new(start, at))
                    .build(),
                );
                if word.ends_with('.') {
                    let words: Vec<&str> = tokens.iter().map(|t| t.text.as_str()).collect();
                    sentences.push(domain::Sentence::new(
                        words.join(" "),
                        std::mem::take(&mut tokens),
                    ));
                }
            }
            Ok(sentences)
        }
        fn identity(&self) -> domain::Identity {
            domain::Identity::new("sentences", None, None, None)
        }
    }

    /// The regression the field names exist for: after the first
    /// character outside ASCII, a byte offset and a character offset
    /// part, so a sentence's span sliced as bytes is its text and the
    /// same numbers taken as characters are not.
    #[test]
    fn non_ascii_sentence_spans_slice_as_bytes() {
        let engine = Engine::with_parser(Box::new(SentenceNlp));
        let text = "Café ☕ is open. Naïve plan.";
        let doc = engine
            .analyze(Input::text(text, Format::PlainText))
            .unwrap();
        let sentences: Vec<_> = doc.sentences().collect();
        assert_eq!(sentences.len(), 2);
        for sentence in &sentences {
            let span = sentence.span;
            assert_eq!(&text[span.byte_start..span.byte_end], sentence.text);
        }
        let second = sentences[1].span;
        assert_eq!(second, Span::new(19, 31));
        let by_chars: String = text
            .chars()
            .skip(second.byte_start)
            .take(second.byte_end - second.byte_start)
            .collect();
        assert_ne!(by_chars, sentences[1].text);
    }

    /// Block input's analysed text is the blocks joined by a blank line,
    /// and every span and the provenance describe that text.
    #[test]
    fn block_input_spans_point_into_the_joined_text() {
        let engine = Engine::with_parser(Box::new(WordNlp));
        let doc = engine
            .analyze(Input::blocks([
                Block::heading(1, "Field notes"),
                Block::paragraph("The river rose overnight."),
            ]))
            .unwrap();
        let joined = "Field notes\n\nThe river rose overnight.";
        assert_eq!(doc.provenance.format, Format::Blocks);
        assert_eq!(doc.provenance.source.bytes, joined.len());
        assert_eq!(
            doc.provenance.source.sha256,
            digest::sha256_hex(joined.as_bytes())
        );
        assert_eq!(doc.sections[0].heading.as_deref(), Some("Field notes"));
        assert_eq!(doc.sections[0].level, 1);
        assert_eq!(doc.sections[0].span, Some(Span::new(0, 11)));
        let token = doc.tokens().nth(1).unwrap();
        assert_eq!(&joined[token.span.byte_start..token.span.byte_end], "river");
    }

    #[test]
    fn a_quote_is_not_parsed() {
        let (nlp, calls, _) = ObservableNlp::new();
        let engine = Engine::with_parser(Box::new(nlp));
        let doc = engine
            .analyze(Input::text("> Quoted words.", Format::Markdown))
            .unwrap();
        let para = doc.paragraphs().next().unwrap();
        assert_eq!(para.kind, ParagraphKind::Quote);
        assert!(para.sentences.is_empty());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn what_the_reader_skipped_reaches_the_document() {
        let doc = accounting_engine()
            .analyze(Input::text(
                "Text.\n\n```\ncode\n```\n\n| a |\n|---|\n| 1 |",
                Format::Markdown,
            ))
            .unwrap();
        let kinds: Vec<domain::SkippedKind> = doc.skipped.iter().map(|s| s.kind).collect();
        assert_eq!(
            kinds,
            [domain::SkippedKind::Code, domain::SkippedKind::Table]
        );
    }

    // ------------------------------------------------------------------
    // Detection and provenance
    // ------------------------------------------------------------------

    /// The provenance is recorded once, at annotation: the parser's own
    /// identity, the format, and the input's path, length and digest.
    #[test]
    fn annotate_records_the_provenance() {
        let engine = law_engine();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("notes.md");
        std::fs::write(&file, "abc").unwrap();
        let doc = engine.annotate(Input::file(&file)).unwrap();
        let p = &doc.provenance;
        assert_eq!(p.parser.name, "dot-split", "the parser names itself");
        assert_eq!(p.format, Format::Markdown);
        assert_eq!(p.schema, domain::SCHEMA_VERSION);
        assert_eq!(p.matra, env!("CARGO_PKG_VERSION"));
        assert_eq!(p.source.path.as_deref(), Some(file.as_path()));
        assert_eq!(p.source.bytes, 3);
        // FIPS 180-2 SHA-256 of "abc".
        assert_eq!(
            p.source.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn a_recognised_format_with_no_reader_is_unsupported() {
        let engine = law_engine();
        for (input, format) in [
            (Input::text("text", Format::Pdf), Format::Pdf),
            (Input::bytes(b"%PDF-1.4 ...".to_vec(), None), Format::Pdf),
            (
                Input::bytes(b"PK\x03\x04..[Content_Types].xml..".to_vec(), None),
                Format::Docx,
            ),
        ] {
            match engine.annotate(input) {
                Err(domain::Error::UnsupportedFormat {
                    format: Some(got),
                    feature: None,
                }) => assert_eq!(got, format),
                other => panic!("expected UnsupportedFormat, got {:?}", other.map(|_| ())),
            }
        }
    }

    #[test]
    fn bytes_that_are_not_text_are_unsupported() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("binary.txt");
        std::fs::write(&file, [0xff, 0xfe, 0x00, 0x01]).unwrap();
        for input in [Input::file(&file), Input::bytes(vec![0xc3, 0x28], None)] {
            assert!(matches!(
                law_engine().annotate(input),
                Err(domain::Error::UnsupportedFormat { format: None, .. })
            ));
        }
    }

    /// Decision 7: a UTF-8 file with an extension matra does not know
    /// reads as plain text.
    #[test]
    fn an_unknown_extension_reads_as_plain_text() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("notes.log");
        std::fs::write(&file, "# not a heading here").unwrap();
        let doc = law_engine().analyze(Input::file(&file)).unwrap();
        assert_eq!(doc.provenance.format, Format::PlainText);
        assert!(doc.sections[0].heading.is_none());
    }

    /// `Input::file_as` reads a file in the format the caller names, so
    /// a JSON file of blocks is block input and a `.md` file can be read
    /// as plain text. The path is recorded as it is for `Input::file`.
    #[test]
    fn a_file_reads_in_the_format_the_caller_names() {
        let dir = tempfile::tempdir().unwrap();
        let blocks = dir.path().join("notes.json");
        std::fs::write(
            &blocks,
            r#"[{"kind": "heading", "level": 1, "text": "Field notes"},
                {"kind": "paragraph", "text": "The river rose."}]"#,
        )
        .unwrap();
        let doc = law_engine()
            .analyze(Input::file_as(&blocks, Format::Blocks))
            .unwrap();
        assert_eq!(doc.provenance.format, Format::Blocks);
        assert_eq!(doc.sections[0].heading.as_deref(), Some("Field notes"));
        assert_eq!(
            doc.provenance.source.path.as_deref(),
            Some(blocks.as_path())
        );

        let markdown = dir.path().join("notes.md");
        std::fs::write(&markdown, "# not a heading here").unwrap();
        let doc = law_engine()
            .analyze(Input::file_as(&markdown, Format::PlainText))
            .unwrap();
        assert_eq!(doc.provenance.format, Format::PlainText);
        assert!(doc.sections[0].heading.is_none());
    }

    /// A signature in the bytes outranks the format the caller names, for
    /// a file as for bytes.
    #[test]
    fn a_signature_outranks_the_format_a_file_is_read_as() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("scan.txt");
        std::fs::write(&file, b"%PDF-1.7\n").unwrap();
        match law_engine().analyze(Input::file_as(&file, Format::Markdown)) {
            Err(domain::Error::UnsupportedFormat {
                format: Some(Format::Pdf),
                ..
            }) => {}
            other => panic!(
                "expected UnsupportedFormat(pdf), got {:?}",
                other.map(|_| ())
            ),
        }
    }

    #[test]
    fn a_markdown_file_reads_as_markdown() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("notes.md");
        std::fs::write(&file, "# Title\n\nBody.").unwrap();
        let doc = law_engine().analyze(Input::file(&file)).unwrap();
        assert_eq!(doc.provenance.format, Format::Markdown);
        assert_eq!(doc.sections[0].heading.as_deref(), Some("Title"));
    }

    // ------------------------------------------------------------------
    // Input and Inputs
    // ------------------------------------------------------------------

    #[test]
    fn input_file_on_a_directory_names_inputs_dir() {
        let dir = tempfile::tempdir().unwrap();
        match law_engine().analyze(Input::file(dir.path())) {
            Err(domain::Error::InvalidInput { message, .. }) => {
                assert!(message.contains("Inputs::dir"), "{message}");
            }
            other => panic!("expected InvalidInput, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn inputs_dir_lists_sorted_files_and_reads_them_lazily() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("b.txt"), "B").unwrap();
        std::fs::write(dir.path().join("a.md"), "# A").unwrap();
        let inputs = Inputs::dir(dir.path()).unwrap();
        assert_eq!(inputs.len(), 2);
        let docs: Vec<Document> = law_engine()
            .analyze_all(inputs)
            .map(Result::unwrap)
            .collect();
        let paths: Vec<_> = docs
            .iter()
            .map(|d| d.provenance.source.path.clone().unwrap())
            .collect();
        assert!(paths[0].ends_with("a.md"));
        assert!(paths[1].ends_with("b.txt"));
        assert_eq!(docs[0].provenance.format, Format::Markdown);
    }

    #[test]
    fn inputs_dir_on_a_missing_path_or_a_file_is_a_constructor_error() {
        let missing = Path::new("/definitely/not/here");
        let err = Inputs::dir(missing).unwrap_err();
        assert_eq!(err.kind(), "io");
        let details = err.details();
        assert_eq!(
            details.get("io_kind"),
            Some(&domain::DetailValue::Text("not_found".to_string()))
        );
        assert_eq!(
            details.get("path"),
            Some(&domain::DetailValue::Text(missing.display().to_string()))
        );
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("one.txt");
        std::fs::write(&file, "x").unwrap();
        match Inputs::dir(&file) {
            Err(domain::Error::InvalidInput { message, .. }) => {
                assert!(message.contains("Input::file"), "{message}");
            }
            other => panic!("expected InvalidInput, got {:?}", other.map(|_| ())),
        }
    }

    /// A symlink in the directory is neither read nor reported: the walk
    /// is over the regular files a listing names, and a link is not one.
    #[cfg(unix)]
    #[test]
    fn inputs_dir_skips_a_symlink() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("real.md"), "# Real").unwrap();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("outside.md");
        std::fs::write(&target, "# Outside").unwrap();
        std::os::unix::fs::symlink(&target, dir.path().join("link.md")).unwrap();

        let items: Vec<_> = law_engine()
            .analyze_all(Inputs::dir(dir.path()).unwrap())
            .collect();
        assert_eq!(items.len(), 1);
        let doc = items.into_iter().next().unwrap().unwrap();
        assert!(doc.provenance.source.path.unwrap().ends_with("real.md"));
    }

    /// Regression: the directory adapter's own `Source::read` dropped a
    /// file it could not read and returned the rest, the silent omission
    /// L8 forbids. That route is gone; through `Inputs::dir` the failed
    /// file is an item carrying its path.
    #[cfg(unix)]
    #[test]
    fn a_per_file_failure_is_an_err_item_not_an_abort() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("good.md"), "# Good").unwrap();
        let bad = dir.path().join("bad.md");
        std::fs::write(&bad, "# Bad").unwrap();
        std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o000)).unwrap();

        let items: Vec<_> = law_engine()
            .analyze_all(Inputs::dir(dir.path()).unwrap())
            .collect();
        let _ = std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o644));

        assert_eq!(items.len(), 2);
        assert_eq!(items.iter().filter(|r| r.is_ok()).count(), 1);
        let err = items.into_iter().find_map(|r| r.err()).unwrap();
        assert!(err.path.unwrap().ends_with("bad.md"));
    }

    #[test]
    fn standard_readers_cover_exactly_the_shipping_formats() {
        let table = standard_readers();
        assert!(table.get(&Format::Markdown).is_some());
        assert!(table.get(&Format::PlainText).is_some());
        assert!(table.get(&Format::Blocks).is_some());
        assert!(
            table.get(&Format::Pdf).is_none(),
            "Pdf is recognised, not read"
        );
        assert!(
            table.get(&Format::Docx).is_none(),
            "Docx is recognised, not read"
        );
    }

    #[test]
    fn input_at_cap_is_accepted() {
        // "a" repeated MAX_INPUT_BYTES times = exactly the cap.
        let engine = Engine::with_parser(Box::new(EmptyNlp));
        assert!(
            engine
                .annotate(Input::text("a".repeat(MAX_INPUT_BYTES), Format::PlainText))
                .is_ok(),
            "input exactly at cap should be accepted"
        );
    }

    #[test]
    fn input_one_byte_over_cap_is_rejected() {
        let engine = Engine::with_parser(Box::new(EmptyNlp));
        match engine.annotate(Input::text(
            "a".repeat(MAX_INPUT_BYTES + 1),
            Format::PlainText,
        )) {
            Err(domain::Error::InputTooLarge {
                limit,
                actual,
                what,
            }) => {
                assert_eq!(limit, MAX_INPUT_BYTES);
                assert_eq!(actual, MAX_INPUT_BYTES + 1);
                assert_eq!(what, "input");
            }
            other => panic!("expected InputTooLarge, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn annotate_leaves_metrics_none_and_compose_fills_them() {
        let engine = law_engine();
        let mut doc = engine
            .annotate(Input::text(
                "Annotate attaches structure. Compose measures it.",
                Format::PlainText,
            ))
            .unwrap();
        assert_eq!(
            doc.metrics,
            domain::DocumentMetrics::default(),
            "annotate does not measure"
        );
        assert_eq!(doc.sentences().count(), 2, "annotate does attach");
        engine.compose(&mut doc);
        assert_eq!(doc.metrics.total_sentences, 2, "compose counts");
        assert_eq!(doc.metrics.paragraph_count, 1, "compose counts");
        // DotSplitNlp builds token-less sentences, so token-derived
        // document metrics stay None here; what compose guarantees is
        // totality, which is the absence of a failure path in its
        // signature. passive_ratio is sentence-derived, so it does
        // fill: no sentence here has a passive construction.
        assert_eq!(
            doc.metrics.passive_ratio,
            Some(0.0),
            "compose fills the sentence-derived aggregate"
        );
    }

    #[test]
    fn annotate_fills_hearst_pairs_at_the_choke_point() {
        /// Returns one sentence carrying the verified "Animals such as
        /// dogs" arc shape for any input, so the test can observe the
        /// pipeline running the detector without a real model.
        struct SuchAsNlp;
        impl NlpProvider for SuchAsNlp {
            fn parse(&self, _text: &str) -> domain::Result<Vec<domain::Sentence>> {
                let tok = |id: usize, text: &str, lemma: &str, pos: &str, dep: &str, head| {
                    domain::Token::builder(
                        id,
                        text.to_string(),
                        lemma.to_string(),
                        pos.to_string(),
                        head,
                        dep.to_string(),
                    )
                    .build()
                };
                Ok(vec![domain::Sentence::new(
                    "Animals such as dogs bark".to_string(),
                    vec![
                        tok(1, "Animals", "animal", "NOUN", "nsubj", 5),
                        tok(2, "such", "such", "ADJ", "case", 4),
                        tok(3, "as", "as", "ADP", "fixed", 2),
                        tok(4, "dogs", "dog", "NOUN", "nmod", 1),
                        tok(5, "bark", "bark", "VERB", "root", 0),
                    ],
                )])
            }

            fn identity(&self) -> domain::Identity {
                domain::Identity::new("such-as", None, None, None)
            }
        }

        let engine = Engine::with_parser(Box::new(SuchAsNlp));
        let doc = engine
            .annotate(Input::text("any text", Format::PlainText))
            .unwrap();
        let sentence = doc.sentences().next().expect("one sentence");
        assert_eq!(sentence.hearst_pairs.len(), 1, "annotate ran the detector");
        let pair = &sentence.hearst_pairs[0];
        assert_eq!(pair.pattern, domain::HearstPattern::SuchAs);
        assert_eq!(pair.hypernym.head_id, 1);
        assert_eq!(pair.hyponym.head_id, 4);
    }

    #[test]
    fn parse_per_paragraph_scopes_sentences_to_originating_paragraph() {
        // FM1 regression: two paragraphs with the same first 30 chars.
        // Pre-fix, attach_sentences would prefix-match and could assign
        // either paragraph's first sentence to the wrong paragraph
        // (and silently drop the other). Per-paragraph parse makes the
        // assignment unambiguous by construction.
        let text = "The system processes input now. Tail one.\n\n\
                    The system processes input now. Tail two.";
        let analysis = law_engine()
            .analyze(Input::text(text, Format::PlainText))
            .unwrap();

        let paras: Vec<_> = analysis.paragraphs().collect();
        assert_eq!(paras.len(), 2);
        assert_eq!(paras[0].sentences.len(), 2);
        assert_eq!(paras[1].sentences.len(), 2);

        // Each paragraph keeps its own tail; no leak.
        assert!(paras[0].sentences.iter().any(|s| s.text.contains("one")));
        assert!(paras[1].sentences.iter().any(|s| s.text.contains("two")));
        assert!(!paras[0].sentences.iter().any(|s| s.text.contains("two")));
        assert!(!paras[1].sentences.iter().any(|s| s.text.contains("one")));
    }

    #[test]
    fn parse_per_paragraph_no_inner_substring_theft() {
        // Inner-substring regression: paragraph A contains paragraph B's
        // first-sentence prefix as a mid-text substring. Pre-fix, the
        // greedy prefix-contains check could steal B's sentence into A.
        // Per-paragraph parse makes the question moot.
        let text = "Outer talks about the special phrase processes input now. End A.\n\n\
                    The special phrase processes input now. End B.";
        let analysis = law_engine()
            .analyze(Input::text(text, Format::PlainText))
            .unwrap();

        let paras: Vec<_> = analysis.paragraphs().collect();
        assert_eq!(paras.len(), 2);

        // Paragraph A should have its own two sentences; paragraph B its own two.
        // Critically: B's "End B" must be in B, not stolen by A.
        assert!(paras[1].sentences.iter().any(|s| s.text.contains("End B")));
        assert!(!paras[0].sentences.iter().any(|s| s.text.contains("End B")));
    }

    #[test]
    fn empty_paragraph_followed_by_valid_paragraph() {
        // Trailing whitespace on an empty paragraph used to confuse the
        // prefix-match wiring; per-paragraph parse handles cleanly.
        // The plain text reader never emits an empty paragraph, so this
        // registers a custom reader that injects one, which also
        // exercises a hand-built reader table end to end.
        struct InjectEmptyParagraph;
        impl Decomposer for InjectEmptyParagraph {
            fn decompose(&self, text: &str) -> domain::Result<decompose::Read> {
                let mut read = decompose::plain::PlainTextDecomposer.decompose(text)?;
                read.blocks.insert(
                    0,
                    decompose::ReadBlock::Paragraph {
                        kind: ParagraphKind::Paragraph,
                        text: String::new(),
                        span: Span::default(),
                        map: decompose::TextMap::at(0),
                    },
                );
                Ok(read)
            }
        }

        let engine = Engine {
            nlp: Box::new(DotSplitNlp),
            readers: decompose::Decomposers::new()
                .with(Format::PlainText, Box::new(InjectEmptyParagraph)),
        };
        let analysis = engine
            .analyze(Input::text(
                "Real content sentence.\n\nAnother real one.",
                Format::PlainText,
            ))
            .unwrap();

        let paras: Vec<_> = analysis.paragraphs().collect();
        assert_eq!(paras.len(), 3);
        assert_eq!(
            paras[0].sentences.len(),
            0,
            "empty paragraph has zero sentences"
        );
        assert!(!paras[1].sentences.is_empty());
        assert!(!paras[2].sentences.is_empty());
        let _: &[Section] = &analysis.sections;
    }

    // ------------------------------------------------------------------
    // clusters: the composition root's pairing of the two halves
    // ------------------------------------------------------------------

    /// Mock embedder: a fixed vector per call, optionally lying about
    /// the count to violate the length contract.
    struct FixedEmbedder {
        identity: &'static str,
        /// When set, return this many vectors regardless of input.
        force_count: Option<usize>,
    }
    impl embed::Embedder for FixedEmbedder {
        fn embed(&self, texts: &[&str]) -> domain::Result<Vec<domain::Embedding>> {
            let n = self.force_count.unwrap_or(texts.len());
            Ok((0..n).map(|_| domain::Embedding(vec![1.0, 0.0])).collect())
        }
        fn identity(&self) -> domain::Identity {
            domain::Identity::new(self.identity, None, None, None)
        }
    }

    /// An embedder that panics if reached: a gate must fire first.
    struct MustNotEmbed;
    impl embed::Embedder for MustNotEmbed {
        fn embed(&self, _texts: &[&str]) -> domain::Result<Vec<domain::Embedding>> {
            panic!("embed ran when a gate should have refused first");
        }
        fn identity(&self) -> domain::Identity {
            domain::Identity::new("unreached", None, None, None)
        }
    }

    fn two_sentence_doc() -> Document {
        let engine = Engine::with_parser(Box::new(DotSplitNlp));
        engine
            .annotate(Input::text(
                "One sentence. Another sentence.",
                Format::PlainText,
            ))
            .expect("annotate")
    }

    /// The result carries the document's provenance, plus the method,
    /// the threshold and the embedder's own identity.
    #[test]
    fn clusters_records_the_embedder_and_the_threshold() {
        let doc = two_sentence_doc();
        let embedder = FixedEmbedder {
            identity: "the-real-model",
            force_count: None,
        };
        let out = clusters(&doc, &embedder, 0.9).expect("cluster");
        let p = &out.provenance;
        assert_eq!(p.method.as_deref(), Some(CLUSTERS_METHOD));
        let parameters = p.parameters.as_ref().expect("parameters");
        assert_eq!(
            parameters.embedder,
            Some(domain::Identity::new("the-real-model", None, None, None))
        );
        assert_eq!(parameters.threshold, Some(0.9));
        assert_eq!(parameters.n, None);
        assert_eq!(p.source, doc.provenance.source, "the document's input");
        assert_eq!(p.parser, doc.provenance.parser, "the document's parser");
        // Identical vectors cluster together, sanity of the plumbing.
        assert_eq!(out.clusters.len(), 1);
        assert_eq!(out.clusters[0].members, [0, 1]);
        assert_eq!(out.edges.len(), 1);
    }

    #[test]
    fn clusters_rejects_a_length_contract_violation() {
        let doc = two_sentence_doc();
        let embedder = FixedEmbedder {
            identity: "liar",
            force_count: Some(1),
        };
        match clusters(&doc, &embedder, 0.9) {
            Err(domain::Error::InvalidInput { message: msg, .. }) => {
                // Regression: the count read "1 vectors".
                assert_eq!(
                    msg,
                    "embedder returned 1 vector for 2 sentences, violating its contract"
                );
            }
            other => panic!("expected InvalidInput, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn clusters_caps_before_embedding() {
        let many = "Word. ".repeat(domain::limits::MAX_CLUSTER_SENTENCES + 1);
        let engine = Engine::with_parser(Box::new(DotSplitNlp));
        let doc = engine
            .annotate(Input::text(many, Format::PlainText))
            .expect("annotate");
        match clusters(&doc, &MustNotEmbed, 0.9) {
            Err(domain::Error::InputTooLarge { what, .. }) => {
                assert_eq!(what, "clusters");
            }
            other => panic!("expected InputTooLarge, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn clusters_refuses_a_non_finite_threshold_before_embedding() {
        let doc = two_sentence_doc();
        for threshold in [f64::NAN, f64::INFINITY] {
            assert!(matches!(
                clusters(&doc, &MustNotEmbed, threshold),
                Err(domain::Error::InvalidInput { .. })
            ));
        }
    }

    // ------------------------------------------------------------------
    // The outcome functions' provenance and defaults
    // ------------------------------------------------------------------

    /// A summary and keyphrases copy the document's provenance and add
    /// the method that ran and the `n` they were asked for.
    #[test]
    fn outcomes_record_the_method_and_n() {
        let doc = Engine::with_parser(Box::new(WordNlp))
            .analyze(Input::text(
                "Rivers carry silt.\n\nSilt builds deltas.",
                Format::PlainText,
            ))
            .unwrap();
        let summary = summarize(&doc, 1).unwrap();
        assert_eq!(summary.provenance.method.as_deref(), Some("tfidf"));
        let summary_with = summarize_with(&doc, 1, SummaryMethod::TextRank).unwrap();
        assert_eq!(summary_with.provenance.method.as_deref(), Some("textrank"));
        let phrases = keyphrases(&doc, 4).unwrap();
        assert_eq!(phrases.provenance.method.as_deref(), Some("rake"));
        let phrases_with = keyphrases_with(&doc, 4, KeyphraseMethod::Yake).unwrap();
        assert_eq!(phrases_with.provenance.method.as_deref(), Some("yake"));
        for (provenance, n) in [
            (&summary.provenance, 1),
            (&summary_with.provenance, 1),
            (&phrases.provenance, 4),
            (&phrases_with.provenance, 4),
        ] {
            let parameters = provenance.parameters.as_ref().unwrap();
            assert_eq!(parameters.n, Some(n));
            assert_eq!(parameters.threshold, None);
            assert_eq!(parameters.embedder, None);
            assert_eq!(provenance.source, doc.provenance.source);
            assert_eq!(provenance.schema, doc.provenance.schema);
        }
        assert!(doc.provenance.method.is_none(), "a document has no method");
        let json = serde_json::to_value(&summary).unwrap();
        assert_eq!(
            json["provenance"]["parameters"],
            serde_json::json!({ "n": 1 }),
            "only the parameters the method takes"
        );
        let json = serde_json::to_value(&doc).unwrap();
        assert!(json["provenance"].get("method").is_none());
        assert!(json["provenance"].get("parameters").is_none());
    }

    /// A summary sentence points back at the document's sentence: its
    /// index among `Document::sentences`, its span and its text.
    #[test]
    fn a_summary_sentence_points_into_the_document() {
        let text = "Rivers carry silt downstream.\n\nThe silt settles where rivers slow.";
        let doc = Engine::with_parser(Box::new(WordNlp))
            .analyze(Input::text(text, Format::PlainText))
            .unwrap();
        let summary = summarize(&doc, 2).unwrap();
        assert_eq!(summary.sentences.len(), 2);
        for picked in &summary.sentences {
            let sentence = doc.sentences().nth(picked.index).unwrap();
            assert_eq!(picked.span, sentence.span);
            assert_eq!(picked.text, sentence.text);
            assert_eq!(
                &text[picked.span.byte_start..picked.span.byte_end],
                picked.text
            );
        }
    }

    /// The library's default methods are the ones `config/default.toml`
    /// ships. The library reads no configuration, so the default is
    /// written twice, and this is what holds the two together.
    #[test]
    fn the_default_methods_are_the_shipped_configuration_defaults() {
        let cfg = config::Config::from_sources(
            |key| (key == "HOME").then(|| "/nonexistent/matra-test-home".to_string()),
            None,
        )
        .unwrap();
        assert_eq!(
            extraction::DEFAULT_SUMMARY_METHOD.name(),
            cfg.summary_method()
        );
        assert_eq!(
            extraction::DEFAULT_KEYPHRASE_METHOD.name(),
            cfg.keyphrases_method()
        );
    }

    // ------------------------------------------------------------------
    // The schema lane with no model: what the pipeline emits matches the
    // published schema. The conformance lanes check the real parser's
    // output against the same schema; this keeps the check in every
    // `cargo test`, through a stand-in parser that emits every kind of
    // record field.
    // ------------------------------------------------------------------

    use crate::document_schema;

    /// One sentence for any input: "Smith never said that animals such as
    /// dogs bark." with the arcs a live parse gives it, so every derived
    /// field on `Sentence` is populated.
    struct RecordParser;

    impl NlpProvider for RecordParser {
        fn parse(&self, _text: &str) -> domain::Result<Vec<domain::Sentence>> {
            let tok = |id: usize, text: &str, pos: &str, dep: &str, head: usize, feats: &str| {
                domain::Token::builder(
                    id,
                    text.to_string(),
                    text.to_lowercase(),
                    pos.to_string(),
                    head,
                    dep.to_string(),
                )
                .feats(feats.to_string())
                .is_punct(pos == "PUNCT")
                .build()
            };
            Ok(vec![domain::Sentence::new(
                "Smith never said that animals such as dogs may bark.".to_string(),
                vec![
                    tok(1, "Smith", "PROPN", "nsubj", 3, "Number=Sing"),
                    tok(2, "never", "ADV", "advmod", 3, "_"),
                    tok(
                        3,
                        "said",
                        "VERB",
                        "root",
                        0,
                        "Mood=Ind|Tense=Past|VerbForm=Fin",
                    ),
                    tok(4, "that", "SCONJ", "mark", 10, "_"),
                    tok(5, "animals", "NOUN", "nsubj", 10, "Number=Plur"),
                    tok(6, "such", "ADJ", "case", 8, "Degree=Pos"),
                    tok(7, "as", "ADP", "fixed", 6, "_"),
                    tok(8, "dogs", "NOUN", "nmod", 5, "Number=Plur"),
                    tok(9, "may", "AUX", "aux", 10, "VerbForm=Fin"),
                    tok(10, "bark", "VERB", "ccomp", 3, "VerbForm=Inf"),
                    tok(11, ".", "PUNCT", "punct", 3, "_"),
                ],
            )])
        }

        fn identity(&self) -> domain::Identity {
            domain::Identity::new(
                "record-parser",
                Some("fixture".to_string()),
                Some("0".repeat(64)),
                Some("MIT".to_string()),
            )
        }
    }

    fn record_document(text: &str, format: Format) -> Document {
        Engine::with_parser(Box::new(RecordParser))
            .analyze(Input::text(text, format))
            .expect("analyze")
    }

    fn record_json(text: &str, format: Format) -> serde_json::Value {
        serde_json::to_value(record_document(text, format)).expect("a document serializes")
    }

    #[test]
    fn a_plain_text_document_matches_the_schema() {
        let doc = record_json("One paragraph of prose.", Format::PlainText);
        // The stand-in emits every record field, so the check below is
        // over populated arrays and objects rather than empty ones.
        let sentence = &doc["sections"][0]["paragraphs"][0]["sentences"][0];
        for field in [
            "negations",
            "modals",
            "clausal_complements",
            "root_adverbials",
            "hearst_pairs",
        ] {
            assert!(
                !sentence[field].as_array().expect(field).is_empty(),
                "the stand-in parse populates {field}"
            );
        }
        assert!(sentence["root_mood"].is_object());
        document_schema::assert_valid("document", &doc, "plain text");
    }

    #[test]
    fn a_markdown_document_with_a_blockquote_matches_the_schema() {
        let doc = record_json(
            "# Title\n\nA paragraph.\n\n> A quotation, never parsed.\n\n## Second\n\nMore prose.\n",
            Format::Markdown,
        );
        assert_eq!(doc["provenance"]["format"], "markdown");
        document_schema::assert_valid("document", &doc, "markdown");
    }

    /// Lists, skipped constructs and their spans are in the schema too.
    #[test]
    fn a_markdown_document_with_skipped_constructs_matches_the_schema() {
        let doc = record_json(
            "---\ntitle: x\n---\n\n# Title\n\n- an item\n- another\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n```\ncode\n```\n",
            Format::Markdown,
        );
        let kinds: Vec<&str> = doc["skipped"]
            .as_array()
            .expect("skipped")
            .iter()
            .map(|s| s["kind"].as_str().expect("kind"))
            .collect();
        assert_eq!(kinds, ["front_matter", "table", "code"]);
        assert_eq!(
            doc["sections"][0]["paragraphs"][0]["kind"], "list_item",
            "a list item is a paragraph of its own kind"
        );
        document_schema::assert_valid("document", &doc, "markdown with skipped constructs");
    }

    /// Block input, as JSON, validates against its own published schema,
    /// reads into a document that validates against the document schema,
    /// and a block the schema does not allow is refused by the reader too.
    #[test]
    fn block_input_matches_its_schema_and_reads_into_a_document() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("spec/schema/v2/blocks.schema.json");
        let schema: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("read")).expect("parse");
        let validator = jsonschema::draft202012::new(&schema).expect("a valid JSON Schema");

        let good = serde_json::json!([
            {"kind": "heading", "level": 1, "text": "Field notes"},
            {"kind": "paragraph", "text": "The river rose overnight."}
        ]);
        assert!(validator.is_valid(&good));
        let doc = record_json(&good.to_string(), Format::Blocks);
        assert_eq!(doc["provenance"]["format"], "blocks");
        assert_eq!(doc["sections"][0]["heading"], "Field notes");
        document_schema::assert_valid("document", &doc, "block input");

        let engine = Engine::with_parser(Box::new(RecordParser));
        for bad in [
            serde_json::json!([{"kind": "table", "text": "x"}]),
            serde_json::json!([{"kind": "heading", "level": 0, "text": "x"}]),
            serde_json::json!([{"kind": "paragraph", "text": "x", "style": "bold"}]),
        ] {
            assert!(!validator.is_valid(&bad), "the schema refuses {bad}");
            let err = engine
                .analyze(Input::text(bad.to_string(), Format::Blocks))
                .expect_err("the reader refuses it too");
            assert_eq!(err.kind(), "invalid_input", "{bad}: {err}");
        }
    }

    /// A document with no sentences is where the ratios are null, and
    /// null has to be something the schema allows.
    #[test]
    fn an_empty_document_matches_the_schema() {
        let doc = record_json("", Format::PlainText);
        assert!(doc["metrics"]["mean_sentence_length"].is_null());
        document_schema::assert_valid("document", &doc, "empty");
    }

    /// The closed check is what catches an undocumented field: a document
    /// carrying one more key than the schema lists fails.
    #[test]
    #[should_panic(expected = "does not match schema 2")]
    fn an_undocumented_field_fails_the_check() {
        let mut doc = record_json("Prose.", Format::PlainText);
        doc["metrics"]["undocumented"] = serde_json::json!(1);
        document_schema::assert_valid("document", &doc, "undocumented field");
    }

    /// Every outcome of a pipeline-built document matches its published
    /// schema, with the derived provenance the document's schema allows.
    #[test]
    fn every_outcome_of_a_parsed_document_matches_its_schema() {
        let doc = record_document("First paragraph.\n\nSecond paragraph.", Format::PlainText);
        for method in [SummaryMethod::TfIdf, SummaryMethod::TextRank] {
            let value = serde_json::to_value(summarize_with(&doc, 1, method).unwrap()).unwrap();
            document_schema::assert_valid("summary", &value, method.name());
        }
        for method in [KeyphraseMethod::Rake, KeyphraseMethod::Yake] {
            let value = serde_json::to_value(keyphrases_with(&doc, 5, method).unwrap()).unwrap();
            assert!(
                !value["phrases"].as_array().unwrap().is_empty(),
                "{}: the check runs over populated phrases",
                method.name()
            );
            document_schema::assert_valid("keyphrases", &value, method.name());
        }
        let embedder = FixedEmbedder {
            identity: "fixed",
            force_count: None,
        };
        let value = serde_json::to_value(clusters(&doc, &embedder, 0.5).unwrap()).unwrap();
        assert!(!value["edges"].as_array().unwrap().is_empty());
        document_schema::assert_valid("clusters", &value, "clusters");
    }

    /// The I/O failures matra raises itself carry a path or a URL as
    /// details, which only the crate can build; each matches the error
    /// schema, closed, so a detail it does not list for `io` fails here.
    /// `tests/corpus_conformance.rs` checks every variant's plain form.
    #[test]
    fn every_io_detail_matches_the_error_schema() {
        let cases = [
            domain::Error::Io(domain::IoContext::at_path(
                std::io::ErrorKind::Unsupported,
                Path::new("link.md"),
                "refusing to read symlink: link.md",
            )),
            domain::Error::Io(domain::IoContext::download(
                std::io::ErrorKind::TimedOut,
                "https://models.example/m",
                "download https://models.example/m: timeout",
                true,
            )),
            match Inputs::dir(Path::new("/nonexistent/matra/dir")) {
                Err(err) => err,
                Ok(_) => panic!("a missing directory lists nothing"),
            },
        ];
        for error in &cases {
            let value = serde_json::to_value(error).unwrap();
            assert_eq!(value["kind"], "io");
            document_schema::assert_valid("error", &value, &error.to_string());
        }
    }

    /// A detail the schema does not list for its kind fails the closed
    /// check, the error schema's counterpart of the document's.
    #[test]
    #[should_panic(expected = "does not match schema 2")]
    fn an_undocumented_detail_fails_the_check() {
        let mut value = serde_json::to_value(domain::Error::InputTooLarge {
            limit: 1,
            actual: 2,
            what: "input",
        })
        .unwrap();
        value["details"]["undocumented"] = serde_json::json!(1);
        document_schema::assert_valid("error", &value, "undocumented detail");
    }

    // ------------------------------------------------------------------
    // The corpus lane's directory walk, with no model
    // ------------------------------------------------------------------

    /// `spec/tests/corpus/items.json`'s directory: one readable file and
    /// one whose bytes are not UTF-8 give two items, in path order, the
    /// second an error item of the kind the fixture names. The failure is
    /// reported rather than raised, and it does not end the walk. The
    /// parser returns no sentences, because what is pinned is the shape
    /// of the item and the order of the walk, not the parse; the
    /// fixture's other expectations run outside the crate, in
    /// `tests/corpus_conformance.rs`.
    #[test]
    fn a_directory_walk_yields_one_item_per_file() {
        #[derive(serde::Deserialize)]
        struct Fixture {
            directory: Directory,
            item_shapes: ItemShapes,
        }
        #[derive(serde::Deserialize)]
        struct ItemShapes {
            error: Vec<String>,
        }
        #[derive(serde::Deserialize)]
        struct Directory {
            files: Vec<FixtureFile>,
            expect: Vec<Expect>,
        }
        #[derive(serde::Deserialize)]
        struct FixtureFile {
            name: String,
            text: Option<String>,
            bytes: Option<Vec<u8>>,
        }
        #[derive(serde::Deserialize)]
        struct Expect {
            name: String,
            shape: String,
            kind: Option<String>,
            details: Option<serde_json::Value>,
        }

        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("spec/tests/corpus/items.json");
        let fixture: Fixture =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        for file in &fixture.directory.files {
            let path = dir.path().join(&file.name);
            match (&file.text, &file.bytes) {
                (Some(text), None) => std::fs::write(&path, text).unwrap(),
                (None, Some(bytes)) => std::fs::write(&path, bytes).unwrap(),
                _ => panic!(
                    "fixture file {} needs exactly one of text, bytes",
                    file.name
                ),
            }
        }

        let items: Vec<_> = Engine::with_parser(Box::new(EmptyNlp))
            .analyze_all(Inputs::dir(dir.path()).unwrap())
            .collect();

        assert_eq!(items.len(), fixture.directory.expect.len());
        for (item, expect) in items.iter().zip(&fixture.directory.expect) {
            match (item, expect.shape.as_str()) {
                (Ok(document), "document") => {
                    let path = document.provenance.source.path.as_ref().unwrap();
                    assert_eq!(path.file_name().unwrap(), expect.name.as_str());
                    document_schema::assert_valid(
                        "document",
                        &serde_json::to_value(document).unwrap(),
                        &expect.name,
                    );
                }
                (Err(err), "error") => {
                    let path = err.path.as_ref().unwrap();
                    assert_eq!(path.file_name().unwrap(), expect.name.as_str());
                    assert_eq!(err.error.kind(), expect.kind.as_deref().unwrap());
                    assert!(!err.error.to_string().is_empty());
                    let value = serde_json::to_value(err).unwrap();
                    let mut keys: Vec<&String> = value.as_object().unwrap().keys().collect();
                    keys.sort();
                    let mut shape = fixture.item_shapes.error.iter().collect::<Vec<_>>();
                    shape.sort();
                    assert_eq!(keys, shape, "{}", expect.name);
                    assert_eq!(Some(&value["details"]), expect.details.as_ref());
                    document_schema::assert_valid("error", &value, &expect.name);
                }
                (got, shape) => panic!(
                    "{} expected shape {shape}, got {}",
                    expect.name,
                    if got.is_ok() { "document" } else { "error" }
                ),
            }
        }
    }
}
