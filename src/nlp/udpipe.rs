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
        let model = Model::load(path).map_err(|e| Error::ModelInvalid(e.to_string()))?;
        Ok(Self { model })
    }

    /// Load from bytes (e.g. embedded via include_bytes!).
    pub fn from_bytes(data: &[u8]) -> crate::domain::Result<Self> {
        let model =
            Model::load_from_memory(data).map_err(|e| Error::ModelInvalid(e.to_string()))?;
        Ok(Self { model })
    }

    /// Download and load the English model, verifying its SHA-256 against
    /// a pinned constant in the source.
    ///
    /// If the cached file fails verification it is removed and re-downloaded
    /// (once). A subsequent failure returns [`Error::ModelInvalid`] without
    /// loading the file — a mismatched model is treated as untrusted.
    ///
    /// **No TOCTOU window.** The bytes that match the SHA-256 are the same
    /// bytes loaded into the model — there is no second disk read between
    /// verify and load. An attacker with write access to `model_dir` who
    /// swaps the file between verify and a hypothetical second read cannot
    /// affect the loaded model, because no second read happens.
    ///
    /// To refresh the pinned hash when the model version changes, run
    /// `scripts/fetch-model-hash.sh` and paste the output into this file.
    pub fn english(model_dir: impl AsRef<Path>) -> crate::domain::Result<Self> {
        let dir = model_dir.as_ref();
        std::fs::create_dir_all(dir)?;
        let path = dir.join(ENGLISH_MODEL_FILENAME);

        // Fresh download if missing.
        if !path.exists() {
            download_english(dir, &path)?;
        }

        // Try to verify-and-read the cached file. On mismatch, redownload
        // once and try again. On second mismatch, give up.
        if let Some(bytes) = read_and_verify(&path, ENGLISH_MODEL_SIZE, ENGLISH_MODEL_SHA256)? {
            return Self::from_bytes(&bytes);
        }
        std::fs::remove_file(&path)?;
        download_english(dir, &path)?;
        match read_and_verify(&path, ENGLISH_MODEL_SIZE, ENGLISH_MODEL_SHA256)? {
            Some(bytes) => Self::from_bytes(&bytes),
            None => Err(Error::ModelInvalid(format!(
                "SHA-256 mismatch after re-download: {}",
                path.display()
            ))),
        }
    }
}

/// Filename `udpipe_rs::download_model("english-ewt", ...)` writes inside
/// the target directory. Hardcoded by the upstream crate.
const ENGLISH_MODEL_FILENAME: &str = "english-ewt-ud-2.5-191206.udpipe";

/// Run a closure with a temporary subdirectory inside `parent`, removing
/// the subdirectory on scope exit (success or panic). The subdirectory
/// name includes the current process id so concurrent calls in different
/// processes do not collide.
fn with_temp_subdir<F, T>(parent: &Path, f: F) -> crate::domain::Result<T>
where
    F: FnOnce(&Path) -> crate::domain::Result<T>,
{
    struct Cleanup<'a>(&'a Path);
    impl Drop for Cleanup<'_> {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(self.0);
        }
    }

    let tmp_name = format!(".tmp.download.{}", std::process::id());
    let tmp_dir = parent.join(&tmp_name);
    // remove any orphan from a previously-killed process with the same pid
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir)?;
    let _cleanup = Cleanup(&tmp_dir);
    f(&tmp_dir)
}

/// Download the English model into `final_path` atomically.
///
/// Writes to a per-process temporary subdirectory of `dir`, then atomically
/// renames the file to `final_path`. `std::fs::rename` is atomic on the
/// same filesystem (POSIX `rename(2)`; Windows `MoveFileExW` with
/// `MOVEFILE_REPLACE_EXISTING`). Concurrent processes calling
/// `Udpipe::english(same_dir)` cannot corrupt each other's file: each
/// downloads to its own `.tmp.download.<pid>` subdirectory and the
/// rename is the only operation that touches `final_path`.
///
/// If a process is killed mid-download, its temp subdirectory is left on
/// disk. The next call with the same pid removes it before re-downloading;
/// other pids are independent.
fn download_english(dir: &Path, final_path: &Path) -> crate::domain::Result<()> {
    with_temp_subdir(dir, |tmp_dir| {
        let tmp_str = tmp_dir.to_str().ok_or_else(|| {
            Error::ModelInvalid("temp download directory path is not valid UTF-8".into())
        })?;
        udpipe_rs::download_model("english-ewt", tmp_str)
            .map_err(|e| Error::ModelInvalid(e.to_string()))?;
        let tmp_file = tmp_dir.join(ENGLISH_MODEL_FILENAME);
        std::fs::rename(&tmp_file, final_path)?;
        Ok(())
    })
}

/// Read a file and verify its SHA-256 in a single read.
///
/// Returns:
/// - `Ok(Some(bytes))` when size and hash both match — the returned bytes
///   are the *exact* bytes that were hashed; the caller can pass them
///   directly to a from-memory loader without a second disk read.
/// - `Ok(None)` when size mismatches (fast-fail, no read of contents)
///   or hash mismatches.
/// - `Err` when the file cannot be read.
///
/// This shape closes the TOCTOU window the previous `verify_file` had:
/// previously the caller hashed the file then re-read it via
/// `Model::load(path)`, giving an attacker with directory write access
/// a window to swap the file between verify and load.
fn read_and_verify(
    path: &Path,
    expected_size: u64,
    expected_hash: &str,
) -> crate::domain::Result<Option<Vec<u8>>> {
    let meta = std::fs::metadata(path)?;
    if meta.len() != expected_size {
        return Ok(None);
    }
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let got = hex_encode(&hasher.finalize());
    if got.eq_ignore_ascii_case(expected_hash) {
        Ok(Some(bytes))
    } else {
        Ok(None)
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Run a closure that may panic inside the `udpipe-rs` C/C++ boundary,
/// catching any panic and converting it into [`Error::ParseFailed`]. This
/// keeps a process-aborting C-side bug (Taleb #1: SPOF with no panic
/// boundary) from taking down the host. Without this wrapper, a panic
/// inside `Model::parse` aborts the host process; in Python it manifests
/// as interpreter death, in WASM as a trap.
fn catch_parse_panic<F, T>(f: F) -> crate::domain::Result<T>
where
    F: FnOnce() -> crate::domain::Result<T>,
{
    use std::panic::{AssertUnwindSafe, catch_unwind};
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(result) => result,
        Err(payload) => {
            let message = payload
                .downcast_ref::<&'static str>()
                .map(|s| (*s).to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "udpipe panic (no message captured)".to_string());
            Err(Error::ParseFailed(format!("udpipe panicked: {message}")))
        }
    }
}

impl NlpProvider for Udpipe {
    fn parse(&self, text: &str) -> crate::domain::Result<Vec<Sentence>> {
        let words = catch_parse_panic(|| {
            self.model
                .parse(text)
                .map_err(|e| Error::ParseFailed(e.to_string()))
        })?;

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
                        if i + 1 < tokens.len() && !tok.misc.contains("SpaceAfter=No") {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Validates the catch_parse_panic technique against an arbitrary
    /// panicking closure. We can't easily make the real `Model::parse`
    /// panic in a unit test without an injected fault, so this test
    /// covers the wrapper's contract: a panic inside the closure
    /// becomes Err(ParseFailed) instead of aborting the test process.
    #[test]
    fn catch_parse_panic_converts_str_panic_to_parse_failed() {
        let result: crate::domain::Result<()> = catch_parse_panic(|| {
            panic!("simulated udpipe-rs panic");
        });
        match result {
            Err(Error::ParseFailed(msg)) => {
                assert!(
                    msg.contains("udpipe panicked"),
                    "expected wrapper prefix in: {msg}"
                );
                assert!(
                    msg.contains("simulated udpipe-rs panic"),
                    "expected payload in: {msg}"
                );
            }
            other => panic!("expected ParseFailed, got {other:?}"),
        }
    }

    #[test]
    fn catch_parse_panic_converts_string_panic_to_parse_failed() {
        let result: crate::domain::Result<()> =
            catch_parse_panic(|| panic!("{}", "owned string panic"));
        match result {
            Err(Error::ParseFailed(msg)) => {
                assert!(
                    msg.contains("owned string panic"),
                    "expected payload in: {msg}"
                );
            }
            other => panic!("expected ParseFailed, got {other:?}"),
        }
    }

    #[test]
    fn catch_parse_panic_passes_through_non_panic_results() {
        let ok: crate::domain::Result<i32> = catch_parse_panic(|| Ok(42));
        assert_eq!(ok.unwrap(), 42);

        let err: crate::domain::Result<i32> =
            catch_parse_panic(|| Err(Error::ParseFailed("boring failure".into())));
        match err {
            Err(Error::ParseFailed(msg)) => assert_eq!(msg, "boring failure"),
            other => panic!("expected pass-through ParseFailed, got {other:?}"),
        }
    }

    /// SHA-256 of "hello" computed offline. Tied to the literal payload below.
    const HELLO_HASH: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

    #[test]
    fn read_and_verify_returns_bytes_on_match() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hello.bin");
        std::fs::write(&path, b"hello").unwrap();

        let result = read_and_verify(&path, 5, HELLO_HASH).unwrap();
        assert_eq!(result.as_deref(), Some(b"hello".as_slice()));
    }

    #[test]
    fn read_and_verify_returns_none_on_size_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hello.bin");
        std::fs::write(&path, b"hello").unwrap();

        // Wrong expected size — fast-fail before hashing.
        let result = read_and_verify(&path, 6, HELLO_HASH).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn read_and_verify_returns_none_on_hash_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("five.bin");
        std::fs::write(&path, b"world").unwrap(); // size matches "hello" but bytes differ

        let result = read_and_verify(&path, 5, HELLO_HASH).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn with_temp_subdir_creates_and_cleans_up_on_success() {
        let parent = tempfile::tempdir().unwrap();
        let mut captured: Option<std::path::PathBuf> = None;

        let _ = with_temp_subdir(parent.path(), |tmp| {
            assert!(tmp.exists(), "temp subdir should exist inside the closure");
            assert!(tmp.starts_with(parent.path()), "temp subdir should be inside parent");
            captured = Some(tmp.to_path_buf());
            Ok(())
        });

        let tmp_path = captured.unwrap();
        assert!(
            !tmp_path.exists(),
            "temp subdir should be removed after the closure returns"
        );
    }

    #[test]
    fn with_temp_subdir_cleans_up_on_error() {
        let parent = tempfile::tempdir().unwrap();
        let mut captured: Option<std::path::PathBuf> = None;

        let _ = with_temp_subdir(parent.path(), |tmp| {
            captured = Some(tmp.to_path_buf());
            Err::<(), Error>(Error::ModelInvalid("synthetic".into()))
        });

        let tmp_path = captured.unwrap();
        assert!(
            !tmp_path.exists(),
            "temp subdir should be removed even when the closure returns Err"
        );
    }

    #[test]
    fn read_and_verify_returned_bytes_are_what_was_hashed() {
        // The TOCTOU-closing property: callers can use the returned bytes
        // directly. Even if an attacker swaps the file after this call,
        // the in-memory bytes (which are what the loader uses) match the
        // verified hash.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hello.bin");
        std::fs::write(&path, b"hello").unwrap();

        let bytes = read_and_verify(&path, 5, HELLO_HASH).unwrap().unwrap();

        // Simulate the attack: swap the file with different content.
        std::fs::write(&path, b"WORLD").unwrap();

        // The bytes we got are still the original "hello" — the verified ones.
        // A loader using these bytes is unaffected by the on-disk swap.
        assert_eq!(&bytes, b"hello");
    }
}
