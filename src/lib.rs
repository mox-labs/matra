#![doc = include_str!("../README.md")]

pub mod decompose;
pub mod domain;
pub mod extraction;
pub mod metrics;
pub mod nlp;
pub mod source;
mod stopwords;

use std::path::Path;

use decompose::Decomposer;
use domain::{Analysis, Section};
use nlp::NlpProvider;
use source::Source;

/// Analyze raw text. Returns structured metrics.
pub fn analyze(text: &str, nlp: &dyn NlpProvider) -> domain::Result<Analysis> {
    let sections = decompose::plain::PlainTextDecomposer.decompose(text);
    run_analysis(sections, text, nlp)
}

/// Analyze markdown text. Returns structured metrics with section awareness.
pub fn analyze_markdown(text: &str, nlp: &dyn NlpProvider) -> domain::Result<Analysis> {
    let sections = decompose::markdown::MarkdownDecomposer.decompose(text);
    let prose: String = sections
        .iter()
        .flat_map(|s| s.paragraphs.iter())
        .filter(|p| !p.in_blockquote)
        .map(|p| p.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    run_analysis(sections, &prose, nlp)
}

/// Analyze a file, detecting format by extension. Returns
/// [`domain::Error::UnsupportedFormat`] for `Pdf`/`Docx` until a
/// decomposer is registered for those formats.
pub fn analyze_file(path: impl AsRef<Path>, nlp: &dyn NlpProvider) -> domain::Result<Analysis> {
    let docs = source::file::FileSource.read(path.as_ref())?;
    let doc = docs.into_iter().next().ok_or_else(|| {
        domain::Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "source returned no documents",
        ))
    })?;
    analyze_raw(&doc.text, doc.format, nlp)
}

fn analyze_raw(
    text: &str,
    format: domain::Format,
    nlp: &dyn NlpProvider,
) -> domain::Result<Analysis> {
    match format {
        domain::Format::Markdown => analyze_markdown(text, nlp),
        domain::Format::PlainText => analyze(text, nlp),
        other @ (domain::Format::Pdf | domain::Format::Docx) => {
            Err(domain::Error::UnsupportedFormat(other))
        }
    }
}

/// Analyze all readable files in a directory. Returns a `Corpus` of
/// successfully analyzed documents and a list of per-document errors
/// (from analysis or unsupported formats).
///
/// Note: at the `Source` layer, `DirectorySource` still aborts on the
/// first filesystem read error. Per-file I/O tolerance is tracked for
/// 0.2. See [`source::directory::DirectorySource`].
pub fn analyze_directory(
    path: impl AsRef<Path>,
    nlp: &dyn NlpProvider,
) -> domain::Result<(domain::Corpus, Vec<(std::path::PathBuf, domain::Error)>)> {
    let docs = source::directory::DirectorySource.read(path.as_ref())?;
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    for doc in docs {
        let path = doc.path.clone();
        match analyze_raw(&doc.text, doc.format, nlp) {
            Ok(analysis) => entries.push(domain::CorpusEntry { path, analysis }),
            Err(e) => errors.push((path.unwrap_or_default(), e)),
        }
    }
    Ok((domain::Corpus::new(entries), errors))
}

/// Parse text into NLP annotations. Call once, pass to multiple consumers.
///
/// This enables the parse-once-use-many pattern:
/// ```no_run
/// # use vaani::nlp::NlpProvider;
/// # fn example(text: &str, nlp: &dyn NlpProvider) -> vaani::domain::Result<()> {
/// let sentences = vaani::parse(text, nlp)?;
/// let summary = vaani::extraction::tfidf_summarize(&sentences, 3);
/// let phrases = vaani::extraction::rake_keyphrases(&sentences, 10);
/// # Ok(())
/// # }
/// ```
pub fn parse(text: &str, nlp: &dyn NlpProvider) -> domain::Result<Vec<domain::Sentence>> {
    nlp.parse(text)
}

/// Analyze from pre-decomposed sections and pre-parsed sentences.
/// Use with [`parse()`] for the no-double-parse pattern.
///
/// ```no_run
/// # use vaani::decompose::Decomposer;
/// # use vaani::nlp::NlpProvider;
/// # fn example(text: &str, nlp: &dyn NlpProvider) -> vaani::domain::Result<()> {
/// let sections = vaani::decompose::markdown::MarkdownDecomposer.decompose(text);
/// let sentences = vaani::parse(text, nlp)?;
/// let analysis = vaani::analyze_from(sections, &sentences);
/// let summary = vaani::extraction::tfidf_summarize(&sentences, 3);
/// # Ok(())
/// # }
/// ```
pub fn analyze_from(sections: Vec<Section>, sentences: &[domain::Sentence]) -> Analysis {
    let mut analysis = Analysis::new(sections);
    let suite = metrics::default_suite();
    metrics::run_suite(&mut analysis, sentences, &suite);
    analysis
}

/// Shared analysis pipeline: parse NLP, run the default metric suite.
fn run_analysis(
    sections: Vec<Section>,
    prose: &str,
    nlp: &dyn NlpProvider,
) -> domain::Result<Analysis> {
    let mut analysis = Analysis::new(sections);
    let sentences = nlp.parse(prose)?;
    let suite = metrics::default_suite();
    metrics::run_suite(&mut analysis, &sentences, &suite);
    Ok(analysis)
}

// ---------------------------------------------------------------------------
// PyO3 bindings (behind "python" feature flag)
// ---------------------------------------------------------------------------

#[cfg(feature = "python")]
mod python {
    use pyo3::prelude::*;
    use pyo3::types::PyAny;

    use crate::nlp::NlpProvider;

    fn to_dict<'py, T: serde::Serialize>(py: Python<'py>, val: &T) -> PyResult<Bound<'py, PyAny>> {
        pythonize::pythonize(py, val)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Holds a loaded NLP model. Create once, reuse across calls.
    ///
    /// Marked unsendable because NLP models may contain C state that is
    /// not thread-safe. Python's GIL provides the necessary synchronization.
    #[pyclass(unsendable)]
    struct Vaani {
        nlp: Box<dyn NlpProvider>,
    }

    #[pymethods]
    impl Vaani {
        #[staticmethod]
        #[cfg(feature = "udpipe")]
        fn from_path(model_path: &str) -> PyResult<Self> {
            let nlp = crate::nlp::udpipe::Udpipe::from_path(model_path)
                .map_err(|e| pyo3::exceptions::PyFileNotFoundError::new_err(e.to_string()))?;
            Ok(Self { nlp: Box::new(nlp) })
        }

        #[staticmethod]
        #[cfg(feature = "udpipe")]
        fn english(model_dir: &str) -> PyResult<Self> {
            let nlp = crate::nlp::udpipe::Udpipe::english(model_dir)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            Ok(Self { nlp: Box::new(nlp) })
        }

        /// Analyze plain text. Returns a Python dict.
        fn analyze<'py>(&self, py: Python<'py>, text: &str) -> PyResult<Bound<'py, PyAny>> {
            let analysis = crate::analyze(text, self.nlp.as_ref())
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            to_dict(py, &analysis)
        }

        /// Analyze markdown. Returns a Python dict.
        fn analyze_markdown<'py>(
            &self,
            py: Python<'py>,
            text: &str,
        ) -> PyResult<Bound<'py, PyAny>> {
            let analysis = crate::analyze_markdown(text, self.nlp.as_ref())
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            to_dict(py, &analysis)
        }

        /// TF-IDF extractive summary. Returns a list of dicts.
        fn tfidf_summarize<'py>(
            &self,
            py: Python<'py>,
            text: &str,
            n: usize,
        ) -> PyResult<Bound<'py, PyAny>> {
            let sentences = self
                .nlp
                .parse(text)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            let result = crate::extraction::tfidf_summarize(&sentences, n);
            to_dict(py, &result)
        }

        /// TextRank extractive summary. Returns a list of dicts.
        fn textrank_summarize<'py>(
            &self,
            py: Python<'py>,
            text: &str,
            n: usize,
        ) -> PyResult<Bound<'py, PyAny>> {
            let sentences = self
                .nlp
                .parse(text)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            let result = crate::extraction::textrank_summarize(&sentences, n)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            to_dict(py, &result)
        }

        /// RAKE keyphrase extraction. Returns a list of dicts.
        fn rake_keyphrases<'py>(
            &self,
            py: Python<'py>,
            text: &str,
            max_phrases: usize,
        ) -> PyResult<Bound<'py, PyAny>> {
            let sentences = self
                .nlp
                .parse(text)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            let result = crate::extraction::rake_keyphrases(&sentences, max_phrases);
            to_dict(py, &result)
        }

        /// YAKE keyphrase extraction. Returns a list of dicts.
        fn yake_keyphrases<'py>(
            &self,
            py: Python<'py>,
            text: &str,
            max_phrases: usize,
        ) -> PyResult<Bound<'py, PyAny>> {
            let sentences = self
                .nlp
                .parse(text)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            let result = crate::extraction::yake_keyphrases(&sentences, max_phrases);
            to_dict(py, &result)
        }
    }

    #[pymodule]
    pub fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_class::<Vaani>()?;
        Ok(())
    }
}
