//! model2vec adapter: static sentence embeddings behind the [`Embedder`] port.
//!
//! Loads the published model2vec artifact format (an embedding matrix in
//! safetensors, a tokenizer.json, a config.json) and embeds by table
//! gather, mean pooling, and optional L2 normalization. No matmul, no
//! kernel dispatch, so vectors are bit-identical across targets, which is
//! the property that won this adapter the first slot (ADR-0010).
//!
//! This is the ONLY file that imports `safetensors` and `tokenizers`
//! (boundary rule 4 analog). The inference semantics replicate the Python
//! reference implementation, pinned by the parity and bit-identity
//! fixtures in this file's tests, and the pinned reference model's own
//! digest is pinned again in `spec/tests/semantic/reference-model.json`
//! for every crust's conformance runner.

use std::fs;
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

use safetensors::SafeTensors;
use safetensors::tensor::Dtype;
use sha2::{Digest, Sha256};
use tokenizers::Tokenizer;

#[cfg(not(target_arch = "wasm32"))]
use crate::config::Config;
use crate::domain::{self, Embedding, Error};
use crate::embed::Embedder;

/// The reference implementation's default token cap per text. Applied
/// after unknown-token removal; also drives the pre-tokenization
/// character truncation (`max_tokens * median_token_length` bytes).
const DEFAULT_MAX_TOKENS: usize = 512;

/// The three artifact filenames, in the order the identity digest covers
/// them. Every path in this file is built from this array, so the digest
/// and the loader can never disagree about which file is which.
const ARTIFACT_FILES: [&str; 3] = ["model.safetensors", "tokenizer.json", "config.json"];

/// SHA-256 over the three artifact files of the pinned potion-base-8M
/// release, concatenated in [`ARTIFACT_FILES`] order. The same value
/// `spec/tests/semantic/reference-model.json` carries as `artifact_hash`,
/// and the same value [`Model2Vec::model_hash`] reports once loaded.
///
/// Refresh this and [`POTION_BASE_8M_URLS`] together with
/// `scripts/fetch-embedding-hash.sh <revision>`.
#[cfg(not(target_arch = "wasm32"))]
const POTION_BASE_8M_SHA256: &str =
    "81c3592150873b1c5a8c4262850f795bff4fd568fbde80ac69889d087f16a0b4";

/// Download URLs for the pinned release, one per [`ARTIFACT_FILES`]
/// entry in the same order. The path segment after `resolve/` is a git
/// commit on the Hugging Face repository rather than a branch, so the
/// URL names one immutable tree; `main` would move under the pin. The
/// digest is what actually decides whether the bytes are trusted, and
/// the revision is what makes a mismatch a bug report rather than an
/// upstream edit.
///
/// Refresh with `scripts/fetch-embedding-hash.sh <revision>`.
#[cfg(not(target_arch = "wasm32"))]
const POTION_BASE_8M_URLS: [&str; 3] = [
    "https://huggingface.co/minishlab/potion-base-8M/resolve/bf8b056651a2c21b8d2565580b8569da283cab23/model.safetensors",
    "https://huggingface.co/minishlab/potion-base-8M/resolve/bf8b056651a2c21b8d2565580b8569da283cab23/tokenizer.json",
    "https://huggingface.co/minishlab/potion-base-8M/resolve/bf8b056651a2c21b8d2565580b8569da283cab23/config.json",
];

/// Ceiling on any one downloaded artifact. The largest of the three in
/// the pinned release is `model.safetensors` at 30,236,760 bytes, so this
/// is a shade over twice the real thing: headroom for a later revision,
/// and small enough that a redirect to something enormous costs a bounded
/// read rather than the machine's memory. Past it,
/// [`Error::InputTooLarge`] with `what` set to `"embedding_download"`.
#[cfg(not(target_arch = "wasm32"))]
const MAX_ARTIFACT_BYTES: usize = 64 * 1024 * 1024;

/// Ceiling on one artifact fetch, DNS lookup through the last byte of
/// the body. The largest artifact in the pinned release is about 30 MB,
/// which needs the whole of this budget only on a link under 1 Mbit/s,
/// so the margin is generous for a slow connection and still finite: a
/// stalled or black-holed transfer fails instead of holding the caller
/// for as long as the socket stays open.
#[cfg(not(target_arch = "wasm32"))]
const FETCH_TIMEOUT: Duration = Duration::from_secs(300);

/// Ceiling on establishing the connection, socket and TLS handshake
/// included. Separate from [`FETCH_TIMEOUT`] so an unreachable host
/// fails in seconds rather than consuming the whole transfer budget.
#[cfg(not(target_arch = "wasm32"))]
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// A loaded static embedding model in the model2vec artifact format.
///
/// Construct with [`Model2Vec::from_dir`]. Each artifact's bytes are
/// read once, hashed, and parsed from that same buffer (the read-then-
/// consume discipline `read_and_verify` established for UDPipe; nothing
/// re-reads disk between hash and load). The digest over all three
/// files is the model identity that provenance-carrying result types
/// report.
pub struct Model2Vec {
    /// Row-major `[vocab, dim]` embedding matrix, converted to f32.
    matrix: Vec<f32>,
    dim: usize,
    /// Optional per-token-id scale factors (tensor `"weights"`).
    weights: Option<Vec<f32>>,
    /// Optional token-id remap (tensor `"mapping"`): row = mapping[id].
    mapping: Option<Vec<usize>>,
    tokenizer: Tokenizer,
    /// Token id of the tokenizer's unknown token, dropped before pooling.
    unk_id: Option<u32>,
    /// Median byte length of vocabulary keys; drives character truncation.
    median_token_length: usize,
    /// `normalize` from config.json (default true): L2-normalize outputs.
    normalize: bool,
    /// SHA-256 over matrix, tokenizer, and config bytes, lowercase hex.
    model_hash: String,
}

impl Model2Vec {
    /// Load a model from a directory holding `model.safetensors`,
    /// `tokenizer.json`, and `config.json`.
    ///
    /// No network is touched, ever, whatever the directory holds:
    /// this constructor loads what the caller supplied and nothing else.
    /// [`Model2Vec::potion_base_8m`] is the one that provisions.
    ///
    /// Paths are trusted as given, matching the `Udpipe::from_path`
    /// precedent for caller-supplied model locations: no size cap (the
    /// artifacts are the model, and a huge one fails in safetensors
    /// validation, not in an OOM loop) and no symlink rejection.
    ///
    /// # Errors
    ///
    /// [`Error::ModelNotFound`] if a file is absent, [`Error::Io`] if one
    /// cannot be read, [`Error::ModelInvalid`] if the artifact does not
    /// parse, uses an embedding dtype other than f32, or panics the
    /// loader internally (the panic is caught at this boundary).
    pub fn from_dir(dir: impl AsRef<Path>) -> domain::Result<Self> {
        let dir = dir.as_ref();
        let [model_path, tokenizer_path, config_path] = ARTIFACT_FILES.map(|f| dir.join(f));
        for p in [&model_path, &tokenizer_path, &config_path] {
            if !p.exists() {
                return Err(Error::ModelNotFound(p.clone()));
            }
        }
        let model_bytes = fs::read(&model_path)?;
        let tokenizer_bytes = fs::read(&tokenizer_path)?;
        let config_bytes = fs::read(&config_path)?;
        catch_embed_panic(|| Self::from_bytes(&model_bytes, &tokenizer_bytes, &config_bytes))
    }

    /// Load the pinned reference model from `dir`, downloading it first
    /// if it is not already there.
    ///
    /// The bytes are trusted only if the SHA-256 over all three artifact
    /// files, in the order `model.safetensors`, `tokenizer.json`,
    /// `config.json`, equals a constant compiled into this file. That is the UDPipe discipline applied to the
    /// embedding model, and it is why "downloads a model" is not the
    /// same claim as "reaches the network for whatever is there":
    /// exactly one artifact set can ever load through this constructor,
    /// and it is named in the source. ADR-0010 decision 6 is amended to
    /// say so.
    ///
    /// The sequence, and the rule it turns on: this constructor only
    /// ever removes files it downloaded itself.
    ///
    /// 1. All three files present and matching the digest: the model
    ///    parses from those same buffers, and nothing is fetched.
    /// 2. All three present and not matching, or only some of them
    ///    present: [`Error::ModelInvalid`] naming `dir`. Nothing is
    ///    downloaded and nothing is removed. The three filenames belong
    ///    to the model2vec artifact format rather than to this one
    ///    model, so a caller who put their own artifacts there would
    ///    otherwise lose them, and a partial set has no provenance worth
    ///    trusting either.
    /// 3. None of the three present: all three are downloaded into
    ///    `dir` (created if needed), as one transaction. Each lands
    ///    through a temporary file in the same directory, and the renames
    ///    onto the artifact names run only once all three have arrived,
    ///    so no reader ever sees a partial artifact or a partial set. A
    ///    failure anywhere in the sequence removes every file the call
    ///    made and returns the error. A download is capped at 64 MiB.
    /// 4. A mismatch over files this call downloaded removes them and
    ///    downloads once more. A second mismatch removes them again and
    ///    returns [`Error::ModelInvalid`].
    ///
    /// So a caller either gets the pinned model or gets an error. A
    /// half-verified or partially-written model is never loaded, a
    /// failed download leaves nothing on disk for a later call to pick
    /// up, and bytes the provisioner did not write are never its to
    /// delete. [`Model2Vec::from_dir`] loads a directory of your own
    /// without any of this.
    ///
    /// **No TOCTOU window.** The bytes that satisfy the digest are the
    /// bytes that get parsed; nothing re-reads the directory between
    /// verify and load.
    ///
    /// ```no_run
    /// use matra::config::Config;
    /// use matra::embed::model2vec::Model2Vec;
    ///
    /// // A directory the caller owns, because provisioning trusts what
    /// // it finds there; a shared one such as `/tmp` is writable by
    /// // anybody on the machine.
    /// let cfg = Config::resolve()?;
    /// let model = Model2Vec::potion_base_8m(cfg.model_dir().join(cfg.embedding_model()))?;
    /// # Ok::<(), matra::domain::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if `dir` cannot be created, read, or written, and
    /// if a download fails at the transport or answers with a non-2xx
    /// status: the message names the URL, and the `io::ErrorKind`
    /// separates a timeout from an unreachable host.
    /// [`Error::InputTooLarge`] with `what` set to
    /// `"embedding_download"` if a response exceeds the 64 MiB cap.
    /// [`Error::ModelInvalid`] if `dir` already holds artifacts that are
    /// not the pinned set, if the digest still mismatches after one
    /// re-download, or if the verified bytes do not parse.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn potion_base_8m(dir: impl AsRef<Path>) -> domain::Result<Self> {
        Self::potion_base_8m_with(dir.as_ref(), fetch_capped)
    }

    /// [`Model2Vec::potion_base_8m`] over the directory a [`Config`]
    /// resolves: the model directory joined with the configured
    /// embedding model name.
    ///
    /// Additive. The explicit-directory constructors are unchanged, and
    /// this one exists so a caller with no opinion about where models
    /// live does not have to invent one.
    ///
    /// ```no_run
    /// use matra::embed::model2vec::Model2Vec;
    ///
    /// let cfg = matra::config::Config::resolve()?;
    /// let model = Model2Vec::from_config(&cfg)?;
    /// # Ok::<(), matra::domain::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Whatever [`Model2Vec::potion_base_8m`] returns.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_config(cfg: &Config) -> domain::Result<Self> {
        Self::potion_base_8m(cfg.model_dir().join(cfg.embedding_model()))
    }

    /// [`Model2Vec::potion_base_8m`] with the fetcher as an argument.
    /// The public constructor passes [`fetch_capped`]; tests pass a
    /// closure, so the download path is exercised without a network.
    #[cfg(not(target_arch = "wasm32"))]
    fn potion_base_8m_with<F>(dir: &Path, fetch: F) -> domain::Result<Self>
    where
        F: Fn(&str) -> domain::Result<Vec<u8>>,
    {
        Self::provision(dir, POTION_BASE_8M_SHA256, &POTION_BASE_8M_URLS, &fetch)
    }

    /// The provisioning sequence [`Model2Vec::potion_base_8m`] documents,
    /// with the three things that make it "potion-base-8M" as arguments:
    /// the pinned digest, the pinned URLs, and the fetcher.
    ///
    /// The pin is a parameter rather than a constant read from scope so
    /// that a test can pin a three-file fixture of its own. A test that
    /// could vary only the fetcher would be testing the fetcher; the
    /// behavior worth pinning is what the digest decides.
    ///
    /// Every path out of here leaves `dir` holding either the full pinned
    /// set or none of the three names. [`download_artifacts`] is one
    /// transaction, so a fetch that fails partway leaves nothing behind,
    /// and a digest mismatch over files this call downloaded clears them
    /// with [`remove_artifacts`] before the retry and again if the retry
    /// mismatches too. The alternative is a directory left holding one
    /// artifact, which every later call refuses as a partial set: a
    /// transient timeout would become a permanent failure that only a
    /// hand deletion clears.
    #[cfg(not(target_arch = "wasm32"))]
    fn provision<F>(
        dir: &Path,
        expected_digest: &str,
        urls: &[&str; 3],
        fetch: &F,
    ) -> domain::Result<Self>
    where
        F: Fn(&str) -> domain::Result<Vec<u8>>,
    {
        let paths: [PathBuf; 3] = ARTIFACT_FILES.map(|f| dir.join(f));
        let present = paths.iter().filter(|p| p.exists()).count();

        // Files this call did not write are never files it removes. The
        // three names are the artifact format's, not this model's, so a
        // caller who pointed the configured embedding name at a
        // directory holding their own model has a full set sitting here.
        if present == ARTIFACT_FILES.len() {
            return match read_verify_load(&paths, expected_digest)? {
                Some(model) => Ok(model),
                None => Err(not_the_pinned_model(
                    dir,
                    "the three artifacts already there are not the pinned reference model",
                    expected_digest,
                )),
            };
        }
        if present > 0 {
            return Err(not_the_pinned_model(
                dir,
                "only part of an artifact set is there, and a partial set carries no provenance",
                expected_digest,
            ));
        }

        // None of the three names is taken, so the directory is this
        // call's to fill, and what lands in it is this call's to remove.
        download_artifacts(dir, &paths, urls, fetch)?;
        if let Some(model) = read_verify_load(&paths, expected_digest)? {
            return Ok(model);
        }

        // One retry, over the files this call just downloaded. The
        // likely cause of a mismatch is a truncated or interrupted
        // transfer, and that heals. Removal comes first so a second
        // failure can never leave bytes behind that a later call would
        // find and refuse.
        remove_artifacts(&paths);
        download_artifacts(dir, &paths, urls, fetch)?;
        match read_verify_load(&paths, expected_digest)? {
            Some(model) => Ok(model),
            None => {
                remove_artifacts(&paths);
                Err(Error::ModelInvalid(format!(
                    "artifact digest mismatch after re-download in {}: expected {expected_digest}",
                    dir.display()
                )))
            }
        }
    }

    /// Parse a model from in-memory artifact bytes. The identity hash is
    /// computed here over the same buffers that get parsed, so it always
    /// matches what was loaded.
    fn from_bytes(
        model_bytes: &[u8],
        tokenizer_bytes: &[u8],
        config_bytes: &[u8],
    ) -> domain::Result<Self> {
        // All three artifacts determine the output vectors (the tokenizer
        // decides which rows pool, config's normalize flips the geometry),
        // so the identity covers all three, in this order.
        let mut hasher = Sha256::new();
        hasher.update(model_bytes);
        hasher.update(tokenizer_bytes);
        hasher.update(config_bytes);
        let model_hash = format!("{:x}", hasher.finalize());

        let tensors = SafeTensors::deserialize(model_bytes)
            .map_err(|e| Error::ModelInvalid(format!("safetensors: {e}")))?;
        let names = tensors.names();
        let emb_name = ["embeddings", "0", "embedding.weight"]
            .into_iter()
            .find(|n| names.iter().any(|t| t == n))
            .ok_or_else(|| {
                Error::ModelInvalid(
                    "no embeddings tensor (tried: embeddings, 0, embedding.weight)".into(),
                )
            })?;
        let emb = tensors
            .tensor(emb_name)
            .map_err(|e| Error::ModelInvalid(format!("embeddings tensor: {e}")))?;
        if emb.shape().len() != 2 {
            return Err(Error::ModelInvalid(format!(
                "embeddings tensor must be 2-dimensional, got shape {:?}",
                emb.shape()
            )));
        }
        let (vocab, dim) = (emb.shape()[0], emb.shape()[1]);
        if vocab == 0 || dim == 0 {
            return Err(Error::ModelInvalid(format!(
                "embeddings tensor has a zero dimension: shape {vocab}x{dim}"
            )));
        }
        let matrix = f32_tensor(emb.dtype(), emb.data(), "embeddings")?;
        if matrix.len() != vocab * dim {
            return Err(Error::ModelInvalid(format!(
                "embeddings tensor data length {} does not match shape {}x{}",
                matrix.len(),
                vocab,
                dim
            )));
        }

        let weights = match tensors.tensor("weights") {
            Ok(t) => Some(f32_tensor(t.dtype(), t.data(), "weights")?),
            Err(_) => None,
        };
        let mapping = match tensors.tensor("mapping") {
            Ok(t) => Some(index_tensor(t.dtype(), t.data())?),
            Err(_) => None,
        };
        // Malformation is loud at load, not silent at embed. Every mapping
        // entry must be a valid matrix row, and a weights table must cover
        // the id space it is indexed by (token ids, so the mapping's
        // length when one exists, the matrix vocab otherwise). The Python
        // reference tolerates short weights by defaulting to 1.0; a
        // half-scaled artifact is a defect, not a model, so matra rejects
        // it (resilience floor over quiet parity on malformed input).
        if let Some(map) = &mapping {
            if let Some(&bad) = map.iter().find(|&&row| row >= vocab) {
                return Err(Error::ModelInvalid(format!(
                    "mapping entry {bad} outside matrix vocab {vocab}"
                )));
            }
        }
        if let Some(w) = &weights {
            let id_space = mapping.as_ref().map_or(vocab, Vec::len);
            if w.len() < id_space {
                return Err(Error::ModelInvalid(format!(
                    "weights tensor length {} does not cover the token id space {id_space}",
                    w.len()
                )));
            }
        }

        let tokenizer = Tokenizer::from_bytes(tokenizer_bytes)
            .map_err(|e| Error::ModelInvalid(format!("tokenizer: {e}")))?;

        // The unknown token lives in tokenizer.json under the model key;
        // the tokenizers API exposes no direct accessor, so read it from
        // the same JSON the tokenizer was built from. WordPiece/BPE store
        // it as a string (`unk_token`); Unigram stores an index
        // (`unk_id`). Missing both means no unk filtering, which is the
        // reference behavior for such tokenizers.
        let tokenizer_json: serde_json::Value = serde_json::from_slice(tokenizer_bytes)
            .map_err(|e| Error::ModelInvalid(format!("tokenizer.json: {e}")))?;
        let model_obj = tokenizer_json.get("model");
        let unk_id = model_obj
            .and_then(|m| m.get("unk_token"))
            .and_then(|u| u.as_str())
            .and_then(|s| tokenizer.token_to_id(s))
            .or_else(|| {
                model_obj
                    .and_then(|m| m.get("unk_id"))
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|v| u32::try_from(v).ok())
            });

        let mut lengths: Vec<usize> = tokenizer
            .get_vocab(false)
            .keys()
            .map(|token| token.len())
            .collect();
        lengths.sort_unstable();
        // Upper middle on even-length vocabularies, matching the Rust
        // reference; only shifts the truncation heuristic on long inputs.
        let median_token_length = lengths.get(lengths.len() / 2).copied().unwrap_or(1).max(1);

        let config: serde_json::Value = serde_json::from_slice(config_bytes)
            .map_err(|e| Error::ModelInvalid(format!("config.json: {e}")))?;
        let normalize = config
            .get("normalize")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);

        Ok(Self {
            matrix,
            dim,
            weights,
            mapping,
            tokenizer,
            unk_id,
            median_token_length,
            normalize,
            model_hash,
        })
    }

    /// SHA-256 over the three artifact files (matrix, tokenizer, config,
    /// in that order), lowercase hex. The model identity for
    /// provenance-carrying result types; all three files determine the
    /// output vectors, so all three are covered.
    #[must_use]
    pub fn model_hash(&self) -> &str {
        &self.model_hash
    }

    /// Number of dimensions of every vector this model produces.
    #[must_use]
    pub fn dimensions(&self) -> usize {
        self.dim
    }

    /// Truncate to at most `max_bytes` bytes on a char boundary, the
    /// reference implementation's cheap pre-tokenization cap.
    fn truncate_str(text: &str, max_bytes: usize) -> &str {
        if text.len() <= max_bytes {
            return text;
        }
        let mut end = max_bytes;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        &text[..end]
    }

    /// Embed one text: truncate, tokenize without special tokens, drop
    /// unknown tokens, cap at [`DEFAULT_MAX_TOKENS`], gather rows
    /// (through the mapping remap when present), scale by per-token
    /// weights when present, mean-pool, and L2-normalize if configured.
    /// An empty token list yields the zero vector, matching the
    /// reference.
    fn embed_one(&self, text: &str) -> domain::Result<Embedding> {
        let truncated = Self::truncate_str(
            text,
            DEFAULT_MAX_TOKENS.saturating_mul(self.median_token_length),
        );
        let encoding = self
            .tokenizer
            .encode_fast(truncated, false)
            .map_err(|e| Error::ModelInvalid(format!("tokenize: {e}")))?;
        let mut ids: Vec<u32> = encoding.get_ids().to_vec();
        if let Some(unk) = self.unk_id {
            ids.retain(|&id| id != unk);
        }
        ids.truncate(DEFAULT_MAX_TOKENS);

        let vocab = self.matrix.len() / self.dim;
        let mut sum = vec![0.0f32; self.dim];
        let mut count = 0usize;
        for &id in &ids {
            let row = match &self.mapping {
                Some(map) => match map.get(id as usize) {
                    Some(&mapped) => mapped,
                    None => {
                        return Err(Error::ModelInvalid(format!(
                            "token id {id} outside mapping (len {})",
                            map.len()
                        )));
                    }
                },
                None => id as usize,
            };
            if row >= vocab {
                return Err(Error::ModelInvalid(format!(
                    "token id {id} maps to row {row} outside vocab {vocab}"
                )));
            }
            // Indexed by original token id, per the reference; load-time
            // validation makes a miss unreachable, so 1.0 is defensive,
            // not a silent tolerance.
            let scale = self
                .weights
                .as_ref()
                .and_then(|w| w.get(id as usize))
                .copied()
                .unwrap_or(1.0);
            let base = row * self.dim;
            for (s, v) in sum.iter_mut().zip(&self.matrix[base..base + self.dim]) {
                *s += v * scale;
            }
            count += 1;
        }
        let denom = count.max(1) as f32;
        for s in &mut sum {
            *s /= denom;
        }
        if self.normalize {
            let norm = sum.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-12);
            for s in &mut sum {
                *s /= norm;
            }
        }
        Ok(Embedding(sum))
    }
}

impl Embedder for Model2Vec {
    fn embed(&self, texts: &[&str]) -> domain::Result<Vec<Embedding>> {
        catch_embed_panic(|| texts.iter().map(|t| self.embed_one(t)).collect())
    }

    fn identity(&self) -> &str {
        self.model_hash()
    }
}

/// Read the three artifacts once, hash those buffers, and parse the model
/// from the same buffers when the digest matches.
///
/// Returns `Ok(None)` on a digest mismatch, which is the caller's signal
/// to remove and retry. Verification happens before parsing rather than
/// after, and not only for the obvious reason: if the parse ran first, a
/// corrupt artifact would fail as [`Error::ModelInvalid`] from
/// safetensors and never reach the retry at all.
///
/// [`Model2Vec::from_bytes`] hashes the same three buffers again to
/// compute the model identity. That is one duplicated pass over 30 MB,
/// kept because the two hashes answer different questions: this one is a
/// gate on bytes that are not yet trusted, the other is a property of a
/// model that loaded.
#[cfg(not(target_arch = "wasm32"))]
fn read_verify_load(
    paths: &[PathBuf; 3],
    expected_digest: &str,
) -> domain::Result<Option<Model2Vec>> {
    let model_bytes = fs::read(&paths[0])?;
    let tokenizer_bytes = fs::read(&paths[1])?;
    let config_bytes = fs::read(&paths[2])?;

    let mut hasher = Sha256::new();
    hasher.update(&model_bytes);
    hasher.update(&tokenizer_bytes);
    hasher.update(&config_bytes);
    let got = format!("{:x}", hasher.finalize());
    if !got.eq_ignore_ascii_case(expected_digest) {
        return Ok(None);
    }

    catch_embed_panic(|| Model2Vec::from_bytes(&model_bytes, &tokenizer_bytes, &config_bytes))
        .map(Some)
}

/// The refusal for a directory the provisioner did not fill itself.
///
/// It names the directory, the reason, and every way out, because the
/// caller reached a constructor that provisions and was told no: either
/// those files are a model of their own (load them with
/// [`Model2Vec::from_dir`]), or they are a stale cache (remove them), or
/// the directory is the wrong one (point the configuration elsewhere).
#[cfg(not(target_arch = "wasm32"))]
fn not_the_pinned_model(dir: &Path, reason: &str, expected_digest: &str) -> Error {
    Error::ModelInvalid(format!(
        "{}: {reason} (expected the three-file SHA-256 {expected_digest}). \
         Nothing was downloaded and nothing was removed. Load a model of your own with \
         Model2Vec::from_dir, remove the files there to provision the pinned model again, \
         or point MATRA_MODEL_DIR or the configured embedding model name at another \
         directory.",
        dir.display()
    ))
}

/// Remove the named paths, ignoring failures. A file that is already
/// gone is the state this wants; one that cannot be removed surfaces at
/// the next verify, which is the loud place for it.
///
/// The one cleanup in this file. [`Transaction`] unwinds a failed
/// download with it, and [`Model2Vec::provision`] clears the artifacts of
/// a mismatched download with it before the retry.
#[cfg(not(target_arch = "wasm32"))]
fn remove_artifacts(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

/// The paths one download created, removed together unless every step of
/// that download succeeded.
///
/// A download is all three artifacts or none of them. The three names
/// appearing one at a time would leave a window in which the directory
/// holds a partial set, and a partial set is what
/// [`Model2Vec::provision`] refuses on the next call: a fetch that timed
/// out on the second artifact would otherwise wedge the directory until
/// somebody deleted files by hand. So every fetch and every write happens
/// first, into temporaries, and the renames start only once nothing is
/// left that can fail on a fetch.
///
/// Cleanup runs from `Drop`, so a panic between the first write and the
/// commit removes what this call made, exactly as an error does. It only
/// ever names paths this call created: a temporary whose open failed was
/// never registered, so a file somebody else put at that name is left
/// exactly as it was found.
#[cfg(not(target_arch = "wasm32"))]
struct Transaction {
    /// Temporaries this call opened.
    temps: Vec<PathBuf>,
    /// Artifacts this call renamed into place.
    landed: Vec<PathBuf>,
    committed: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl Transaction {
    fn new() -> Transaction {
        Transaction {
            temps: Vec::new(),
            landed: Vec::new(),
            committed: false,
        }
    }

    /// Write `bytes` to `tmp`, which must not already exist.
    ///
    /// The temporary is opened with `create_new`, which is `O_EXCL`: a
    /// path that already exists fails the open, symlink or not. A
    /// predictable name in a directory somebody else can write to is
    /// otherwise a place to plant a symlink, and a plain write would
    /// follow it and put 30 MB through to whatever it points at. Failing
    /// is the right answer and the message names the path, because what
    /// to do about a leftover temporary from a killed process is remove
    /// it, which is not a decision this code gets to make on its own.
    fn write_temp(&mut self, tmp: &Path, bytes: &[u8]) -> domain::Result<()> {
        use std::io::Write;

        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(tmp)
            .map_err(|e| {
                std::io::Error::new(
                    e.kind(),
                    format!(
                        "cannot create the temporary file {}: {e} (a leftover from a killed \
                         process, or one planted there; remove it and try again)",
                        tmp.display()
                    ),
                )
            })?;
        // Registered only once the file is this call's own, so a path
        // that was already there is never this call's to remove.
        self.temps.push(tmp.to_path_buf());
        file.write_all(bytes)?;
        Ok(())
    }

    /// Move one written temporary onto its artifact name.
    ///
    /// `std::fs::rename` is atomic on one filesystem (POSIX `rename(2)`,
    /// Windows `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`), so no
    /// reader sees a half-written artifact, and two processes
    /// provisioning into the same directory cannot interleave: the
    /// temporary name carries the process id and the rename is the only
    /// operation that touches the final path.
    fn rename_into_place(&mut self, tmp: &Path, final_path: &Path) -> domain::Result<()> {
        fs::rename(tmp, final_path)?;
        self.landed.push(final_path.to_path_buf());
        Ok(())
    }

    /// All three artifacts are in place. Nothing is removed from here on.
    fn commit(&mut self) {
        self.committed = true;
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for Transaction {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        remove_artifacts(&self.temps);
        remove_artifacts(&self.landed);
    }
}

/// Fetch all three artifacts and put them in place, all of them or none.
///
/// One transaction: every artifact is fetched and written to a temporary
/// in `dir` first, and only when all three have arrived does a rename put
/// any of them under an artifact name. A failure at any point (a fetch,
/// the size cap, a temporary that cannot be opened, a rename) removes
/// every temporary and every artifact this call had already renamed, then
/// returns the error, so the directory is left as this call found it.
///
/// This is deliberately a second implementation of the temp-then-rename
/// pattern `nlp/udpipe.rs` uses rather than a shared helper. The two
/// adapters share no module by design, and a utility module both imported
/// would put a third file into the wiring to save twenty lines.
#[cfg(not(target_arch = "wasm32"))]
fn download_artifacts<F>(
    dir: &Path,
    paths: &[PathBuf; 3],
    urls: &[&str; 3],
    fetch: &F,
) -> domain::Result<()>
where
    F: Fn(&str) -> domain::Result<Vec<u8>>,
{
    fs::create_dir_all(dir)?;
    let temps: [PathBuf; 3] =
        ARTIFACT_FILES.map(|name| dir.join(format!(".tmp.{name}.{}", std::process::id())));
    let mut transaction = Transaction::new();

    for i in 0..ARTIFACT_FILES.len() {
        let bytes = fetch(urls[i])?;
        // The cap is enforced here, at the one place every artifact
        // passes through, rather than inside the fetcher: the fetcher is
        // replaceable and the bound is not.
        if bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(Error::InputTooLarge {
                limit: MAX_ARTIFACT_BYTES,
                actual: bytes.len(),
                what: "embedding_download",
            });
        }
        transaction.write_temp(&temps[i], &bytes)?;
    }

    // Past this line nothing fetches, so every rename runs against bytes
    // already on disk and the set becomes visible as a set.
    for i in 0..ARTIFACT_FILES.len() {
        transaction.rename_into_place(&temps[i], &paths[i])?;
    }
    transaction.commit();
    Ok(())
}

/// Fetch one URL into memory, reading at most one byte past
/// [`MAX_ARTIFACT_BYTES`].
///
/// Stopping one byte over is what lets the caller tell "at the cap" from
/// "over it" while keeping the read bounded: an endless or misdirected
/// response costs 64 MiB and a rejection, not the machine's memory. The
/// consequence is that the `actual` on the resulting
/// [`Error::InputTooLarge`] is the bound that was breached rather than
/// the response's full length, which is not knowable without reading it.
///
/// `ureq` treats a non-2xx status as an error by default, so an HTML
/// error page never reaches the digest.
///
/// The call is bounded in time as well as in size: [`FETCH_TIMEOUT`] end
/// to end and [`CONNECT_TIMEOUT`] on the connection, because `ureq` sets
/// no timeout of its own and a socket that accepts and then says nothing
/// would otherwise block the caller for as long as it stays open.
#[cfg(not(target_arch = "wasm32"))]
fn fetch_capped(url: &str) -> domain::Result<Vec<u8>> {
    use std::io::Read;

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(FETCH_TIMEOUT))
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .build()
        .into();
    let response = agent
        .get(url)
        .call()
        .map_err(|e| transport_failure(url, &e))?;
    let mut bytes = Vec::new();
    response
        .into_body()
        .into_reader()
        .take(MAX_ARTIFACT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| {
            Error::Io(std::io::Error::new(
                e.kind(),
                format!("download {url}: {e}"),
            ))
        })?;
    Ok(bytes)
}

/// Map a `ureq` failure to [`Error::Io`].
///
/// A download that never arrives is a transport failure, not an invalid
/// model: [`Error::ModelInvalid`] stays reserved for bytes that did
/// arrive and then failed the digest or the parser. A non-2xx status
/// belongs on this side too, because what came back was a server's
/// answer about the request rather than a model. The `io::ErrorKind` is
/// preserved where `ureq` knows it, so a caller can tell a timeout from
/// an unreachable host without reading the message.
#[cfg(not(target_arch = "wasm32"))]
fn transport_failure(url: &str, error: &ureq::Error) -> Error {
    use std::io::ErrorKind;

    let kind = match error {
        ureq::Error::Io(e) => e.kind(),
        ureq::Error::Timeout(_) => ErrorKind::TimedOut,
        ureq::Error::ConnectionFailed | ureq::Error::HostNotFound => ErrorKind::NotConnected,
        _ => ErrorKind::Other,
    };
    Error::Io(std::io::Error::new(
        kind,
        format!("download {url}: {error}"),
    ))
}

/// Convert a tensor's raw little-endian bytes to f32. Only f32 sources
/// are supported; f16 and i8 artifacts exist in the wild and return
/// [`Error::ModelInvalid`] with the dtype named, so the gap is loud
/// rather than silently wrong.
fn f32_tensor(dtype: Dtype, data: &[u8], name: &str) -> domain::Result<Vec<f32>> {
    match dtype {
        Dtype::F32 => Ok(data
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect()),
        other => Err(Error::ModelInvalid(format!(
            "{name} tensor dtype {other:?} not supported (f32 only in this build)"
        ))),
    }
}

/// Convert an i32/i64 index tensor to usize rows, rejecting negatives.
fn index_tensor(dtype: Dtype, data: &[u8]) -> domain::Result<Vec<usize>> {
    let to_usize = |v: i64| -> domain::Result<usize> {
        usize::try_from(v).map_err(|_| Error::ModelInvalid(format!("negative mapping index {v}")))
    };
    match dtype {
        Dtype::I32 => data
            .chunks_exact(4)
            .map(|b| to_usize(i64::from(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))))
            .collect(),
        Dtype::I64 => data
            .chunks_exact(8)
            .map(|b| {
                to_usize(i64::from_le_bytes([
                    b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
                ]))
            })
            .collect(),
        other => Err(Error::ModelInvalid(format!(
            "mapping tensor dtype {other:?} not supported (i32/i64 only)"
        ))),
    }
}

/// Run a closure that may panic inside the safetensors or tokenizers
/// parsing paths, converting any panic to [`Error::ModelInvalid`]. Both
/// crates are pure Rust, so a panic here is a Rust panic on malformed
/// input, not a C abort; the boundary exists because library code must
/// not panic (the UDPipe precedent, same shape).
fn catch_embed_panic<F, T>(f: F) -> domain::Result<T>
where
    F: FnOnce() -> domain::Result<T>,
{
    use std::panic::{AssertUnwindSafe, catch_unwind};
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(result) => result,
        Err(payload) => {
            let message = payload
                .downcast_ref::<&'static str>()
                .map(|s| (*s).to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "model2vec panic (no message captured)".to_string());
            Err(Error::ModelInvalid(format!(
                "model2vec panicked: {message}"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The normalized tiny model, shared with the parity module.
    pub(super) fn write_model_normalized(dir: &Path) {
        write_model(dir, None, r#"{"normalize": true}"#);
    }
    use safetensors::tensor::TensorView;
    use std::collections::HashMap;

    /// Vocab: [UNK]=0, hello=1, world=2. Row 0 is poison so any test
    /// that pools it fails loudly.
    const MATRIX: [[f32; 4]; 3] = [
        [9.0, 9.0, 9.0, 9.0],
        [1.0, 0.0, 2.0, 0.0],
        [0.0, 2.0, 0.0, 4.0],
    ];

    const TOKENIZER_JSON: &str = r###"{
        "version": "1.0",
        "truncation": null,
        "padding": null,
        "added_tokens": [],
        "normalizer": {"type": "BertNormalizer", "clean_text": true,
            "handle_chinese_chars": true, "strip_accents": null,
            "lowercase": true},
        "pre_tokenizer": {"type": "BertPreTokenizer"},
        "post_processor": null,
        "decoder": null,
        "model": {"type": "WordPiece", "unk_token": "[UNK]",
            "continuing_subword_prefix": "##",
            "max_input_chars_per_word": 100,
            "vocab": {"[UNK]": 0, "hello": 1, "world": 2}}
    }"###;

    fn tensor_bytes(rows: &[[f32; 4]]) -> Vec<u8> {
        rows.iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect()
    }

    fn write_model_full(
        dir: &Path,
        weights: Option<&[f32]>,
        mapping: Option<&[i64]>,
        config: &str,
    ) {
        let emb = tensor_bytes(&MATRIX);
        let mut tensors = HashMap::new();
        tensors.insert(
            "embeddings".to_string(),
            TensorView::new(Dtype::F32, vec![3, 4], &emb).unwrap(),
        );
        let wbytes: Vec<u8>;
        if let Some(w) = weights {
            wbytes = w.iter().flat_map(|v| v.to_le_bytes()).collect();
            tensors.insert(
                "weights".to_string(),
                TensorView::new(Dtype::F32, vec![w.len()], &wbytes).unwrap(),
            );
        }
        let mbytes: Vec<u8>;
        if let Some(m) = mapping {
            mbytes = m.iter().flat_map(|v| v.to_le_bytes()).collect();
            tensors.insert(
                "mapping".to_string(),
                TensorView::new(Dtype::I64, vec![m.len()], &mbytes).unwrap(),
            );
        }
        let bytes = safetensors::serialize(&tensors, None).unwrap();
        fs::write(dir.join("model.safetensors"), bytes).unwrap();
        fs::write(dir.join("tokenizer.json"), TOKENIZER_JSON).unwrap();
        fs::write(dir.join("config.json"), config).unwrap();
    }

    fn write_model(dir: &Path, weights: Option<&[f32]>, config: &str) {
        write_model_full(dir, weights, None, config);
    }

    fn load(weights: Option<&[f32]>, config: &str) -> Model2Vec {
        let dir = tempfile::tempdir().unwrap();
        write_model(dir.path(), weights, config);
        Model2Vec::from_dir(dir.path()).unwrap()
    }

    fn load_full(weights: Option<&[f32]>, mapping: Option<&[i64]>, config: &str) -> Model2Vec {
        let dir = tempfile::tempdir().unwrap();
        write_model_full(dir.path(), weights, mapping, config);
        Model2Vec::from_dir(dir.path()).unwrap()
    }

    #[test]
    fn mean_pool_is_exact() {
        let m = load(None, r#"{"normalize": false}"#);
        let out = m.embed(&["hello world"]).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, vec![0.5, 1.0, 1.0, 2.0]);
    }

    #[test]
    fn normalization_is_exact() {
        // norm of [0.5, 1, 1, 2] is sqrt(6.25) = 2.5, exact in f32.
        let m = load(None, r#"{"normalize": true}"#);
        let out = m.embed(&["hello world"]).unwrap();
        assert_eq!(out[0].0, vec![0.2, 0.4, 0.4, 0.8]);
    }

    #[test]
    fn normalize_defaults_to_true_when_absent() {
        let m = load(None, "{}");
        let out = m.embed(&["hello world"]).unwrap();
        assert_eq!(out[0].0, vec![0.2, 0.4, 0.4, 0.8]);
    }

    #[test]
    fn unknown_tokens_are_dropped_not_pooled() {
        let m = load(None, r#"{"normalize": false}"#);
        // "xyzzy" tokenizes to [UNK]; only "hello" (row 1) remains.
        let out = m.embed(&["hello xyzzy"]).unwrap();
        assert_eq!(out[0].0, vec![1.0, 0.0, 2.0, 0.0]);
    }

    #[test]
    fn per_token_weights_scale_before_pooling() {
        let m = load(Some(&[1.0, 2.0, 0.5]), r#"{"normalize": false}"#);
        // hello scaled by 2 -> [2,0,4,0]; world by 0.5 -> [0,1,0,2].
        let out = m.embed(&["hello world"]).unwrap();
        assert_eq!(out[0].0, vec![1.0, 0.5, 2.0, 1.0]);
    }

    #[test]
    fn empty_and_all_unknown_input_yield_zero_vectors() {
        let m = load(None, r#"{"normalize": true}"#);
        let out = m.embed(&["", "xyzzy qwerty"]).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].0, vec![0.0; 4]);
        assert_eq!(out[1].0, vec![0.0; 4]);
    }

    #[test]
    fn contract_length_and_dimension_hold() {
        let m = load(None, "{}");
        let texts = ["hello", "world", "hello world hello", ""];
        let out = m.embed(&texts).unwrap();
        assert_eq!(out.len(), texts.len());
        assert!(out.iter().all(|e| e.0.len() == m.dimensions()));
    }

    #[test]
    fn model_hash_covers_all_three_artifact_files() {
        let dir = tempfile::tempdir().unwrap();
        write_model(dir.path(), None, "{}");
        let mut hasher = Sha256::new();
        for f in ["model.safetensors", "tokenizer.json", "config.json"] {
            hasher.update(fs::read(dir.path().join(f)).unwrap());
        }
        let expected = format!("{:x}", hasher.finalize());
        let m = Model2Vec::from_dir(dir.path()).unwrap();
        assert_eq!(m.model_hash(), expected);

        // A config-only change is a different geometry, so a different
        // identity, which is the point of covering all three files.
        let dir2 = tempfile::tempdir().unwrap();
        write_model(dir2.path(), None, r#"{"normalize": false}"#);
        let m2 = Model2Vec::from_dir(dir2.path()).unwrap();
        assert_ne!(m.model_hash(), m2.model_hash());
    }

    #[test]
    fn mapping_remaps_token_ids_to_rows() {
        // Swap hello and world: id 1 -> row 2, id 2 -> row 1.
        let m = load_full(None, Some(&[0, 2, 1]), r#"{"normalize": false}"#);
        let out = m.embed(&["hello"]).unwrap();
        assert_eq!(out[0].0, vec![0.0, 2.0, 0.0, 4.0]);
    }

    #[test]
    fn weights_index_by_original_token_id_not_mapped_row() {
        // Mapping swaps rows; weights[1]=2.0 must scale token id 1
        // (hello, now gathering row 2), pinning the reference semantics.
        let m = load_full(
            Some(&[1.0, 2.0, 1.0]),
            Some(&[0, 2, 1]),
            r#"{"normalize": false}"#,
        );
        let out = m.embed(&["hello"]).unwrap();
        assert_eq!(out[0].0, vec![0.0, 4.0, 0.0, 8.0]);
    }

    #[test]
    fn mapping_entry_outside_vocab_is_rejected_at_load() {
        let dir = tempfile::tempdir().unwrap();
        write_model_full(dir.path(), None, Some(&[0, 3, 1]), "{}");
        match Model2Vec::from_dir(dir.path()) {
            Err(Error::ModelInvalid(msg)) => assert!(msg.contains("mapping entry")),
            other => panic!("expected ModelInvalid, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn short_weights_tensor_is_rejected_at_load() {
        let dir = tempfile::tempdir().unwrap();
        write_model(dir.path(), Some(&[1.0, 2.0]), "{}");
        match Model2Vec::from_dir(dir.path()) {
            Err(Error::ModelInvalid(msg)) => assert!(msg.contains("weights tensor length")),
            other => panic!("expected ModelInvalid, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn zero_dimension_matrix_is_rejected_at_load() {
        let dir = tempfile::tempdir().unwrap();
        let mut tensors = HashMap::new();
        let empty: Vec<u8> = Vec::new();
        tensors.insert(
            "embeddings".to_string(),
            TensorView::new(Dtype::F32, vec![3, 0], &empty).unwrap(),
        );
        let bytes = safetensors::serialize(&tensors, None).unwrap();
        fs::write(dir.path().join("model.safetensors"), bytes).unwrap();
        fs::write(dir.path().join("tokenizer.json"), TOKENIZER_JSON).unwrap();
        fs::write(dir.path().join("config.json"), "{}").unwrap();
        match Model2Vec::from_dir(dir.path()) {
            Err(Error::ModelInvalid(msg)) => assert!(msg.contains("zero dimension")),
            other => panic!("expected ModelInvalid, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn missing_file_is_model_not_found() {
        let dir = tempfile::tempdir().unwrap();
        match Model2Vec::from_dir(dir.path()) {
            Err(Error::ModelNotFound(p)) => {
                assert!(p.ends_with("model.safetensors"));
            }
            Err(other) => panic!("expected ModelNotFound, got {other:?}"),
            Ok(_) => panic!("expected ModelNotFound, got a loaded model"),
        }
    }

    #[test]
    fn corrupt_safetensors_is_model_invalid_not_panic() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("model.safetensors"), b"garbage").unwrap();
        fs::write(dir.path().join("tokenizer.json"), TOKENIZER_JSON).unwrap();
        fs::write(dir.path().join("config.json"), "{}").unwrap();
        match Model2Vec::from_dir(dir.path()) {
            Err(Error::ModelInvalid(_)) => {}
            Err(other) => panic!("expected ModelInvalid, got {other:?}"),
            Ok(_) => panic!("expected ModelInvalid, got a loaded model"),
        }
    }

    #[test]
    fn truncation_respects_char_boundaries() {
        // A multi-byte char straddling the cut must not split.
        let text = "hé";
        let t = Model2Vec::truncate_str(text, 2);
        assert_eq!(t, "h");
        assert_eq!(Model2Vec::truncate_str(text, 3), "hé");
    }
}

#[cfg(test)]
mod parity {
    //! Parity against the Python reference implementation (model2vec
    //! 0.9.0). Expected vectors were produced by running
    //! `StaticModel.from_pretrained` + `encode` on the exact artifact
    //! `tests::write_model` constructs (same matrix, tokenizer, config,
    //! normalize=true). The reference computes in float64, so the
    //! comparison is tolerance-based here; bit-exactness is the
    //! Rust-across-targets property, asserted separately in `tests`.
    use super::tests::write_model_normalized;
    use super::*;

    const TOLERANCE: f32 = 1e-6;

    #[test]
    fn matches_python_reference_on_pinned_inputs() {
        let dir = tempfile::tempdir().unwrap();
        write_model_normalized(dir.path());
        let m = Model2Vec::from_dir(dir.path()).unwrap();

        let cases: [(&str, [f64; 4]); 6] = [
            ("hello world", [0.2, 0.4, 0.4, 0.8]),
            ("hello", [0.4472135954999579, 0.0, 0.8944271909999159, 0.0]),
            // xyzzy is [UNK], dropped: identical to "hello" alone.
            (
                "hello xyzzy",
                [0.4472135954999579, 0.0, 0.8944271909999159, 0.0],
            ),
            ("", [0.0, 0.0, 0.0, 0.0]),
            ("xyzzy qwerty", [0.0, 0.0, 0.0, 0.0]),
            // Exercises the BertNormalizer lowercase path.
            (
                "HELLO World hello",
                [
                    0.31622776601683794,
                    0.31622776601683794,
                    0.6324555320336759,
                    0.6324555320336759,
                ],
            ),
        ];
        let texts: Vec<&str> = cases.iter().map(|(t, _)| *t).collect();
        let out = m.embed(&texts).unwrap();
        for ((text, expected), got) in cases.iter().zip(&out) {
            for (i, (e, g)) in expected.iter().zip(&got.0).enumerate() {
                let diff = (*e as f32 - g).abs();
                assert!(
                    diff <= TOLERANCE,
                    "{text:?} dim {i}: python {e} vs rust {g} (diff {diff})"
                );
            }
        }
    }
}

#[cfg(test)]
mod bit_parity {
    //! The property the static adapter buys (ADR-0010): identical bits on
    //! every target. These constants were produced once on aarch64; the
    //! CI matrix runs this test on x86_64 and aarch64, so a target whose
    //! arithmetic dispatches differently fails loudly here.
    use super::tests::write_model_normalized;
    use super::*;

    #[test]
    fn vectors_are_bit_identical_across_targets() {
        let dir = tempfile::tempdir().unwrap();
        write_model_normalized(dir.path());
        let m = Model2Vec::from_dir(dir.path()).unwrap();

        let cases: [(&str, [u32; 4]); 3] = [
            (
                "hello world",
                [0x3e4c_cccd, 0x3ecc_cccd, 0x3ecc_cccd, 0x3f4c_cccd],
            ),
            (
                "hello",
                [0x3ee4_f92e, 0x0000_0000, 0x3f64_f92e, 0x0000_0000],
            ),
            (
                "HELLO World hello",
                [0x3ea1_e89c, 0x3ea1_e89c, 0x3f21_e89c, 0x3f21_e89c],
            ),
        ];
        let texts: Vec<&str> = cases.iter().map(|(t, _)| *t).collect();
        let out = m.embed(&texts).unwrap();
        for ((text, expected), got) in cases.iter().zip(&out) {
            let bits: Vec<u32> = got.0.iter().map(|v| v.to_bits()).collect();
            assert_eq!(&bits[..], expected, "bit drift on {text:?}");
        }
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod provisioning {
    //! The pinned-download path (i10 M4), exercised with the fetcher as
    //! an argument so no test here touches the network or needs the
    //! 30 MB reference artifact. The fixture is the same tiny three-file
    //! model the rest of this file's tests use, and its digest is
    //! computed here rather than pinned.
    use std::cell::RefCell;

    use super::tests::write_model_normalized;
    use super::*;

    /// Stand-ins for [`POTION_BASE_8M_URLS`]. Nothing dereferences them;
    /// they are the keys the test fetcher is asked for, and asserting on
    /// them is how a test shows which artifacts were requested.
    const URLS: [&str; 3] = ["test://model", "test://tokenizer", "test://config"];

    /// The three artifacts as bytes, plus the digest over them in
    /// [`ARTIFACT_FILES`] order.
    struct Fixture {
        bytes: [Vec<u8>; 3],
        digest: String,
    }

    impl Fixture {
        fn new() -> Fixture {
            let dir = tempfile::tempdir().unwrap();
            write_model_normalized(dir.path());
            let bytes = ARTIFACT_FILES.map(|name| fs::read(dir.path().join(name)).unwrap());
            let mut hasher = Sha256::new();
            for buf in &bytes {
                hasher.update(buf);
            }
            Fixture {
                bytes,
                digest: format!("{:x}", hasher.finalize()),
            }
        }

        fn write_into(&self, dir: &Path) {
            for (name, buf) in ARTIFACT_FILES.iter().zip(&self.bytes) {
                fs::write(dir.join(name), buf).unwrap();
            }
        }
    }

    /// What a fetcher hands back for every request.
    enum Serves {
        /// The fixture bytes, so a download heals the directory.
        TheModel,
        /// Bytes that parse as nothing, so the digest can never match.
        Garbage,
        /// Garbage for the first round of three requests and the fixture
        /// for every one after it: a corrupt download that heals on the
        /// retry, with the corruption in bytes this call produced.
        GarbageThenTheModel,
        /// One byte past the cap.
        TooMuch,
        /// The fixture for the first request and a transport failure for
        /// every one after it: a download that dies partway through.
        TheFirstThenAFailure,
        /// The fixture for the first two requests and one byte past the
        /// cap for the third: the cap firing on the last of the set.
        TooMuchOnTheLast,
        /// Nothing: being called at all is the failure.
        Nothing,
    }

    /// A fetcher that records every URL it was asked for.
    struct Fetcher<'a> {
        fixture: &'a Fixture,
        serves: Serves,
        log: RefCell<Vec<String>>,
    }

    impl<'a> Fetcher<'a> {
        fn new(fixture: &'a Fixture, serves: Serves) -> Fetcher<'a> {
            Fetcher {
                fixture,
                serves,
                log: RefCell::new(Vec::new()),
            }
        }

        fn fetch(&self, url: &str) -> domain::Result<Vec<u8>> {
            self.log.borrow_mut().push(url.to_string());
            let index = URLS.iter().position(|u| *u == url).expect("known url");
            let round = self.log.borrow().len();
            match self.serves {
                Serves::TheModel => Ok(self.fixture.bytes[index].clone()),
                Serves::Garbage => Ok(b"not a model".to_vec()),
                Serves::GarbageThenTheModel => {
                    if round <= ARTIFACT_FILES.len() {
                        Ok(b"not a model".to_vec())
                    } else {
                        Ok(self.fixture.bytes[index].clone())
                    }
                }
                Serves::TooMuch => Ok(vec![0u8; MAX_ARTIFACT_BYTES + 1]),
                Serves::TheFirstThenAFailure => {
                    if round == 1 {
                        Ok(self.fixture.bytes[index].clone())
                    } else {
                        Err(Error::Io(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            format!("download {url}: the transfer stalled"),
                        )))
                    }
                }
                Serves::TooMuchOnTheLast => {
                    if round < ARTIFACT_FILES.len() {
                        Ok(self.fixture.bytes[index].clone())
                    } else {
                        Ok(vec![0u8; MAX_ARTIFACT_BYTES + 1])
                    }
                }
                Serves::Nothing => {
                    panic!("the fetcher was called for {url}, and should not have been")
                }
            }
        }

        fn calls(&self) -> Vec<String> {
            self.log.borrow().clone()
        }
    }

    fn provision(
        dir: &Path,
        fixture: &Fixture,
        fetcher: &Fetcher<'_>,
    ) -> domain::Result<Model2Vec> {
        Model2Vec::provision(dir, &fixture.digest, &URLS, &|url| fetcher.fetch(url))
    }

    /// `Result::unwrap_err` would need `Model2Vec: Debug`, and a loaded
    /// model has nothing worth printing (a 30 MB matrix and a tokenizer),
    /// so the surface stays as it is and the tests unwrap by hand.
    fn expect_err(result: domain::Result<Model2Vec>) -> Error {
        match result {
            Ok(_) => panic!("expected an error, got a loaded model"),
            Err(e) => e,
        }
    }

    fn present(dir: &Path) -> Vec<&'static str> {
        ARTIFACT_FILES
            .into_iter()
            .filter(|name| dir.join(name).exists())
            .collect()
    }

    /// The temporaries left in `dir`. Every artifact lands through one,
    /// and none of them outlives the call that opened it, so anything
    /// this returns is a leak. A directory that was never created counts
    /// as empty.
    fn temporaries(dir: &Path) -> Vec<String> {
        let Ok(entries) = fs::read_dir(dir) else {
            return Vec::new();
        };
        entries
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(".tmp."))
            .collect()
    }

    #[test]
    fn a_matching_digest_loads_and_nothing_is_fetched() {
        let fixture = Fixture::new();
        let dir = tempfile::tempdir().unwrap();
        fixture.write_into(dir.path());

        let fetcher = Fetcher::new(&fixture, Serves::Nothing);
        let model = provision(dir.path(), &fixture, &fetcher).unwrap();

        assert_eq!(model.model_hash(), fixture.digest);
        assert!(fetcher.calls().is_empty());
    }

    #[test]
    fn a_corrupted_download_is_removed_and_downloaded_again() {
        let fixture = Fixture::new();
        // Empty, so every byte in it came from this call's fetcher and
        // removing it on a mismatch destroys nothing of the caller's.
        let dir = tempfile::tempdir().unwrap();

        let fetcher = Fetcher::new(&fixture, Serves::GarbageThenTheModel);
        let model = provision(dir.path(), &fixture, &fetcher).unwrap();

        // Two rounds of three. All three come back each time, not just
        // the one that drifted: the digest covers the set, so the set is
        // what gets replaced.
        assert_eq!(fetcher.calls(), [URLS, URLS].concat());
        assert_eq!(model.model_hash(), fixture.digest);
    }

    #[test]
    fn a_second_mismatch_is_model_invalid_and_leaves_no_files() {
        let fixture = Fixture::new();
        let dir = tempfile::tempdir().unwrap();

        let fetcher = Fetcher::new(&fixture, Serves::Garbage);
        let err = expect_err(provision(dir.path(), &fixture, &fetcher));

        assert!(
            matches!(err, Error::ModelInvalid(ref m) if m.contains(&fixture.digest)),
            "expected a digest mismatch, got {err:?}"
        );
        // Exactly one retry, so two rounds of three, and nothing left
        // behind for a later call to find.
        assert_eq!(fetcher.calls().len(), 2 * ARTIFACT_FILES.len());
        assert_eq!(present(dir.path()), Vec::<&str>::new());
    }

    #[test]
    fn an_oversized_download_is_input_too_large() {
        let fixture = Fixture::new();
        let dir = tempfile::tempdir().unwrap();

        let fetcher = Fetcher::new(&fixture, Serves::TooMuch);
        let err = expect_err(provision(dir.path(), &fixture, &fetcher));

        match err {
            Error::InputTooLarge {
                limit,
                actual,
                what,
            } => {
                assert_eq!(limit, MAX_ARTIFACT_BYTES);
                assert_eq!(actual, MAX_ARTIFACT_BYTES + 1);
                assert_eq!(what, "embedding_download");
            }
            other => panic!("expected InputTooLarge, got {other:?}"),
        }
        // The cap fires before anything is written.
        assert_eq!(present(dir.path()), Vec::<&str>::new());
    }

    /// A download is all three artifacts or none of them. Leaving the
    /// first one behind would be the worst of both: every later call
    /// refuses a partial set, so one timed-out transfer would wedge the
    /// directory until somebody deleted files by hand.
    #[test]
    fn a_fetch_that_fails_partway_leaves_no_artifacts() {
        let fixture = Fixture::new();
        let dir = tempfile::tempdir().unwrap();

        let fetcher = Fetcher::new(&fixture, Serves::TheFirstThenAFailure);
        let err = expect_err(provision(dir.path(), &fixture, &fetcher));

        // The fetcher's own error, not a digest verdict on bytes that
        // never arrived.
        match err {
            Error::Io(e) => assert_eq!(e.kind(), std::io::ErrorKind::TimedOut),
            other => panic!("expected the fetcher's Io error, got {other:?}"),
        }
        // A transport failure is not retried, so the second URL is the
        // last one asked for, and the first artifact does not survive it.
        assert_eq!(fetcher.calls(), URLS[..2].to_vec());
        assert_eq!(present(dir.path()), Vec::<&str>::new());
        assert_eq!(temporaries(dir.path()), Vec::<String>::new());
    }

    /// The cap firing on the last of the three is the same transaction:
    /// the two that already arrived are temporaries rather than
    /// artifacts, and they go with it.
    #[test]
    fn an_oversized_last_artifact_leaves_no_artifacts() {
        let fixture = Fixture::new();
        let dir = tempfile::tempdir().unwrap();

        let fetcher = Fetcher::new(&fixture, Serves::TooMuchOnTheLast);
        let err = expect_err(provision(dir.path(), &fixture, &fetcher));

        match err {
            Error::InputTooLarge {
                limit,
                actual,
                what,
            } => {
                assert_eq!(limit, MAX_ARTIFACT_BYTES);
                assert_eq!(actual, MAX_ARTIFACT_BYTES + 1);
                assert_eq!(what, "embedding_download");
            }
            other => panic!("expected InputTooLarge, got {other:?}"),
        }
        assert_eq!(fetcher.calls(), URLS.to_vec());
        assert_eq!(present(dir.path()), Vec::<&str>::new());
        assert_eq!(temporaries(dir.path()), Vec::<String>::new());
    }

    /// The names are the artifact format's, not this model's, so a full
    /// set that does not match the pin is somebody else's model. The
    /// provisioner says so and leaves every byte where it found it.
    #[test]
    fn pre_existing_mismatching_files_are_refused_and_untouched() {
        let fixture = Fixture::new();
        let dir = tempfile::tempdir().unwrap();
        fixture.write_into(dir.path());
        let mine: &[u8] = br#"{"normalize": false}"#;
        fs::write(dir.path().join("config.json"), mine).unwrap();

        let fetcher = Fetcher::new(&fixture, Serves::Nothing);
        let err = expect_err(provision(dir.path(), &fixture, &fetcher));

        let dir_name = dir.path().display().to_string();
        assert!(
            matches!(err, Error::ModelInvalid(ref m) if m.contains(&dir_name)),
            "expected the directory to be named, got {err:?}"
        );
        assert_eq!(present(dir.path()).len(), ARTIFACT_FILES.len());
        assert_eq!(fs::read(dir.path().join("config.json")).unwrap(), mine);
        assert_eq!(
            fs::read(dir.path().join("model.safetensors")).unwrap(),
            fixture.bytes[0]
        );
        assert_eq!(
            fs::read(dir.path().join("tokenizer.json")).unwrap(),
            fixture.bytes[1]
        );
    }

    /// A partial set has no provenance: it could be an interrupted
    /// download of ours or the start of somebody else's model. Filling
    /// the gap would mix two artifact sets under one digest, so the
    /// provisioner refuses this the same way and touches nothing.
    #[test]
    fn a_partial_directory_is_refused_and_untouched() {
        let fixture = Fixture::new();
        let dir = tempfile::tempdir().unwrap();
        fixture.write_into(dir.path());
        fs::remove_file(dir.path().join("tokenizer.json")).unwrap();

        let fetcher = Fetcher::new(&fixture, Serves::Nothing);
        let err = expect_err(provision(dir.path(), &fixture, &fetcher));

        let dir_name = dir.path().display().to_string();
        assert!(
            matches!(err, Error::ModelInvalid(ref m) if m.contains(&dir_name)),
            "expected the directory to be named, got {err:?}"
        );
        assert_eq!(
            present(dir.path()),
            vec!["model.safetensors", "config.json"]
        );
        assert_eq!(
            fs::read(dir.path().join("model.safetensors")).unwrap(),
            fixture.bytes[0]
        );
        assert_eq!(
            fs::read(dir.path().join("config.json")).unwrap(),
            fixture.bytes[2]
        );
    }

    /// The temporary each artifact lands through has a predictable name,
    /// which is a place to plant a symlink in a directory somebody else
    /// can write to. `create_new` is `O_EXCL`, so the open fails and the
    /// 30 MB never travels through the link to whatever it points at.
    #[cfg(unix)]
    #[test]
    fn a_planted_temporary_is_not_written_through() {
        let victim_bytes: &[u8] = b"the caller's bytes";
        let fixture = Fixture::new();
        let dir = tempfile::tempdir().unwrap();
        let victim = dir.path().join("victim");
        fs::write(&victim, victim_bytes).unwrap();

        let planted = dir
            .path()
            .join(format!(".tmp.model.safetensors.{}", std::process::id()));
        std::os::unix::fs::symlink(&victim, &planted).unwrap();

        let fetcher = Fetcher::new(&fixture, Serves::TheModel);
        let err = expect_err(provision(dir.path(), &fixture, &fetcher));

        match err {
            Error::Io(e) => assert_eq!(e.kind(), std::io::ErrorKind::AlreadyExists),
            other => panic!("expected Io(AlreadyExists), got {other:?}"),
        }
        assert_eq!(fs::read(&victim).unwrap(), victim_bytes);
        // The plant is left where it was found, like any other file this
        // call did not create.
        assert!(fs::symlink_metadata(&planted).is_ok());
        assert_eq!(present(dir.path()), Vec::<&str>::new());
    }

    /// The pin lives in two places by necessity: this file compiles it
    /// into the loader, and the conformance fixture states it for every
    /// crust's runner. Two copies that nothing compares are two copies
    /// that drift.
    #[test]
    fn the_pinned_digest_matches_the_conformance_fixture() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("spec/tests/semantic/reference-model.json");
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let fixture: serde_json::Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("malformed fixture {}: {e}", path.display()));
        let pinned = fixture["model"]["artifact_hash"]
            .as_str()
            .expect("model.artifact_hash is a string");

        assert_eq!(
            pinned,
            POTION_BASE_8M_SHA256,
            "{} pins a different artifact digest than src/embed/model2vec.rs; \
             refresh both with scripts/fetch-embedding-hash.sh",
            path.display()
        );
    }

    #[test]
    fn a_missing_directory_is_created_and_filled() {
        let fixture = Fixture::new();
        let parent = tempfile::tempdir().unwrap();
        let dir = parent.path().join("models").join("potion-base-8M");

        let fetcher = Fetcher::new(&fixture, Serves::TheModel);
        let model = provision(&dir, &fixture, &fetcher).unwrap();

        assert_eq!(model.model_hash(), fixture.digest);
        assert_eq!(present(&dir).len(), 3);
        // The temporary each artifact landed through is gone.
        assert_eq!(temporaries(&dir), Vec::<String>::new());
    }

    #[test]
    fn from_dir_never_downloads() {
        let fixture = Fixture::new();
        let fetcher = Fetcher::new(&fixture, Serves::TheModel);

        // The fetcher is live: provisioning an empty directory calls it.
        let provisioned = tempfile::tempdir().unwrap();
        provision(provisioned.path(), &fixture, &fetcher).unwrap();
        assert_eq!(fetcher.calls().len(), 3);

        // `from_dir` has no fetcher to call, and an empty directory is
        // where the difference shows: it reports what is missing instead
        // of going to get it.
        let empty = tempfile::tempdir().unwrap();
        let err = expect_err(Model2Vec::from_dir(empty.path()));
        assert!(
            matches!(err, Error::ModelNotFound(ref p) if p.ends_with("model.safetensors")),
            "expected ModelNotFound, got {err:?}"
        );
        assert_eq!(fetcher.calls().len(), 3);
        assert_eq!(present(empty.path()), Vec::<&str>::new());
    }
}
