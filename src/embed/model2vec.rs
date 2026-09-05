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
//! fixtures in this file's tests; `spec/` conformance fixtures for the
//! pinned reference model arrive with the docs milestone.

use std::fs;
use std::path::Path;

use safetensors::SafeTensors;
use safetensors::tensor::Dtype;
use sha2::{Digest, Sha256};
use tokenizers::Tokenizer;

use crate::domain::{self, Embedding, Error};
use crate::embed::Embedder;

/// The reference implementation's default token cap per text. Applied
/// after unknown-token removal; also drives the pre-tokenization
/// character truncation (`max_tokens * median_token_length` bytes).
const DEFAULT_MAX_TOKENS: usize = 512;

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
    /// No network is touched; the caller supplies the files (ADR-0010).
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
        let model_path = dir.join("model.safetensors");
        let tokenizer_path = dir.join("tokenizer.json");
        let config_path = dir.join("config.json");
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
