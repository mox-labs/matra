//! Resolved configuration: where things are, and what the defaults are.
//!
//! [`Config`] is a composition-root value, in the same layer as
//! [`Engine`](crate::Engine). It resolves *locations and defaults* and
//! never behavior: everything it carries is something a caller could
//! pass as an argument instead (ADR-0011). Which metrics run and how
//! output is shaped stay where they were, in the caller and in the
//! binary.
//!
//! Resolution order, per key: an explicit argument, then the `MATRA_*`
//! environment, then the config file, then the defaults compiled into
//! the crate from `config/default.toml`. The argument rung is
//! [`Config::with_model_dir`], which is the only thing that produces
//! it. Every resolved value records which rung it came from, readable
//! through [`Config::sources`].
//!
//! Three environment variables name the thing they override:
//!
//! | Variable | Overrides |
//! |---|---|
//! | `MATRA_CONFIG_FILE` | the config file path |
//! | `MATRA_DATA_DIR` | the data root |
//! | `MATRA_MODEL_DIR` | the model directory |
//!
//! Paths follow the XDG conventions on Linux and macOS: the config file
//! is `$XDG_CONFIG_HOME/matra/config.toml`, defaulting to
//! `~/.config/matra/config.toml`; the data root is
//! `$XDG_DATA_HOME/matra`, defaulting to `~/.local/share/matra`, with
//! models under `models/`. matra never creates `~/.matra`; when an
//! existing, non-empty legacy cache is selected it is used as the model
//! directory, downloads and re-downloads included. Create the new
//! location, or set `MATRA_MODEL_DIR`, to move off it.

use std::path::{Component, Path, PathBuf};

use crate::domain::{self, Error};

/// The defaults compiled into the crate. This is the last rung of the
/// resolution order, and the file a user config overrides key by key.
const DEFAULT_TOML: &str = include_str!("../config/default.toml");

/// Cap on the config file, checked against the file metadata before any
/// read. A config file is small by construction; anything larger is a
/// mistake or an attack, and either way it should not be read into
/// memory first and rejected second.
const MAX_CONFIG_BYTES: u64 = 64 * 1024;

/// Summarization algorithms this build knows. An unknown name in the
/// config file is rejected at resolve time, not at call time.
const SUMMARIZE_ALGORITHMS: [&str; 2] = ["tfidf", "textrank"];

/// Keyphrase algorithms this build knows.
const KEYPHRASE_ALGORITHMS: [&str; 2] = ["rake", "yake"];

/// The rung of the resolution order a value came from.
///
/// Paired with a key by [`Config::sources`], this is what lets a caller
/// print where every effective value originated instead of asserting a
/// number with no provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValueSource {
    /// Passed explicitly by the caller through
    /// [`Config::with_model_dir`], outranking every other rung.
    Argument,
    /// Read from the named environment variable.
    Environment(String),
    /// Read from the config file at this path.
    File(PathBuf),
    /// The built-in default, compiled in from `config/default.toml`.
    Default,
}

/// Resolved locations and defaults.
///
/// Build one with [`Config::resolve`] (the real environment and the
/// real config file) or with [`Config::from_sources`] (both injected,
/// which is how the tests avoid reading the developer's home).
///
/// ```no_run
/// let cfg = matra::config::Config::resolve()?;
/// println!("models live in {}", cfg.model_dir().display());
/// # Ok::<(), matra::domain::Error>(())
/// ```
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Config {
    data_dir: PathBuf,
    model_dir: PathBuf,
    udpipe_model: String,
    embedding_model: String,
    semantic_threshold: f32,
    summarize_n: usize,
    summarize_algorithm: String,
    keyphrases_n: usize,
    keyphrases_algorithm: String,
    sources: Vec<(&'static str, ValueSource)>,
    /// The config file this configuration's own environment names, or
    /// `None` when that environment names none. Carried rather than
    /// recomputed, so a caller asking about this configuration is never
    /// answered from the process environment instead.
    config_file: Option<PathBuf>,
}

impl Config {
    /// Resolve from the process environment and the user's config file.
    ///
    /// A missing config file is not an error: the built-in defaults
    /// stand. A malformed one is [`Error::InvalidInput`] naming the file
    /// and the offending key or line.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidInput`] for a malformed config file, an unknown
    /// algorithm name, or an environment with none of `MATRA_DATA_DIR`,
    /// `XDG_DATA_HOME` or `HOME` set. [`Error::InputTooLarge`] for a
    /// config file over 64 KiB. [`Error::Io`] for a read that fails for
    /// any reason other than the file not existing.
    pub fn resolve() -> domain::Result<Config> {
        let env = |key: &str| std::env::var(key).ok();
        let contents = match config_file_path_from(&env) {
            Some(path) => read_config_file(&path)?,
            None => None,
        };
        Config::from_sources(env, contents.as_deref())
    }

    /// Resolve from an injected environment and an injected config file.
    ///
    /// `env` answers environment lookups by name; `file` is the config
    /// file's *contents*, or `None` when there is no file. The path the
    /// contents are attributed to is derived from `env`, so a test that
    /// injects `HOME` gets file provenance without touching a disk.
    ///
    /// This is the form every test uses. [`Config::resolve`] is the same
    /// function with the process environment and a real read wired in.
    ///
    /// # Errors
    ///
    /// The same set as [`Config::resolve`], minus the read failures.
    pub fn from_sources(
        env: impl Fn(&str) -> Option<String>,
        file: Option<&str>,
    ) -> domain::Result<Config> {
        let env: &dyn Fn(&str) -> Option<String> = &env;

        let config_file = config_file_path_from(env);
        // The attribution path for messages about the file's contents.
        // A configuration with no file still needs a name to hang a
        // parse error on, and nothing reads this one.
        let file_path = config_file
            .clone()
            .unwrap_or_else(|| PathBuf::from("config.toml"));
        let from_file = match file {
            Some(text) => parse_toml(text, &file_path)?,
            None => FileConfig::default(),
        };
        let from_default = parse_toml(DEFAULT_TOML, Path::new("config/default.toml"))?;

        let mut sources: Vec<(&'static str, ValueSource)> = Vec::new();

        let (data_dir, data_dir_source) = resolve_data_dir(env)?;
        sources.push(("data_dir", data_dir_source));
        let (model_dir, model_dir_source) = resolve_model_dir(env, &data_dir);
        sources.push(("model_dir", model_dir_source));

        let models_file = from_file.models.unwrap_or_default();
        let models_default = from_default.models.unwrap_or_default();
        let semantic_file = from_file.semantic.unwrap_or_default();
        let semantic_default = from_default.semantic.unwrap_or_default();
        let summarize_file = from_file.summarize.unwrap_or_default();
        let summarize_default = from_default.summarize.unwrap_or_default();
        let keyphrases_file = from_file.keyphrases.unwrap_or_default();
        let keyphrases_default = from_default.keyphrases.unwrap_or_default();

        let udpipe_model = choose(
            "models.udpipe",
            models_file.udpipe,
            models_default.udpipe,
            &file_path,
            &mut sources,
        )?;
        let embedding_model = choose(
            "models.embedding",
            models_file.embedding,
            models_default.embedding,
            &file_path,
            &mut sources,
        )?;
        let semantic_threshold = choose(
            "semantic.threshold",
            semantic_file.threshold,
            semantic_default.threshold,
            &file_path,
            &mut sources,
        )?;
        let summarize_n = choose(
            "summarize.n",
            summarize_file.n,
            summarize_default.n,
            &file_path,
            &mut sources,
        )?;
        let summarize_algorithm = choose(
            "summarize.algorithm",
            summarize_file.algorithm,
            summarize_default.algorithm,
            &file_path,
            &mut sources,
        )?;
        let keyphrases_n = choose(
            "keyphrases.n",
            keyphrases_file.n,
            keyphrases_default.n,
            &file_path,
            &mut sources,
        )?;
        let keyphrases_algorithm = choose(
            "keyphrases.algorithm",
            keyphrases_file.algorithm,
            keyphrases_default.algorithm,
            &file_path,
            &mut sources,
        )?;

        // Validation is at resolve time so a typo in a config file
        // surfaces once, at construction, rather than at whichever call
        // first reaches for the value.
        if !semantic_threshold.is_finite() {
            return Err(Error::InvalidInput(format!(
                "{}: semantic.threshold must be finite, got {semantic_threshold}",
                origin_of("semantic.threshold", &sources, &file_path),
            )));
        }
        check_algorithm(
            "summarize.algorithm",
            &summarize_algorithm,
            &SUMMARIZE_ALGORITHMS,
            &sources,
            &file_path,
        )?;
        check_algorithm(
            "keyphrases.algorithm",
            &keyphrases_algorithm,
            &KEYPHRASE_ALGORITHMS,
            &sources,
            &file_path,
        )?;
        check_path_component("models.embedding", &embedding_model, &sources, &file_path)?;

        Ok(Config {
            data_dir,
            model_dir,
            udpipe_model,
            embedding_model,
            semantic_threshold,
            summarize_n,
            summarize_algorithm,
            keyphrases_n,
            keyphrases_algorithm,
            sources,
            config_file,
        })
    }

    /// The same configuration with the model directory taken from an
    /// explicit argument.
    ///
    /// [`ValueSource::Argument`] is the top rung of the resolution order
    /// (ADR-0011: "A directory passed explicitly wins over all three"),
    /// so a caller that was handed a directory can layer it on without
    /// losing the provenance of every other value. A command line's
    /// `--model-dir` is the case this exists for: the flag reaches the
    /// adapter through `Config` rather than past it into the NLP port.
    ///
    /// Only `model_dir` changes rung; every other key keeps the source
    /// it resolved from.
    ///
    /// ```
    /// # let cfg = matra::config::Config::from_sources(
    /// #     |k| (k == "HOME").then(|| "/tmp/matra-doctest".to_string()), None)?;
    /// let cfg = cfg.with_model_dir("/opt/models");
    /// assert_eq!(cfg.model_dir(), std::path::Path::new("/opt/models"));
    /// # Ok::<(), matra::domain::Error>(())
    /// ```
    #[must_use]
    pub fn with_model_dir(mut self, dir: impl AsRef<Path>) -> Config {
        self.model_dir = dir.as_ref().to_path_buf();
        for (key, source) in &mut self.sources {
            if *key == "model_dir" {
                *source = ValueSource::Argument;
            }
        }
        self
    }

    /// The config file this process would read: `MATRA_CONFIG_FILE`,
    /// else `$XDG_CONFIG_HOME/matra/config.toml`, else
    /// `~/.config/matra/config.toml`.
    ///
    /// `None` when none of `MATRA_CONFIG_FILE`, `XDG_CONFIG_HOME` and
    /// `HOME` is set, which is the one environment where matra cannot
    /// name a config file at all. The path is not read here and need not
    /// exist.
    pub fn config_file_path() -> Option<PathBuf> {
        config_file_path_from(&|key: &str| std::env::var(key).ok())
    }

    /// The config file *this* configuration's environment named, or
    /// `None` when it named none. The path is not read here and need not
    /// exist.
    ///
    /// The instance counterpart to [`Config::config_file_path`], and the
    /// one to reach for whenever a `Config` is in hand. The static reads
    /// the process environment, so a caller holding a configuration
    /// built through [`Config::from_sources`] with an injected
    /// environment would otherwise be answered about a different
    /// machine's files.
    pub fn config_file(&self) -> Option<&Path> {
        self.config_file.as_deref()
    }

    /// The data root: `MATRA_DATA_DIR`, else `$XDG_DATA_HOME/matra`,
    /// else `~/.local/share/matra`.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// The model directory: `MATRA_MODEL_DIR`, else `data_dir/models`,
    /// except that an existing, non-empty `~/.matra/models` wins when
    /// `data_dir/models` does not exist.
    ///
    /// matra never creates `~/.matra`. When an existing, non-empty
    /// legacy cache is selected it is used as the model directory,
    /// downloads and re-downloads included: `Udpipe::english` writes
    /// into whichever directory it is handed. Create the new location,
    /// or set `MATRA_MODEL_DIR`, to move off it.
    ///
    /// An empty `~/.matra/models` is not selected. There is no cache in
    /// it to keep working, and picking it would capture every later
    /// download into a directory matra would otherwise never touch.
    ///
    /// Resolved once, at construction: the directory this returns does
    /// not change under the process's feet when the filesystem does.
    pub fn model_dir(&self) -> &Path {
        &self.model_dir
    }

    /// The UDPipe model name from `[models] udpipe`.
    pub fn udpipe_model(&self) -> &str {
        &self.udpipe_model
    }

    /// The embedding model name from `[models] embedding`.
    pub fn embedding_model(&self) -> &str {
        &self.embedding_model
    }

    /// The default cosine similarity threshold for semantic clustering.
    pub fn semantic_threshold(&self) -> f32 {
        self.semantic_threshold
    }

    /// The default number of sentences a summary keeps.
    pub fn summarize_n(&self) -> usize {
        self.summarize_n
    }

    /// The default summarization algorithm: `"tfidf"` or `"textrank"`.
    pub fn summarize_algorithm(&self) -> &str {
        &self.summarize_algorithm
    }

    /// The default number of keyphrases extracted.
    pub fn keyphrases_n(&self) -> usize {
        self.keyphrases_n
    }

    /// The default keyphrase algorithm: `"rake"` or `"yake"`.
    pub fn keyphrases_algorithm(&self) -> &str {
        &self.keyphrases_algorithm
    }

    /// Every resolved key paired with the rung it came from, in a stable
    /// order. This is what a `config show` prints.
    ///
    /// ```no_run
    /// let cfg = matra::config::Config::resolve()?;
    /// for (key, source) in cfg.sources() {
    ///     println!("{key}: {source:?}");
    /// }
    /// # Ok::<(), matra::domain::Error>(())
    /// ```
    pub fn sources(&self) -> impl Iterator<Item = (&'static str, ValueSource)> {
        self.sources
            .iter()
            .map(|(key, source)| (*key, source.clone()))
    }
}

// ---------------------------------------------------------------------------
// The config file's shape
// ---------------------------------------------------------------------------

// Unknown keys are rejected rather than ignored. A file matra half-reads
// is a file whose author believes a setting is in force when it is not,
// and the silence lasts until someone measures. Every field is optional,
// so a partial file overrides key by key.

#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FileConfig {
    models: Option<ModelsSection>,
    semantic: Option<SemanticSection>,
    summarize: Option<CountAndAlgorithm>,
    keyphrases: Option<CountAndAlgorithm>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelsSection {
    udpipe: Option<String>,
    embedding: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SemanticSection {
    threshold: Option<f32>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CountAndAlgorithm {
    n: Option<usize>,
    algorithm: Option<String>,
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

/// Parse one config document, naming `path` and the offending line when
/// it does not parse. toml's own error carries the line, the column and
/// the key; wrapping it would lose that, so it travels intact.
fn parse_toml(text: &str, path: &Path) -> domain::Result<FileConfig> {
    toml::from_str(text).map_err(|e| {
        Error::InvalidInput(format!("{}: {}", path.display(), e.to_string().trim_end()))
    })
}

/// The file rung wins over the default rung; a key absent from both is a
/// broken `config/default.toml`, which is a bug in this crate rather
/// than in the caller's file, and says so.
fn choose<T>(
    key: &'static str,
    from_file: Option<T>,
    from_default: Option<T>,
    file_path: &Path,
    sources: &mut Vec<(&'static str, ValueSource)>,
) -> domain::Result<T> {
    if let Some(value) = from_file {
        sources.push((key, ValueSource::File(file_path.to_path_buf())));
        return Ok(value);
    }
    if let Some(value) = from_default {
        sources.push((key, ValueSource::Default));
        return Ok(value);
    }
    Err(Error::InvalidInput(format!(
        "built-in defaults (config/default.toml) are missing `{key}`"
    )))
}

/// The origin to name in a validation message: the config file when the
/// value came from it, the built-in defaults otherwise.
fn origin_of(key: &str, sources: &[(&'static str, ValueSource)], file_path: &Path) -> String {
    match sources.iter().find(|(k, _)| *k == key) {
        Some((_, ValueSource::File(path))) => path.display().to_string(),
        _ => format!("built-in defaults ({})", file_path.display()),
    }
}

fn check_algorithm(
    key: &'static str,
    value: &str,
    known: &[&str],
    sources: &[(&'static str, ValueSource)],
    file_path: &Path,
) -> domain::Result<()> {
    if known.contains(&value) {
        return Ok(());
    }
    Err(Error::InvalidInput(format!(
        "{}: {key} is `{value}`, which is not one of {}",
        origin_of(key, sources, file_path),
        known.join(", "),
    )))
}

/// Reject a configured name that is not a single ordinary path
/// component.
///
/// `models.embedding` is joined onto the model directory, and the
/// directory that results is where the embedding provisioner writes its
/// temporaries and sweeps aged ones. A free string reaching a
/// delete-by-pattern sweep is the reason this check exists: `"../.."`
/// resolves to a directory the operator never named, and while the sweep
/// only unlinks `.tmp.` entries older than ten minutes, a delete
/// operation whose directory comes from configuration should be told
/// where it may not go rather than trusted not to wander.
///
/// A single component is the whole rule: no separator, no `..`, no `.`,
/// no root, and not empty. A backslash is refused as well, so the same
/// string is accepted or refused whatever the platform, rather than
/// being one component on Unix and two on Windows.
///
/// `models.udpipe` gets no such check because nothing joins it to a
/// path: the UDPipe artifact's filename is pinned in the adapter beside
/// its digest, and this value is only ever printed. Validating a value
/// nothing resolves would refuse configurations that harm nobody.
fn check_path_component(
    key: &'static str,
    value: &str,
    sources: &[(&'static str, ValueSource)],
    file_path: &Path,
) -> domain::Result<()> {
    let mut components = Path::new(value).components();
    let single =
        matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none();
    if single && !value.contains('\\') {
        return Ok(());
    }
    Err(Error::InvalidInput(format!(
        "{}: {key} is `{value}`, which is not a single path component. It names a directory \
         inside the model directory, so it cannot be empty, absolute, or contain `/`, `\\`, \
         `.` or `..`",
        origin_of(key, sources, file_path),
    )))
}

fn config_file_path_from(env: &dyn Fn(&str) -> Option<String>) -> Option<PathBuf> {
    if let Some(path) = non_empty(env("MATRA_CONFIG_FILE")) {
        return Some(PathBuf::from(path));
    }
    if let Some(dir) = non_empty(env("XDG_CONFIG_HOME")) {
        return Some(PathBuf::from(dir).join("matra").join("config.toml"));
    }
    let home = non_empty(env("HOME"))?;
    Some(
        PathBuf::from(home)
            .join(".config")
            .join("matra")
            .join("config.toml"),
    )
}

fn resolve_data_dir(
    env: &dyn Fn(&str) -> Option<String>,
) -> domain::Result<(PathBuf, ValueSource)> {
    if let Some(dir) = non_empty(env("MATRA_DATA_DIR")) {
        return Ok((
            PathBuf::from(dir),
            ValueSource::Environment("MATRA_DATA_DIR".to_string()),
        ));
    }
    if let Some(dir) = non_empty(env("XDG_DATA_HOME")) {
        return Ok((
            PathBuf::from(dir).join("matra"),
            ValueSource::Environment("XDG_DATA_HOME".to_string()),
        ));
    }
    if let Some(home) = non_empty(env("HOME")) {
        return Ok((
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("matra"),
            ValueSource::Default,
        ));
    }
    Err(Error::InvalidInput(
        "cannot locate the data directory: set MATRA_DATA_DIR, XDG_DATA_HOME, or HOME".to_string(),
    ))
}

fn resolve_model_dir(
    env: &dyn Fn(&str) -> Option<String>,
    data_dir: &Path,
) -> (PathBuf, ValueSource) {
    if let Some(dir) = non_empty(env("MATRA_MODEL_DIR")) {
        return (
            PathBuf::from(dir),
            ValueSource::Environment("MATRA_MODEL_DIR".to_string()),
        );
    }
    let current = data_dir.join("models");
    if !current.exists()
        && let Some(home) = non_empty(env("HOME"))
    {
        let legacy = PathBuf::from(home).join(".matra").join("models");
        if holds_something(&legacy) {
            return (legacy, ValueSource::Default);
        }
    }
    (current, ValueSource::Default)
}

/// Whether `dir` is a directory with at least one entry in it.
///
/// The legacy fallback exists to keep an existing cache working, and an
/// empty directory is not a cache. Selecting one anyway would hand the
/// adapter a directory to download into, permanently, in a location
/// matra would otherwise never create. Existence alone is therefore not
/// the test: something has to be in there.
///
/// Deliberately blind to what the entries are called. Which files a
/// model cache holds is the adapter's knowledge, not this module's, and
/// a filename list here would go stale the first time an adapter
/// changed one.
fn holds_something(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_some())
}

/// An environment variable set to the empty string carries no path, and
/// treating it as one produces a relative path rooted at the working
/// directory. Absent and empty mean the same thing here.
fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.is_empty())
}

/// Read the config file if it is there. Absent is `Ok(None)`; oversized,
/// unreadable and non-UTF-8 are all loud.
fn read_config_file(path: &Path) -> domain::Result<Option<String>> {
    // A config file under the user's own home is routinely a symlink into
    // a dotfiles repository, so unlike `source/file.rs` (which reads paths
    // chosen by whoever calls matra) this path does not reject symlinks.
    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(Error::Io(e)),
    };
    if metadata.len() > MAX_CONFIG_BYTES {
        return Err(Error::InputTooLarge {
            limit: MAX_CONFIG_BYTES as usize,
            actual: metadata.len() as usize,
            what: "config_file",
        });
    }
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::InvalidData => Err(Error::InvalidInput(format!(
            "{}: not valid UTF-8",
            path.display()
        ))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::Io(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// An injected environment. Every test builds one of these, so no
    /// test reads the developer's real environment or home directory.
    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        move |key: &str| map.get(key).cloned()
    }

    fn source_of(cfg: &Config, key: &str) -> ValueSource {
        cfg.sources()
            .find(|(k, _)| *k == key)
            .map(|(_, s)| s)
            .unwrap_or_else(|| panic!("no source recorded for {key}"))
    }

    // -- the default rung ---------------------------------------------

    #[test]
    fn embedded_defaults_parse_and_carry_the_documented_values() {
        let cfg = Config::from_sources(env_of(&[("HOME", "/home/tester")]), None).unwrap();
        assert_eq!(cfg.udpipe_model(), "english-ewt-ud-2.5-191206");
        assert_eq!(cfg.embedding_model(), "potion-base-8M");
        assert_eq!(cfg.semantic_threshold(), 0.85);
        assert_eq!(cfg.summarize_n(), 3);
        assert_eq!(cfg.summarize_algorithm(), "tfidf");
        assert_eq!(cfg.keyphrases_n(), 10);
        assert_eq!(cfg.keyphrases_algorithm(), "rake");
    }

    #[test]
    fn every_key_reports_the_default_rung_when_nothing_overrides() {
        let cfg = Config::from_sources(env_of(&[("HOME", "/home/tester")]), None).unwrap();
        let keys: Vec<&'static str> = cfg.sources().map(|(k, _)| k).collect();
        assert_eq!(
            keys,
            vec![
                "data_dir",
                "model_dir",
                "models.udpipe",
                "models.embedding",
                "semantic.threshold",
                "summarize.n",
                "summarize.algorithm",
                "keyphrases.n",
                "keyphrases.algorithm",
            ]
        );
        for (key, source) in cfg.sources() {
            assert_eq!(source, ValueSource::Default, "{key} did not default");
        }
    }

    #[test]
    fn default_paths_follow_xdg_when_only_home_is_set() {
        let cfg = Config::from_sources(env_of(&[("HOME", "/home/tester")]), None).unwrap();
        assert_eq!(cfg.data_dir(), Path::new("/home/tester/.local/share/matra"));
        assert_eq!(
            cfg.model_dir(),
            Path::new("/home/tester/.local/share/matra/models")
        );
    }

    // -- the file rung ------------------------------------------------

    #[test]
    fn file_beats_the_built_in_default() {
        let file = r#"
            [semantic]
            threshold = 0.5

            [summarize]
            algorithm = "textrank"
        "#;
        let cfg = Config::from_sources(env_of(&[("HOME", "/home/tester")]), Some(file)).unwrap();
        assert_eq!(cfg.semantic_threshold(), 0.5);
        assert_eq!(cfg.summarize_algorithm(), "textrank");
        // Untouched keys still come from the built-in defaults.
        assert_eq!(cfg.summarize_n(), 3);
        assert_eq!(cfg.keyphrases_algorithm(), "rake");
    }

    #[test]
    fn sources_name_the_file_for_keys_the_file_set() {
        let file = "[semantic]\nthreshold = 0.5\n";
        let cfg = Config::from_sources(env_of(&[("HOME", "/home/tester")]), Some(file)).unwrap();
        assert_eq!(
            source_of(&cfg, "semantic.threshold"),
            ValueSource::File(PathBuf::from("/home/tester/.config/matra/config.toml")),
        );
        assert_eq!(source_of(&cfg, "summarize.n"), ValueSource::Default);
    }

    // -- the environment rung -----------------------------------------

    #[test]
    fn matra_data_dir_beats_xdg_and_home() {
        let cfg = Config::from_sources(
            env_of(&[
                ("MATRA_DATA_DIR", "/srv/matra"),
                ("XDG_DATA_HOME", "/home/tester/.local/share"),
                ("HOME", "/home/tester"),
            ]),
            None,
        )
        .unwrap();
        assert_eq!(cfg.data_dir(), Path::new("/srv/matra"));
        assert_eq!(
            source_of(&cfg, "data_dir"),
            ValueSource::Environment("MATRA_DATA_DIR".to_string())
        );
    }

    #[test]
    fn xdg_data_home_beats_home() {
        let cfg = Config::from_sources(
            env_of(&[
                ("XDG_DATA_HOME", "/elsewhere/share"),
                ("HOME", "/home/tester"),
            ]),
            None,
        )
        .unwrap();
        assert_eq!(cfg.data_dir(), Path::new("/elsewhere/share/matra"));
        assert_eq!(
            source_of(&cfg, "data_dir"),
            ValueSource::Environment("XDG_DATA_HOME".to_string())
        );
    }

    #[test]
    fn matra_model_dir_beats_the_derived_model_directory() {
        let cfg = Config::from_sources(
            env_of(&[("MATRA_MODEL_DIR", "/models"), ("HOME", "/home/tester")]),
            None,
        )
        .unwrap();
        assert_eq!(cfg.model_dir(), Path::new("/models"));
        assert_eq!(
            source_of(&cfg, "model_dir"),
            ValueSource::Environment("MATRA_MODEL_DIR".to_string())
        );
        // The data root is unaffected by the model-directory override.
        assert_eq!(cfg.data_dir(), Path::new("/home/tester/.local/share/matra"));
    }

    #[test]
    fn an_environment_variable_set_to_empty_is_treated_as_unset() {
        let cfg = Config::from_sources(
            env_of(&[("MATRA_DATA_DIR", ""), ("HOME", "/home/tester")]),
            None,
        )
        .unwrap();
        assert_eq!(cfg.data_dir(), Path::new("/home/tester/.local/share/matra"));
    }

    #[test]
    fn an_environment_with_no_home_at_all_is_an_error() {
        let err = Config::from_sources(env_of(&[]), None).unwrap_err();
        assert!(
            matches!(err, Error::InvalidInput(ref m) if m.contains("MATRA_DATA_DIR")),
            "unexpected error: {err}"
        );
    }

    // -- the ~/.matra/models fallback ---------------------------------

    /// A legacy cache with something in it, which is the only shape the
    /// fallback exists for.
    fn legacy_cache_in(home: &Path) -> PathBuf {
        let legacy = home.join(".matra").join("models");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("english-ewt.udpipe"), b"not a real model").unwrap();
        legacy
    }

    #[test]
    fn model_dir_falls_back_to_a_non_empty_dot_matra_when_only_that_exists() {
        let home = tempfile::tempdir().unwrap();
        let legacy = legacy_cache_in(home.path());

        let cfg =
            Config::from_sources(env_of(&[("HOME", home.path().to_str().unwrap())]), None).unwrap();
        assert_eq!(cfg.model_dir(), legacy);
    }

    /// An empty `~/.matra/models` is a leftover, not a cache. Selecting
    /// it would send every future download into a directory matra
    /// would otherwise never create, and it would keep doing so.
    #[test]
    fn model_dir_ignores_an_empty_dot_matra_directory() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join(".matra").join("models")).unwrap();

        let cfg =
            Config::from_sources(env_of(&[("HOME", home.path().to_str().unwrap())]), None).unwrap();
        assert_eq!(
            cfg.model_dir(),
            home.path()
                .join(".local")
                .join("share")
                .join("matra")
                .join("models")
        );
    }

    #[test]
    fn model_dir_prefers_the_data_directory_when_both_exist() {
        let home = tempfile::tempdir().unwrap();
        legacy_cache_in(home.path());
        let current = home
            .path()
            .join(".local")
            .join("share")
            .join("matra")
            .join("models");
        std::fs::create_dir_all(&current).unwrap();

        let cfg =
            Config::from_sources(env_of(&[("HOME", home.path().to_str().unwrap())]), None).unwrap();
        assert_eq!(cfg.model_dir(), current);
    }

    #[test]
    fn model_dir_ignores_dot_matra_when_the_environment_names_a_directory() {
        let home = tempfile::tempdir().unwrap();
        legacy_cache_in(home.path());

        let cfg = Config::from_sources(
            env_of(&[
                ("HOME", home.path().to_str().unwrap()),
                ("MATRA_MODEL_DIR", "/models"),
            ]),
            None,
        )
        .unwrap();
        assert_eq!(cfg.model_dir(), Path::new("/models"));
    }

    // -- the argument rung --------------------------------------------

    #[test]
    fn with_model_dir_puts_the_model_directory_on_the_argument_rung() {
        let cfg = Config::from_sources(env_of(&[("HOME", "/home/tester")]), None)
            .unwrap()
            .with_model_dir("/opt/models");
        assert_eq!(cfg.model_dir(), Path::new("/opt/models"));
        assert_eq!(source_of(&cfg, "model_dir"), ValueSource::Argument);
    }

    #[test]
    fn with_model_dir_beats_the_environment_rung() {
        let cfg = Config::from_sources(
            env_of(&[("MATRA_MODEL_DIR", "/models"), ("HOME", "/home/tester")]),
            None,
        )
        .unwrap()
        .with_model_dir("/opt/models");
        assert_eq!(cfg.model_dir(), Path::new("/opt/models"));
        assert_eq!(source_of(&cfg, "model_dir"), ValueSource::Argument);
    }

    #[test]
    fn with_model_dir_leaves_every_other_key_and_its_source_alone() {
        let file = "[semantic]\nthreshold = 0.5\n";
        let before = Config::from_sources(env_of(&[("HOME", "/home/tester")]), Some(file)).unwrap();
        let before_sources: Vec<(&'static str, ValueSource)> = before.sources().collect();
        let after = before.clone().with_model_dir("/opt/models");

        // The data root and every value are untouched.
        assert_eq!(
            after.data_dir(),
            Path::new("/home/tester/.local/share/matra")
        );
        assert_eq!(after.semantic_threshold(), 0.5);
        assert_eq!(after.summarize_algorithm(), "tfidf");

        // Only the model_dir key changed rung.
        let after_sources: Vec<(&'static str, ValueSource)> = after.sources().collect();
        assert_eq!(after_sources.len(), before_sources.len());
        for ((key, before), (after_key, after)) in before_sources.iter().zip(&after_sources) {
            assert_eq!(key, after_key, "key order changed");
            if *key == "model_dir" {
                assert_eq!(*after, ValueSource::Argument);
            } else {
                assert_eq!(before, after, "{key} changed rung");
            }
        }
    }

    // -- the config file path -----------------------------------------

    #[test]
    fn config_file_path_prefers_matra_config_file() {
        let path = config_file_path_from(&env_of(&[
            ("MATRA_CONFIG_FILE", "/etc/matra.toml"),
            ("XDG_CONFIG_HOME", "/home/tester/.config"),
            ("HOME", "/home/tester"),
        ]));
        assert_eq!(path, Some(PathBuf::from("/etc/matra.toml")));
    }

    #[test]
    fn config_file_path_then_xdg_config_home_then_home() {
        let xdg = config_file_path_from(&env_of(&[
            ("XDG_CONFIG_HOME", "/elsewhere/config"),
            ("HOME", "/home/tester"),
        ]));
        assert_eq!(
            xdg,
            Some(PathBuf::from("/elsewhere/config/matra/config.toml"))
        );

        let home = config_file_path_from(&env_of(&[("HOME", "/home/tester")]));
        assert_eq!(
            home,
            Some(PathBuf::from("/home/tester/.config/matra/config.toml"))
        );

        assert_eq!(config_file_path_from(&env_of(&[])), None);
    }

    // -- malformed input ----------------------------------------------

    #[test]
    fn malformed_toml_is_invalid_input_naming_the_file_and_the_line() {
        let err = Config::from_sources(
            env_of(&[("HOME", "/home/tester")]),
            Some("[semantic\nthreshold = 0.5\n"),
        )
        .unwrap_err();
        let Error::InvalidInput(message) = err else {
            panic!("expected InvalidInput, got {err:?}");
        };
        assert!(
            message.contains("/home/tester/.config/matra/config.toml"),
            "message does not name the file: {message}"
        );
        assert!(
            message.contains("line 1"),
            "message does not name the line: {message}"
        );
    }

    #[test]
    fn a_value_of_the_wrong_type_is_invalid_input_naming_the_key() {
        let err = Config::from_sources(
            env_of(&[("HOME", "/home/tester")]),
            Some("[semantic]\nthreshold = \"high\"\n"),
        )
        .unwrap_err();
        let Error::InvalidInput(message) = err else {
            panic!("expected InvalidInput, got {err:?}");
        };
        assert!(
            message.contains("/home/tester/.config/matra/config.toml")
                && message.contains("line 2"),
            "message does not locate the fault: {message}"
        );
    }

    #[test]
    fn an_unknown_key_is_rejected_rather_than_ignored() {
        let err = Config::from_sources(
            env_of(&[("HOME", "/home/tester")]),
            Some("[semantic]\ntreshold = 0.5\n"),
        )
        .unwrap_err();
        let Error::InvalidInput(message) = err else {
            panic!("expected InvalidInput, got {err:?}");
        };
        assert!(
            message.contains("treshold"),
            "message does not name the unknown key: {message}"
        );
    }

    #[test]
    fn an_unknown_summarize_algorithm_is_rejected_at_resolve_time() {
        let err = Config::from_sources(
            env_of(&[("HOME", "/home/tester")]),
            Some("[summarize]\nalgorithm = \"lexrank\"\n"),
        )
        .unwrap_err();
        let Error::InvalidInput(message) = err else {
            panic!("expected InvalidInput, got {err:?}");
        };
        assert!(
            message.contains("summarize.algorithm")
                && message.contains("lexrank")
                && message.contains("textrank")
                && message.contains("/home/tester/.config/matra/config.toml"),
            "message is not actionable: {message}"
        );
    }

    #[test]
    fn an_unknown_keyphrase_algorithm_is_rejected_at_resolve_time() {
        let err = Config::from_sources(
            env_of(&[("HOME", "/home/tester")]),
            Some("[keyphrases]\nalgorithm = \"textrank\"\n"),
        )
        .unwrap_err();
        let Error::InvalidInput(message) = err else {
            panic!("expected InvalidInput, got {err:?}");
        };
        assert!(
            message.contains("keyphrases.algorithm") && message.contains("rake, yake"),
            "message is not actionable: {message}"
        );
    }

    /// Regression (review of #77, L1): `models.embedding` is joined onto
    /// the model directory and the result is where the embedding
    /// provisioner sweeps aged temporaries, so a value that escapes the
    /// model directory points a delete-by-pattern operation somewhere the
    /// operator did not name. The check runs at resolve time, so it fires
    /// once rather than at whichever call first reaches for the value.
    #[test]
    fn an_embedding_name_that_is_not_a_path_component_is_rejected() {
        for value in ["../..", "..", ".", "", "/etc", "a/b", "a\\b"] {
            let err = Config::from_sources(
                env_of(&[("HOME", "/home/tester")]),
                Some(&format!("[models]\nembedding = '{value}'\n")),
            )
            .unwrap_err();
            let Error::InvalidInput(message) = err else {
                panic!("expected InvalidInput for {value:?}, got {err:?}");
            };
            assert!(
                message.contains("models.embedding")
                    && message.contains("/home/tester/.config/matra/config.toml"),
                "message names neither the key nor the file for {value:?}: {message}"
            );
        }
    }

    /// The rejection is narrow. An ordinary directory name still
    /// resolves, including one carrying the dots and dashes a model
    /// revision name uses.
    #[test]
    fn an_ordinary_embedding_name_resolves() {
        for value in ["potion-base-8M", "potion.base.8M", "my_model", "..leading"] {
            let cfg = Config::from_sources(
                env_of(&[("HOME", "/home/tester")]),
                Some(&format!("[models]\nembedding = '{value}'\n")),
            )
            .unwrap_or_else(|e| panic!("{value:?} should resolve, got {e:?}"));
            assert_eq!(cfg.embedding_model(), value);
        }
    }

    #[test]
    fn a_non_finite_threshold_is_rejected() {
        let err = Config::from_sources(
            env_of(&[("HOME", "/home/tester")]),
            Some("[semantic]\nthreshold = nan\n"),
        )
        .unwrap_err();
        assert!(
            matches!(err, Error::InvalidInput(ref m) if m.contains("semantic.threshold")),
            "unexpected error: {err}"
        );
    }

    // -- reading the file ---------------------------------------------

    #[test]
    fn a_missing_config_file_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nothing").join("config.toml");
        assert_eq!(read_config_file(&missing).unwrap(), None);
    }

    #[test]
    fn an_oversized_config_file_is_rejected_before_it_is_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, vec![b'#'; (MAX_CONFIG_BYTES + 1) as usize]).unwrap();
        let err = read_config_file(&path).unwrap_err();
        assert!(
            matches!(
                err,
                Error::InputTooLarge {
                    what: "config_file",
                    ..
                }
            ),
            "unexpected error: {err}"
        );
    }
}
