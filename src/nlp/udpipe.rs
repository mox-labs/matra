//! UDPipe adapter. Implements NlpProvider using the udpipe-rs crate.
//! This file is the ONLY place that imports udpipe_rs.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};
use udpipe_rs::Model;

use crate::config::Config;
use crate::domain::Error;
use crate::domain::{ProvisionNotice, Sentence, Token};

use super::NlpProvider;

/// Expected SHA-256 of the English UD-EWT 2.5 (release 191206) UDPipe model.
/// Refresh with `scripts/fetch-model-hash.sh` when updating the model version.
const ENGLISH_MODEL_SHA256: &str =
    "784bd0fa85e3d831fd02a55290d0acfd05c953159dc38cc33d52e1b28add9957";

/// Expected size in bytes, checked before hashing as a fast-fail guard.
const ENGLISH_MODEL_SIZE: u64 = 16_309_608;

/// Direct download URL for the pinned model. LINDAT migrated the bitstream
/// endpoint from `/repository/xmlui/bitstream/...` (now a 200 HTML preview)
/// to `/repository/server/api/core/bitstreams/...` (the actual binary).
/// `udpipe_rs::download_model` still uses the old pattern, so we call
/// `download_model_from_url` directly with the working URL.
const ENGLISH_MODEL_URL: &str = "https://lindat.mff.cuni.cz/repository/server/api/core/bitstreams/handle/11234/1-3131/english-ewt-ud-2.5-191206.udpipe?sequence=17&isAllowed=y";

/// Ceiling on the downloaded model. The pinned artifact is 16.3 MB, so
/// this is four times the real thing: headroom for a later model version
/// and small enough that a redirect to something enormous costs a
/// bounded read rather than the machine's memory. Past it,
/// [`Error::InputTooLarge`] with `what` set to `"udpipe_download"`. The
/// same number the embedding adapter uses, because it is the same
/// question.
const MAX_MODEL_BYTES: usize = 64 * 1024 * 1024;

/// Ceiling on the fetch, DNS lookup through the last byte of the body.
/// The pinned model is 16.3 MB, which needs this whole budget only on a
/// link under 0.5 Mbit/s, so the margin is generous for a slow
/// connection and still finite: a stalled or black-holed transfer fails
/// instead of holding the caller for as long as the socket stays open.
/// Before this existed, 90 seconds against an unreachable host produced
/// zero bytes with the process still running, unbounded.
const FETCH_TIMEOUT: Duration = Duration::from_secs(300);

/// Ceiling on establishing the connection, socket and TLS handshake
/// included. Separate from [`FETCH_TIMEOUT`] so an unreachable host
/// fails in seconds rather than consuming the whole transfer budget.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Age past which a temporary download directory cannot belong to a
/// live provisioning call, so it is a leftover and this call reclaims
/// it.
///
/// Twice [`FETCH_TIMEOUT`]. A fetch cannot outlive that budget, and the
/// directory is created after the bytes are already in memory, so its
/// modification time is within milliseconds of the write that follows.
/// Anything older than ten minutes is therefore an orphan from a killed
/// process, not a peer. The margin is what protects a concurrent cold
/// start: three processes racing on one empty model directory each see
/// the others' directories as minutes-fresh and leave them alone.
const STALE_TEMP_AGE: Duration = Duration::from_secs(600);

/// Prefix of the temporary directory a download lands in. Shared by the
/// creation and the reclaim, so the sweep cannot look for a name the
/// writer does not use.
const TEMP_DIR_PREFIX: &str = ".tmp.download.";

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
    /// Silent. [`Udpipe::english_with_notice`] is the same call with a
    /// say-so before the download, which is what the command line uses.
    ///
    /// The bytes are fetched into memory, verified there, and only then
    /// written: nothing that failed the digest ever reaches the model
    /// directory, and a run interrupted during the transfer leaves
    /// nothing behind at all. A cached file that fails verification is
    /// refetched once and replaced only when the new bytes verify, so a
    /// refetch that cannot reach the network leaves the cached file
    /// where it was rather than leaving nothing; a second mismatch returns
    /// [`Error::ModelInvalid`] without loading anything, because a
    /// mismatched model is untrusted.
    ///
    /// **No TOCTOU window.** The bytes that match the SHA-256 are the
    /// same bytes loaded into the model. There is no second disk read
    /// between verify and load, so an attacker with write access to
    /// `model_dir` who swaps the file after verification cannot affect
    /// the loaded model.
    ///
    /// To refresh the pinned hash when the model version changes, run
    /// `scripts/fetch-model-hash.sh` and paste the output into this file.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the model directory cannot be created, if the
    /// download fails at the transport or answers with a non-2xx status
    /// (the message names the URL, and the `io::ErrorKind` separates a
    /// timeout from an unreachable host), or if the verified bytes
    /// cannot be written. [`Error::InputTooLarge`] with `what` set to
    /// `"udpipe_download"` if the response exceeds
    /// [`MAX_MODEL_BYTES`](self). [`Error::ModelInvalid`] if the bytes
    /// still fail the digest after one refetch, or if the verified bytes
    /// do not load.
    pub fn english(model_dir: impl AsRef<Path>) -> crate::domain::Result<Self> {
        Self::english_with_notice(model_dir, |_| {})
    }

    /// [`Udpipe::english`], with `notice` called once before each fetch
    /// and not at all when the model is already on disk.
    ///
    /// The first run downloads 16 MB from a university server in Prague.
    /// Measured cold starts ran from 3 to 35 seconds with nothing on
    /// screen, which reads as a hung process. The library renders
    /// nothing itself, so this is the seam a caller renders through.
    ///
    /// ```no_run
    /// use matra::nlp::udpipe::Udpipe;
    ///
    /// let cfg = matra::config::Config::resolve()?;
    /// let nlp = Udpipe::english_with_notice(cfg.model_dir(), |n| {
    ///     eprintln!("fetching {} ({} bytes)", n.artifact, n.bytes);
    /// })?;
    /// # Ok::<(), matra::domain::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Whatever [`Udpipe::english`] returns.
    pub fn english_with_notice(
        model_dir: impl AsRef<Path>,
        mut notice: impl FnMut(&ProvisionNotice),
    ) -> crate::domain::Result<Self> {
        let bytes = provision(
            model_dir.as_ref(),
            ENGLISH_MODEL_FILENAME,
            ENGLISH_MODEL_SIZE,
            ENGLISH_MODEL_SHA256,
            ENGLISH_MODEL_URL,
            &mut notice,
            &fetch_capped,
        )?;
        Self::from_bytes(&bytes)
    }

    /// [`Udpipe::english`] over the model directory a [`Config`] resolved.
    ///
    /// Additive: the explicit-directory constructors are unchanged, and
    /// this one exists so a caller who has no opinion about where models
    /// live does not have to invent one.
    ///
    /// ```no_run
    /// use matra::nlp::udpipe::Udpipe;
    ///
    /// let cfg = matra::config::Config::resolve()?;
    /// let nlp = Udpipe::from_config(&cfg)?;
    /// # Ok::<(), matra::domain::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Whatever [`Udpipe::english`] returns.
    pub fn from_config(cfg: &Config) -> crate::domain::Result<Self> {
        Self::english(cfg.model_dir())
    }

    /// [`Udpipe::from_config`] with the say-so
    /// [`Udpipe::english_with_notice`] takes.
    ///
    /// ```no_run
    /// use matra::nlp::udpipe::Udpipe;
    ///
    /// let cfg = matra::config::Config::resolve()?;
    /// let nlp = Udpipe::from_config_with_notice(&cfg, |n| {
    ///     eprintln!("fetching {} ({} bytes)", n.artifact, n.bytes);
    /// })?;
    /// # Ok::<(), matra::domain::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Whatever [`Udpipe::english`] returns.
    pub fn from_config_with_notice(
        cfg: &Config,
        notice: impl FnMut(&ProvisionNotice),
    ) -> crate::domain::Result<Self> {
        Self::english_with_notice(cfg.model_dir(), notice)
    }
}

/// Filename `udpipe_rs::download_model("english-ewt", ...)` writes inside
/// the target directory. Hardcoded by the upstream crate.
const ENGLISH_MODEL_FILENAME: &str = "english-ewt-ud-2.5-191206.udpipe";

/// Obtain the pinned artifact's verified bytes, downloading it if the
/// model directory does not already hold it.
///
/// The pin, the URL and the fetcher are arguments rather than constants
/// read from scope, on the precedent the embedding adapter set: a test
/// can then pin a fixture of its own, and what is under test is what the
/// digest decides rather than what the fetcher returns.
///
/// The order is deliberate. Bytes are fetched into memory, checked
/// against the pinned size and digest there, and only then written, so
/// nothing that failed verification ever reaches the model directory and
/// an interrupted transfer leaves nothing behind. The bytes returned are
/// the bytes that satisfied the digest, which is what closes the TOCTOU
/// window: the loader never reads the disk again.
///
/// A cached file that is not the pinned model is removed, but only once
/// a replacement is in hand. Removing it first cost the user their
/// working file whenever the refetch then failed, which offline plus a
/// corrupt cache made a certainty, and bought nothing: [`install`] lands
/// through a rename and `fs::rename` replaces an existing destination,
/// so the write never needed the name free.
fn provision(
    dir: &Path,
    filename: &str,
    expected_size: u64,
    expected_hash: &str,
    url: &str,
    notice: &mut dyn FnMut(&ProvisionNotice),
    fetch: &dyn Fn(&str) -> crate::domain::Result<Vec<u8>>,
) -> crate::domain::Result<Vec<u8>> {
    create_model_dir(dir)?;
    let path = dir.join(filename);

    if path.exists() {
        // A cached file that is not the pinned model is not removed here.
        // `install` lands through a rename, which replaces the destination,
        // so an unlink first would buy nothing and cost three things: it
        // would falsify the guarantee below that the rename is the only
        // operation touching the final path, it would let a failed remove
        // abort a run whose fetch had already succeeded, and it would open
        // a window in which the file does not exist, so a concurrent
        // process would see no cache and pay a redundant 16 MB fetch.
        // Leaving it also means a refetch that cannot reach the network
        // leaves the user their old file rather than nothing.
        if let Some(bytes) = read_and_verify(&path, expected_size, expected_hash)? {
            return Ok(bytes);
        }
    }

    let bytes = fetch_verified(
        dir,
        filename,
        expected_size,
        expected_hash,
        url,
        notice,
        fetch,
    )?;
    install(dir, filename, &bytes)?;
    Ok(bytes)
}

/// Fetch the artifact and return it only if it is the pinned one.
///
/// Two attempts. A first response that fails the digest is most often a
/// truncated or intercepted transfer rather than a changed upstream, and
/// one retry costs a bounded read; a second failure is not a transient,
/// so it returns [`Error::ModelInvalid`] and nothing is written.
///
/// The size cap is enforced here rather than inside the fetcher, at the
/// one place every response passes through: the fetcher is replaceable
/// and the bound is not.
fn fetch_verified(
    dir: &Path,
    filename: &str,
    expected_size: u64,
    expected_hash: &str,
    url: &str,
    notice: &mut dyn FnMut(&ProvisionNotice),
    fetch: &dyn Fn(&str) -> crate::domain::Result<Vec<u8>>,
) -> crate::domain::Result<Vec<u8>> {
    for _ in 0..2 {
        notice(&ProvisionNotice {
            artifact: filename.to_string(),
            bytes: expected_size,
            destination: dir.to_path_buf(),
        });
        let bytes = fetch(url)?;
        if bytes.len() > MAX_MODEL_BYTES {
            return Err(Error::InputTooLarge {
                limit: MAX_MODEL_BYTES,
                actual: bytes.len(),
                what: "udpipe_download",
            });
        }
        if verified(&bytes, expected_size, expected_hash) {
            return Ok(bytes);
        }
    }
    Err(Error::ModelInvalid(format!(
        "SHA-256 mismatch after re-download from {url}"
    )))
}

/// Write verified bytes into the model directory atomically.
///
/// The bytes land in a temporary subdirectory of `dir` and arrive at
/// their final name under one rename. `std::fs::rename` is atomic on one
/// filesystem (POSIX `rename(2)`; Windows `MoveFileExW` with
/// `MOVEFILE_REPLACE_EXISTING`), so no reader sees a partial model and
/// two processes provisioning into the same directory cannot interleave:
/// each writes inside its own subdirectory and the rename is the only
/// operation that touches the final path.
///
/// The temporary is opened with `create_new`, which is `O_EXCL`: a path
/// already there, symlink or not, fails the open rather than being
/// written through.
fn install(dir: &Path, filename: &str, bytes: &[u8]) -> crate::domain::Result<()> {
    use std::io::Write;

    with_temp_subdir(dir, |tmp_dir| {
        let tmp_file = tmp_dir.join(filename);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_file)
            .map_err(|e| io_at("create the temporary file", &tmp_file, &e))?;
        file.write_all(bytes)
            .map_err(|e| io_at("write the model to", &tmp_file, &e))?;
        drop(file);
        let final_path = dir.join(filename);
        std::fs::rename(&tmp_file, &final_path)
            .map_err(|e| io_at("move the model into place at", &final_path, &e))
    })
}

/// Fetch one URL into memory, reading at most one byte past
/// [`MAX_MODEL_BYTES`].
///
/// Stopping one byte over is what lets the caller tell "at the cap" from
/// "over it" while keeping the read bounded: an endless or misdirected
/// response costs 64 MiB and a rejection, not the machine's memory.
///
/// `ureq` treats a non-2xx status as an error by default, so an HTML
/// error page never reaches the digest.
///
/// The call is bounded in time as well as in size, [`FETCH_TIMEOUT`] end
/// to end and [`CONNECT_TIMEOUT`] on the connection, because `ureq` sets
/// no timeout of its own: every timeout in its default configuration is
/// `None`, and a socket that accepts and then says nothing would
/// otherwise block the caller for as long as it stays open. That is what
/// the fetch through `udpipe_rs::download_model_from_url` did before
/// this function existed.
fn fetch_capped(url: &str) -> crate::domain::Result<Vec<u8>> {
    use std::io::Read;

    let response = download_agent()
        .get(url)
        .call()
        .map_err(|e| transport_failure(url, &e))?;
    let mut bytes = Vec::new();
    response
        .into_body()
        .into_reader()
        .take(MAX_MODEL_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| body_failure(url, e))?;
    Ok(bytes)
}

/// The client [`fetch_capped`] downloads through.
///
/// A function rather than an inline builder so a test can assert the
/// configuration without a network. `https_only` is the part worth
/// asserting: `ureq` follows up to ten redirects by default, so without
/// it an `https` URL that redirects to `http` is fetched in cleartext.
/// The pinned digest means that cannot change which bytes load, so this
/// is confidentiality rather than integrity, but a redirect is not the
/// user's decision to make and the pinned URLs are all `https`.
fn download_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(FETCH_TIMEOUT))
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .https_only(true)
        .build()
        .into()
}

/// Map a failure part-way through reading the response body.
///
/// The body reader hands back an `io::Error`, and `ureq` builds that one
/// with `Error::into_io`, which returns the inner error only for
/// `Error::Io` and wraps everything else, `Timeout` included, in
/// `io::Error::other`. Reading `kind()` straight off it therefore gives
/// `Other`, so a fetch that ran past [`FETCH_TIMEOUT`] mid-transfer
/// reported `Other` where `book/src/reference/errors.md` and ADR-0015
/// both promise `TimedOut`. `From<io::Error> for ureq::Error` unwraps
/// the wrapped error again, which recovers the kind and also gives a
/// mid-stream certificate rejection the sentence [`download_message`]
/// writes. An `io::Error` that was never a `ureq::Error` comes back as
/// `Error::Io` and keeps its own kind.
fn body_failure(url: &str, error: std::io::Error) -> Error {
    transport_failure(url, &ureq::Error::from(error))
}

/// Map a `ureq` failure to [`Error::Io`].
///
/// A download that never arrives is a transport failure, not an invalid
/// model: [`Error::ModelInvalid`] is reserved for bytes that did arrive
/// and then failed the digest or the loader. A non-2xx status belongs on
/// this side too, because what came back was a server's answer about the
/// request rather than a model. The `io::ErrorKind` is preserved where
/// `ureq` knows it, so a caller can tell a timeout from an unreachable
/// host without reading the message. ADR-0015 records the classification
/// and what it replaced.
fn transport_failure(url: &str, error: &ureq::Error) -> Error {
    use std::io::ErrorKind;

    let kind = match error {
        ureq::Error::Io(e) => e.kind(),
        ureq::Error::Timeout(_) => ErrorKind::TimedOut,
        ureq::Error::ConnectionFailed | ureq::Error::HostNotFound => ErrorKind::NotConnected,
        _ => ErrorKind::Other,
    };
    Error::Io(std::io::Error::new(kind, download_message(url, error)))
}

/// What the user reads when a download fails.
///
/// A rejected certificate gets a sentence instead of a `Debug` rendering
/// of a `rustls` enum. `invalid peer certificate: Other(OtherError(
/// CaUsedAsEndEntity))` is what someone behind a TLS-intercepting
/// corporate proxy saw, and it says nothing about why matra cannot be
/// made to trust their proxy: matra verifies against root certificates
/// compiled into the binary and never reads the system trust store,
/// which is why it needs no `ca-certificates` package and also why
/// adding one to the system store changes nothing. The raw failure is
/// kept at the end, because a bug report needs it.
fn download_message(url: &str, error: &ureq::Error) -> String {
    let detail = error.to_string();
    if !is_certificate_rejection(&detail, error) {
        return format!("download {url}: {detail}");
    }
    format!(
        "download {url}: the TLS certificate offered for {} was rejected. matra verifies \
         TLS against root certificates compiled into it and never reads the system trust \
         store, so a proxy that re-signs TLS cannot be trusted by installing its CA. \
         Fetch {} by hand and put it in the model directory instead. Underlying failure: {detail}",
        host_of(url),
        ENGLISH_MODEL_FILENAME,
    )
}

/// Whether a transport failure is the peer's certificate being refused.
///
/// `ureq` surfaces a `rustls` handshake failure as `Error::Io` wrapping
/// an `io::Error` whose message is the `rustls` error, so the variant
/// alone does not say. The rendered text does, and `Error::Tls` is
/// checked as well for a bespoke transport that reports it directly.
fn is_certificate_rejection(detail: &str, error: &ureq::Error) -> bool {
    if matches!(error, ureq::Error::Tls(_)) {
        return true;
    }
    let lowered = detail.to_ascii_lowercase();
    lowered.contains("certificate") || lowered.contains("rustls")
}

/// The host in an absolute URL, or the whole URL when it has no
/// recognisable authority. Enough for a message; not a URL parser.
fn host_of(url: &str) -> &str {
    let after_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    let authority = after_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(after_scheme);
    // Strip userinfo and any port.
    let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    host.split(':').next().unwrap_or(host)
}

/// A filesystem failure that names the operation and the path.
///
/// `io error: Permission denied (os error 13)` was the whole message a
/// user got when the model directory could not be created, with the
/// directory sitting in a variable one line away.
fn io_at(operation: &str, path: &Path, error: &std::io::Error) -> Error {
    Error::Io(std::io::Error::new(
        error.kind(),
        format!("cannot {operation} {}: {error}", path.display()),
    ))
}

fn create_model_dir(dir: &Path) -> crate::domain::Result<()> {
    std::fs::create_dir_all(dir).map_err(|e| io_at("create the model directory", dir, &e))
}

/// Whether these bytes are the pinned artifact: the size first, as a
/// fast-fail, then the digest.
fn verified(bytes: &[u8], expected_size: u64, expected_hash: &str) -> bool {
    if bytes.len() as u64 != expected_size {
        return false;
    }
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex_encode(&hasher.finalize()).eq_ignore_ascii_case(expected_hash)
}

/// Run a closure with a temporary subdirectory inside `parent`, removing
/// the subdirectory on scope exit (success or panic). The subdirectory
/// name is unique per call ([`temp_stamp`]), so concurrent calls cannot
/// collide even when they share a process id.
///
/// `Drop` does not run on `SIGINT`, so cleanup on scope exit cannot be
/// the whole answer: a run killed between the create and the rename
/// leaves the directory behind. Two things bound that. The bytes are
/// already in memory by the time this is called, so the window is the
/// length of one write rather than the length of a download, and every
/// leftover older than [`STALE_TEMP_AGE`] is reclaimed here before a new
/// one is made. The previous reclaim matched only the current process's
/// own pid, which on a real machine never recurs, so an orphan was
/// permanent.
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

    reclaim_stale_temp_dirs(parent, SystemTime::now());

    let tmp_dir = parent.join(format!("{TEMP_DIR_PREFIX}{}", temp_stamp()));
    // `create_dir`, not `create_dir_all`: this name is this call's alone,
    // so anything already sitting under it is a surprise to fail on
    // rather than a directory to write into. Nothing is removed by name
    // here either, because a name that looks like this call's may be a
    // peer's: age is the only thing that says a directory is a leftover.
    std::fs::create_dir(&tmp_dir)
        .map_err(|e| io_at("create the temporary directory", &tmp_dir, &e))?;
    let _cleanup = Cleanup(&tmp_dir);
    f(&tmp_dir)
}

/// A name fragment no call in flight will pick twice: the process id,
/// the wall clock in nanoseconds, and a counter for two calls in one
/// process that read the same nanosecond.
///
/// The process id alone was the previous answer and it is not unique. In
/// a container every process is pid 1, so two cold starts sharing a
/// bind-mounted model directory choose the same name, and the removal
/// that used to precede the create would delete a live peer's download.
/// Uniqueness here is what leaves age as the only rule that reclaims
/// anything.
fn temp_stamp() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    format!(
        "{}.{nanos:x}.{:x}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

/// Remove every temporary download directory in `parent` that is too old
/// to belong to a running provisioning call.
///
/// `now` is a parameter so the rule is testable without touching
/// modification times on disk. A directory whose modification time is in
/// the future (a clock that moved, a copied tree) is left alone: an
/// unexplained timestamp is a reason not to delete, not a reason to.
///
/// Failures are ignored throughout. Reclaiming a leftover is tidying,
/// and a directory that cannot be listed or removed must not turn a
/// working download into an error.
fn reclaim_stale_temp_dirs(parent: &Path, now: SystemTime) {
    let Ok(entries) = std::fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !name.starts_with(TEMP_DIR_PREFIX) {
            continue;
        }
        let stale = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|modified| now.duration_since(modified).ok())
            .is_some_and(|age| age >= STALE_TEMP_AGE);
        if stale {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
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
    let meta = std::fs::metadata(path).map_err(|e| io_at("read", path, &e))?;
    if meta.len() != expected_size {
        return Ok(None);
    }
    let bytes = std::fs::read(path).map_err(|e| io_at("read", path, &e))?;
    if verified(&bytes, expected_size, expected_hash) {
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

                Ok(Sentence::new(text, tokens))
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
            assert!(
                tmp.starts_with(parent.path()),
                "temp subdir should be inside parent"
            );
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

    // -----------------------------------------------------------------
    // Provisioning
    //
    // Every test below runs offline against an injected fetcher and a
    // fixture pin, which is why `provision` takes the pin, the URL and
    // the fetcher as arguments. What is under test is what the digest
    // decides, not what the fetcher returns.
    // -----------------------------------------------------------------

    const FIXTURE: &[u8] = b"hello";
    const FIXTURE_NAME: &str = "fixture.udpipe";
    const FIXTURE_URL: &str = "https://models.example/fixture.udpipe";

    /// Records what the fetcher was asked for and what the notice said.
    struct Recorder {
        notices: Vec<ProvisionNotice>,
        fetches: usize,
    }

    fn provision_fixture(
        dir: &Path,
        recorder: &mut Recorder,
        body: &dyn Fn() -> crate::domain::Result<Vec<u8>>,
    ) -> crate::domain::Result<Vec<u8>> {
        let fetches = std::cell::Cell::new(0usize);
        let fetch = |_: &str| {
            fetches.set(fetches.get() + 1);
            body()
        };
        let mut notice = |n: &ProvisionNotice| recorder.notices.push(n.clone());
        let result = provision(
            dir,
            FIXTURE_NAME,
            FIXTURE.len() as u64,
            HELLO_HASH,
            FIXTURE_URL,
            &mut notice,
            &fetch,
        );
        recorder.fetches += fetches.get();
        result
    }

    fn recorder() -> Recorder {
        Recorder {
            notices: Vec::new(),
            fetches: 0,
        }
    }

    /// Regression (report H1): a cold run says what it is fetching, how
    /// big it is, and where it is going, before the transfer starts. The
    /// first run used to write zero bytes to either stream for 3 to 35
    /// seconds.
    #[test]
    fn a_cold_run_announces_the_download_once() {
        let dir = tempfile::tempdir().unwrap();
        let mut rec = recorder();
        let bytes = provision_fixture(dir.path(), &mut rec, &|| Ok(FIXTURE.to_vec())).unwrap();

        assert_eq!(bytes, FIXTURE);
        assert_eq!(rec.notices.len(), 1, "one notice per fetch, one fetch");
        assert_eq!(rec.notices[0].artifact, FIXTURE_NAME);
        assert_eq!(rec.notices[0].bytes, FIXTURE.len() as u64);
        assert_eq!(rec.notices[0].destination, dir.path());
        assert_eq!(
            std::fs::read(dir.path().join(FIXTURE_NAME)).unwrap(),
            FIXTURE
        );
    }

    /// The other half of H1: a warm run is silent and touches nothing.
    /// A notice on every run would be noise, and noise gets filtered.
    #[test]
    fn a_warm_run_announces_nothing_and_fetches_nothing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(FIXTURE_NAME), FIXTURE).unwrap();

        let mut rec = recorder();
        let bytes = provision_fixture(dir.path(), &mut rec, &|| {
            panic!("a cached, verified model must not be refetched")
        })
        .unwrap();

        assert_eq!(bytes, FIXTURE);
        assert!(rec.notices.is_empty(), "nothing to announce");
        assert_eq!(rec.fetches, 0);
    }

    /// Bytes that fail the digest are refetched once and never written.
    /// Verification happens in memory, so an untrusted response does not
    /// reach the model directory even briefly.
    #[test]
    fn bytes_that_fail_the_digest_are_refetched_once_then_refused() {
        let dir = tempfile::tempdir().unwrap();
        let mut rec = recorder();
        let err = provision_fixture(dir.path(), &mut rec, &|| Ok(b"WORLD".to_vec()))
            .expect_err("wrong bytes are refused");

        match err {
            Error::ModelInvalid(msg) => {
                assert!(msg.contains("SHA-256 mismatch after re-download"), "{msg}");
                assert!(
                    msg.contains(FIXTURE_URL),
                    "the message names the URL: {msg}"
                );
            }
            other => panic!("expected ModelInvalid, got {other:?}"),
        }
        assert_eq!(rec.fetches, 2, "one retry, not more");
        assert!(
            !dir.path().join(FIXTURE_NAME).exists(),
            "nothing that failed the digest is written"
        );
        assert!(leftovers(dir.path()).is_empty(), "no temporary directory");
    }

    /// A good response after a bad one lands. The retry exists for a
    /// truncated transfer, so it has to be able to succeed.
    #[test]
    fn a_retry_that_succeeds_lands_the_model() {
        let dir = tempfile::tempdir().unwrap();
        let attempts = std::cell::Cell::new(0usize);
        let mut rec = recorder();
        let bytes = provision_fixture(dir.path(), &mut rec, &|| {
            attempts.set(attempts.get() + 1);
            if attempts.get() == 1 {
                Ok(b"WORLD".to_vec())
            } else {
                Ok(FIXTURE.to_vec())
            }
        })
        .unwrap();

        assert_eq!(bytes, FIXTURE);
        assert_eq!(rec.notices.len(), 2, "a notice per fetch");
        assert_eq!(
            std::fs::read(dir.path().join(FIXTURE_NAME)).unwrap(),
            FIXTURE
        );
    }

    /// A cached file that is not the pinned model is replaced, which is
    /// the self-heal a truncated or corrupted cache needs.
    #[test]
    fn a_corrupt_cache_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(FIXTURE_NAME), b"junk").unwrap();

        let mut rec = recorder();
        let bytes = provision_fixture(dir.path(), &mut rec, &|| Ok(FIXTURE.to_vec())).unwrap();

        assert_eq!(bytes, FIXTURE);
        assert_eq!(rec.fetches, 1);
        assert_eq!(
            std::fs::read(dir.path().join(FIXTURE_NAME)).unwrap(),
            FIXTURE
        );
    }

    /// Regression (review of #77, M1): the replacement is fetched before
    /// the cached file is removed. The removal used to run first, so a
    /// user who was offline with a corrupt cache lost the file they had
    /// and got nothing back, and the removal bought nothing: the install
    /// lands through a rename, which replaces an existing destination.
    #[test]
    fn a_corrupt_cache_outlives_a_failed_refetch() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FIXTURE_NAME);
        std::fs::write(&path, b"junk").unwrap();

        let mut rec = recorder();
        let err = provision_fixture(dir.path(), &mut rec, &|| {
            Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "download https://models.example/fixture.udpipe: host not found",
            )))
        })
        .unwrap_err();

        assert!(matches!(err, Error::Io(_)), "got {err:?}");
        assert_eq!(std::fs::read(&path).unwrap(), b"junk");
    }

    /// Regression (report H4): a transport failure travels as the error
    /// the fetcher produced. It is not relabelled as an invalid model,
    /// because no model arrived.
    #[test]
    fn a_transport_failure_is_not_an_invalid_model() {
        let dir = tempfile::tempdir().unwrap();
        let mut rec = recorder();
        let err = provision_fixture(dir.path(), &mut rec, &|| {
            Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "download https://models.example/fixture.udpipe: host not found",
            )))
        })
        .expect_err("an unreachable host fails");

        assert_eq!(err.kind(), "io", "{err}");
        assert!(err.to_string().contains("models.example"), "{err}");
        assert!(leftovers(dir.path()).is_empty());
    }

    /// A response past the cap is rejected by size before anything is
    /// hashed or written, and it names the gate so a consumer can route
    /// it. The cap lives at the one place every response passes through.
    #[test]
    fn a_response_over_the_cap_is_refused_by_size() {
        let dir = tempfile::tempdir().unwrap();
        let mut rec = recorder();
        let err = provision_fixture(dir.path(), &mut rec, &|| Ok(vec![0u8; MAX_MODEL_BYTES + 1]))
            .expect_err("over the cap");

        match err {
            Error::InputTooLarge {
                limit,
                actual,
                what,
            } => {
                assert_eq!(limit, MAX_MODEL_BYTES);
                assert_eq!(actual, MAX_MODEL_BYTES + 1);
                assert_eq!(what, "udpipe_download");
            }
            other => panic!("expected InputTooLarge, got {other:?}"),
        }
        assert_eq!(rec.fetches, 1, "the cap fails the call, it does not retry");
    }

    /// Regression (report H6): a filesystem failure names the operation
    /// and the path. `io error: Permission denied (os error 13)` was the
    /// whole message, with the directory one line away in a variable.
    #[test]
    fn a_filesystem_failure_names_the_operation_and_the_path() {
        // A file where a directory has to be: the create fails, and it
        // fails before anything reaches the network.
        let parent = tempfile::tempdir().unwrap();
        let blocked = parent.path().join("not-a-directory");
        std::fs::write(&blocked, b"x").unwrap();
        let dir = blocked.join("models");

        let mut rec = recorder();
        let err = provision_fixture(&dir, &mut rec, &|| {
            panic!("the directory must fail before any fetch")
        })
        .expect_err("cannot create the model directory");

        assert_eq!(err.kind(), "io");
        let message = err.to_string();
        assert!(
            message.contains("create the model directory"),
            "names the operation: {message}"
        );
        assert!(
            message.contains(&dir.display().to_string()),
            "names the path: {message}"
        );
    }

    /// Regression (report H3): a temporary directory left by a killed
    /// process is reclaimed. The previous sweep matched only the current
    /// process's own pid, which on a real machine never recurs, so a
    /// 15.5 MB orphan was permanent.
    #[test]
    fn an_aged_orphan_temp_directory_is_reclaimed() {
        let dir = tempfile::tempdir().unwrap();
        let orphan = dir.path().join(format!("{TEMP_DIR_PREFIX}424242"));
        std::fs::create_dir_all(&orphan).unwrap();
        std::fs::write(orphan.join("partial.udpipe"), b"half a model").unwrap();

        // The clock is a parameter so the rule is exercised without
        // touching modification times on disk.
        reclaim_stale_temp_dirs(
            dir.path(),
            SystemTime::now() + STALE_TEMP_AGE + Duration::from_secs(1),
        );

        assert!(!orphan.exists(), "an aged leftover is reclaimed");
    }

    /// The constraint on that sweep: a concurrent cold start's directory
    /// is minutes fresh, and reclaiming it would delete another live
    /// process's download. Three racing processes on one empty model
    /// directory must still produce one correct file and no residue.
    #[test]
    fn a_live_concurrent_temp_directory_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let peer = dir.path().join(format!("{TEMP_DIR_PREFIX}424242"));
        std::fs::create_dir_all(&peer).unwrap();
        std::fs::write(peer.join("partial.udpipe"), b"in flight").unwrap();

        reclaim_stale_temp_dirs(dir.path(), SystemTime::now());
        assert!(peer.exists(), "a fresh peer is not this call's to remove");

        // And the sweep runs on the real path too, without touching it.
        let mut rec = recorder();
        provision_fixture(dir.path(), &mut rec, &|| Ok(FIXTURE.to_vec())).unwrap();
        assert!(peer.exists(), "a download does not evict a live peer");
    }

    /// A directory whose modification time is in the future is not
    /// reclaimed. An unexplained timestamp is a reason not to delete.
    #[test]
    fn a_temp_directory_from_the_future_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let odd = dir.path().join(format!("{TEMP_DIR_PREFIX}7"));
        std::fs::create_dir_all(&odd).unwrap();

        reclaim_stale_temp_dirs(dir.path(), SystemTime::now() - Duration::from_secs(3600));
        assert!(odd.exists());
    }

    /// The sweep only ever looks at its own names. Anything else in the
    /// model directory, including a hand-placed model, is untouched.
    #[test]
    fn the_sweep_only_matches_its_own_temporary_names() {
        let dir = tempfile::tempdir().unwrap();
        let model = dir.path().join("english-ewt-ud-2.5-191206.udpipe");
        std::fs::write(&model, b"a hand-placed model").unwrap();
        let other = dir.path().join("potion-base-8M");
        std::fs::create_dir_all(&other).unwrap();

        reclaim_stale_temp_dirs(dir.path(), SystemTime::now() + Duration::from_secs(86_400));

        assert!(model.exists(), "a model is not a temporary directory");
        assert!(other.exists(), "another model's directory is not either");
    }

    /// Every entry in `dir` whose name marks it as this module's
    /// temporary working space.
    fn leftovers(dir: &Path) -> Vec<String> {
        std::fs::read_dir(dir)
            .expect("list")
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(TEMP_DIR_PREFIX))
            .collect()
    }

    #[test]
    fn a_url_yields_the_host_a_message_should_name() {
        assert_eq!(host_of(ENGLISH_MODEL_URL), "lindat.mff.cuni.cz");
        assert_eq!(host_of("https://example.org:8443/a/b?c=d"), "example.org");
        assert_eq!(host_of("https://user@example.org/a"), "example.org");
        assert_eq!(host_of("not a url"), "not a url");
    }

    /// Regression (report H5 / finding 5): a rejected certificate is a
    /// sentence naming the host, not `invalid peer certificate:
    /// Other(OtherError(CaUsedAsEndEntity))`. The raw failure is kept at
    /// the end because a bug report needs it.
    #[test]
    fn a_rejected_certificate_reads_as_a_sentence() {
        let error = ureq::Error::Io(std::io::Error::other(
            "invalid peer certificate: Other(OtherError(CaUsedAsEndEntity))",
        ));
        let message = download_message(ENGLISH_MODEL_URL, &error);

        assert!(message.contains("lindat.mff.cuni.cz"), "{message}");
        assert!(message.contains("system trust store"), "{message}");
        assert!(message.contains(ENGLISH_MODEL_FILENAME), "{message}");
        assert!(
            message.contains("CaUsedAsEndEntity"),
            "the underlying failure survives: {message}"
        );
        assert_eq!(transport_failure(ENGLISH_MODEL_URL, &error).kind(), "io");
    }

    /// A failure that is not about certificates keeps its own words.
    #[test]
    fn an_ordinary_transport_failure_is_reported_plainly() {
        let error = ureq::Error::HostNotFound;
        let message = download_message(ENGLISH_MODEL_URL, &error);
        assert!(message.starts_with("download https://lindat"), "{message}");
        assert!(!message.contains("trust store"), "{message}");

        let mapped = transport_failure(ENGLISH_MODEL_URL, &error);
        match mapped {
            Error::Io(e) => assert_eq!(e.kind(), std::io::ErrorKind::NotConnected),
            other => panic!("expected Io, got {other:?}"),
        }
    }

    /// Regression (review of #77): a timeout while reading the body
    /// reports `TimedOut`, which is what `book/src/reference/errors.md`
    /// and ADR-0015's decision table both promise. `ureq`'s body reader
    /// builds its `io::Error` with `Error::into_io`, which wraps
    /// everything that is not already an `io::Error` in
    /// `io::Error::other`, so reading `kind()` off it gave `Other` for
    /// every body-phase failure. The connect phase was always right;
    /// only this one was wrong.
    #[test]
    fn a_timeout_while_reading_the_body_is_reported_as_a_timeout() {
        // Exactly the shape `ureq` hands the body reader for a transfer
        // that runs past the global budget after the response begins.
        let from_ureq = std::io::Error::other(ureq::Error::Timeout(ureq::Timeout::Global));
        assert_eq!(
            from_ureq.kind(),
            std::io::ErrorKind::Other,
            "the wrapping this test exists to undo"
        );

        match body_failure(ENGLISH_MODEL_URL, from_ureq) {
            Error::Io(e) => {
                assert_eq!(e.kind(), std::io::ErrorKind::TimedOut);
                assert!(e.to_string().contains(ENGLISH_MODEL_URL), "{e}");
                assert!(e.to_string().contains("timeout"), "{e}");
            }
            other => panic!("expected Io(TimedOut), got {other:?}"),
        }
    }

    /// The other half: an `io::Error` that never was a `ureq::Error`
    /// keeps its own kind, so unwrapping costs nothing on the ordinary
    /// path.
    #[test]
    fn a_plain_body_read_failure_keeps_its_own_kind() {
        let raw = std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "peer disconnected");

        match body_failure(ENGLISH_MODEL_URL, raw) {
            Error::Io(e) => {
                assert_eq!(e.kind(), std::io::ErrorKind::UnexpectedEof);
                assert!(e.to_string().contains(ENGLISH_MODEL_URL), "{e}");
            }
            other => panic!("expected Io(UnexpectedEof), got {other:?}"),
        }
    }

    /// A mid-stream certificate rejection reaches the same sentence the
    /// connect phase gets. It used to be wrapped by hand and lost it.
    #[test]
    fn a_certificate_rejected_mid_stream_still_reads_as_a_sentence() {
        let from_ureq = std::io::Error::other(ureq::Error::Io(std::io::Error::other(
            "invalid peer certificate: Other(OtherError(CaUsedAsEndEntity))",
        )));

        match body_failure(ENGLISH_MODEL_URL, from_ureq) {
            Error::Io(e) => {
                let message = e.to_string();
                assert!(message.contains("lindat.mff.cuni.cz"), "{message}");
                assert!(message.contains("system trust store"), "{message}");
            }
            other => panic!("expected Io, got {other:?}"),
        }
    }

    /// `ureq` follows up to ten redirects, so without `https_only` an
    /// `https` URL that redirects to `http` is fetched in cleartext. The
    /// digest pin keeps that from changing which bytes load, so this is
    /// confidentiality only, and it is still not the redirect's call.
    #[test]
    fn the_download_agent_refuses_to_leave_https() {
        let agent = download_agent();
        assert!(agent.config().https_only());
        assert!(
            agent.config().max_redirects() > 0,
            "the redirect following that makes https_only load-bearing"
        );
    }

    /// Regression (review of #77): a temporary directory carrying this
    /// process's own pid may be a live peer's. In a container every
    /// process is pid 1, so two cold starts sharing a bind-mounted model
    /// directory pick the same pid, and the unconditional removal that
    /// used to precede the create deleted the peer's download.
    #[test]
    fn a_peer_temp_directory_bearing_this_pid_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let peer = dir
            .path()
            .join(format!("{TEMP_DIR_PREFIX}{}", std::process::id()));
        std::fs::create_dir_all(&peer).unwrap();
        std::fs::write(peer.join("partial.udpipe"), b"in flight").unwrap();

        let mut rec = recorder();
        provision_fixture(dir.path(), &mut rec, &|| Ok(FIXTURE.to_vec())).unwrap();

        assert!(
            peer.exists(),
            "a fresh directory under this pid is a peer's, not this call's"
        );
        assert_eq!(
            std::fs::read(peer.join("partial.udpipe")).unwrap(),
            b"in flight"
        );
    }

    /// And the name that makes that hold: two calls in one process never
    /// choose the same temporary.
    #[test]
    fn two_temporary_names_in_one_process_differ() {
        assert_ne!(temp_stamp(), temp_stamp());
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
