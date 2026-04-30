//! UDPipe adapter. Implements NlpProvider using the udpipe-rs crate.
//! This file is the ONLY place that imports udpipe_rs.

use std::collections::HashMap;
use std::path::Path;

use sha2::{Digest, Sha256};
use udpipe_rs::Model;

use crate::domain::Error;
use crate::domain::{Sentence, Token};

use super::NlpProvider;

/// Expected SHA-256 of the English UD-EWT 2.5 (release 191206) UDPipe model.
/// Refresh with `scripts/fetch-model-hash.sh` when updating the model version.
const ENGLISH_MODEL_SHA256: &str =
    "eeeb1e45bcc89c7497b27fd03ba66bfe6af96fb6df2e64027e6c07bfda52c6a2";

/// Expected size in bytes, checked before hashing as a fast-fail guard.
const ENGLISH_MODEL_SIZE: u64 = 451_996;

/// UDPipe adapter. Validated at construction: if the model is invalid,
/// construction fails. After construction, parse calls are trusted.
pub struct Udpipe {
    model: Model,
}

impl std::fmt::Debug for Udpipe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Udpipe").finish_non_exhaustive()
    }
}

impl Udpipe {
    /// Load from a file path. Fails fast if the model is invalid.
    pub fn from_path(path: impl AsRef<Path>) -> crate::domain::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(Error::ModelNotFound(path.to_path_buf()));
        }
        let model = Model::load(path)
            .map_err(|e| Error::ModelInvalid(e.to_string()))?;
        Ok(Self { model })
    }

    /// Load from bytes (e.g. embedded via include_bytes!).
    pub fn from_bytes(data: &[u8]) -> crate::domain::Result<Self> {
        let model = Model::load_from_memory(data)
            .map_err(|e| Error::ModelInvalid(e.to_string()))?;
        Ok(Self { model })
    }

    /// Download and load the English model, verifying its SHA-256 against
    /// the pinned constant [`ENGLISH_MODEL_SHA256`].
    ///
    /// If the cached file fails verification it is removed and re-downloaded
    /// (once). A subsequent failure returns [`Error::ModelInvalid`] without
    /// loading the file — a mismatched model is treated as untrusted.
    ///
    /// To refresh the pinned hash when the model version changes, run
    /// `scripts/fetch-model-hash.sh` and paste the output into this file.
    pub fn english(model_dir: impl AsRef<Path>) -> crate::domain::Result<Self> {
        let dir = model_dir.as_ref();
        std::fs::create_dir_all(dir)?;
        let path = dir.join("english-ewt-ud-2.5-191206.udpipe");

        // Fresh download if missing; re-download once if cached file fails verify.
        if !path.exists() {
            download_english(dir)?;
        }
        if !verify_file(&path, ENGLISH_MODEL_SIZE, ENGLISH_MODEL_SHA256)? {
            std::fs::remove_file(&path)?;
            download_english(dir)?;
            if !verify_file(&path, ENGLISH_MODEL_SIZE, ENGLISH_MODEL_SHA256)? {
                return Err(Error::ModelInvalid(format!(
                    "SHA-256 mismatch after re-download: {}",
                    path.display()
                )));
            }
        }

        Self::from_path(&path)
    }
}

fn download_english(dir: &Path) -> crate::domain::Result<()> {
    let dir_str = dir
        .to_str()
        .ok_or_else(|| Error::ModelInvalid("model directory path is not valid UTF-8".into()))?;
    udpipe_rs::download_model("english-ewt", dir_str)
        .map_err(|e| Error::ModelInvalid(e.to_string()))?;
    Ok(())
}

/// Verify a file matches the expected size and SHA-256. Returns `Ok(true)`
/// on match, `Ok(false)` on mismatch, and `Err` if the file cannot be read.
fn verify_file(path: &Path, expected_size: u64, expected_hash: &str) -> crate::domain::Result<bool> {
    let meta = std::fs::metadata(path)?;
    if meta.len() != expected_size {
        return Ok(false);
    }
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let got = hex_encode(&hasher.finalize());
    Ok(got.eq_ignore_ascii_case(expected_hash))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

impl NlpProvider for Udpipe {
    fn parse(&self, text: &str) -> crate::domain::Result<Vec<Sentence>> {
        let words = self.model.parse(text)
            .map_err(|e| Error::ParseFailed(e.to_string()))?;

        let mut by_sentence: HashMap<i32, Vec<&udpipe_rs::Word>> = HashMap::new();
        for word in &words {
            by_sentence.entry(word.sentence_id).or_default().push(word);
        }

        let mut ids: Vec<i32> = by_sentence.keys().copied().collect();
        ids.sort();

        ids.into_iter()
            .map(|id| {
                let sent_words = &by_sentence[&id];
                let tokens: Vec<Token> = sent_words
                    .iter()
                    .map(|w| {
                        let id = usize::try_from(w.id).map_err(|_| {
                            Error::ParseFailed(format!(
                                "invalid token id {} in sentence {}",
                                w.id, w.sentence_id
                            ))
                        })?;
                        let head = usize::try_from(w.head).map_err(|_| {
                            Error::ParseFailed(format!(
                                "invalid head {} for token {} in sentence {}",
                                w.head, w.id, w.sentence_id
                            ))
                        })?;
                        Ok(Token {
                            id,
                            text: w.form.clone(),
                            lemma: w.lemma.clone(),
                            pos: w.upostag.clone(),
                            xpos: w.xpostag.clone(),
                            feats: w.feats.clone(),
                            dep: w.deprel.clone(),
                            head,
                            deps: String::from("_"),
                            misc: w.misc.clone(),
                            is_punct: w.is_punct(),
                        })
                    })
                    .collect::<crate::domain::Result<Vec<Token>>>()?;

                // Reconstruct original text using SpaceAfter=No from misc field.
                let text = {
                    let mut buf = String::new();
                    for (i, tok) in tokens.iter().enumerate() {
                        buf.push_str(&tok.text);
                        if i + 1 < tokens.len()
                            && !tok.misc.contains("SpaceAfter=No")
                        {
                            buf.push(' ');
                        }
                    }
                    buf
                };

                Ok(Sentence { text, tokens })
            })
            .collect()
    }
}
