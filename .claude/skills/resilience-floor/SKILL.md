---
name: resilience-floor
description: Antifragile operational discipline for matra — size caps at the entry point, symlink rejection, atomic file writes, TOCTOU closure on hash-verified loads, `catch_unwind` panic boundaries at C/C++ FFI, cycle-safety in graph walks. The Taleb principles applied. Use when adding or auditing I/O, external library boundaries, user-input handling, or failure modes. Pair with `aces` for the structural design philosophy.
---

# resilience-floor

The antifragile operational discipline for matra. This skill codifies the patterns that emerged from the i2 iteration's resilience work.

ACES (`.claude/skills/aces/SKILL.md`) is the **structural** discipline (adaptability/composability/extensibility resisting stasis/drag/opacity). This skill is the **operational** discipline that complements it: when a process gets a hostile input, when a C library panics, when two processes race on the same file, the system survives loudly, not silently. ACES designs the system to evolve; resilience-floor designs it to fail well.

## When to invoke

- Adding a new I/O path.
- Wrapping a new external library boundary.
- Adding a hash-verified load.
- Adding a graph-walk algorithm.
- Adding a feature that touches user-controlled input.
- Auditing existing code for resilience gaps.

## The six disciplines

### 1. Size caps at the entry point, not deep in the call stack

`MAX_INPUT_BYTES = 8 * 1024 * 1024` is checked once, in `Engine::annotate`, the only route from text to the parser:

```rust
fn check_input_size(text: &str) -> domain::Result<()> {
    if text.len() > MAX_INPUT_BYTES {
        return Err(domain::Error::InputTooLarge {
            limit: MAX_INPUT_BYTES,
            actual: text.len(),
            what: "input",
        });
    }
    Ok(())
}
```

Called from `Engine::annotate` before decomposition. Because annotate is the unique path to `NlpProvider::parse`, the bound is a property of the pipeline rather than a per-entry-point restatement, and equivalence law L7 pins it in tests. Source adapters (`source/file.rs::read`) check file metadata size *before* reading into memory. Extractors with quadratic-class characteristics (TextRank) check their own `MAX_SENTENCES` cap.

**Rule**: the gate lives at the unique choke point, checked before real work. Deep callers may trust the bound is already checked.

**Discriminator**: each `InputTooLarge` carries a `what: &'static str` so consumers can distinguish input-too-large at the apex from per-extractor caps:

| `what` value | Where it fires |
|---|---|
| `"input"` | `Engine::annotate` in `lib.rs` |
| `"file_source"` | `source/file.rs` before reading the file |
| `"rake"`, `"yake"` | Per-extractor caps in `extraction/` |

When adding a new gate, pick a distinct `what` discriminator.

### 2. Symlink rejection

`FileSource` and `DirectorySource` use `symlink_metadata` (non-traversing) and reject any path whose file type is a symlink. This prevents path-redirection attacks: an attacker who controls a path passed to matra cannot redirect to an arbitrary file via a symlink.

```rust
let metadata = std::fs::symlink_metadata(input)?;
if metadata.file_type().is_symlink() {
    return Err(Error::Io(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        format!("refusing to read symlink: {}", input.display()),
    )));
}
```

When adding a new `Source` adapter, follow the same pattern.

### 3. Atomic file writes via per-process temp + rename

`nlp/udpipe.rs::download_english` writes to `<dir>/.tmp.download.<pid>/`, then `std::fs::rename`s the file to its final path:

```rust
fn download_english(dir: &Path, final_path: &Path) -> domain::Result<()> {
    with_temp_subdir(dir, |tmp_dir| {
        let tmp_str = tmp_dir.to_str()...?;
        udpipe_rs::download_model("english-ewt", tmp_str)?;
        let tmp_file = tmp_dir.join(ENGLISH_MODEL_FILENAME);
        std::fs::rename(&tmp_file, final_path)?;
        Ok(())
    })
}
```

`std::fs::rename` is atomic on the same filesystem (POSIX `rename(2)`, Windows `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`). Concurrent processes calling the same operation cannot corrupt each other's files because each downloads to its own per-pid temp subdirectory and only the rename touches the final path.

When adding a new file-write path:

- Write to a per-process temp subdirectory.
- Rename to the final path as the last step.
- Use `Drop` (the `Cleanup` struct in `udpipe.rs`) to remove the temp on scope exit, even on panic.

### 4. TOCTOU closure on hash-verified loads

`nlp/udpipe.rs::read_and_verify` reads the file *once* into memory, hashes those bytes, and returns the same bytes for the loader:

```rust
fn read_and_verify(
    path: &Path, expected_size: u64, expected_hash: &str,
) -> domain::Result<Option<Vec<u8>>> {
    let meta = std::fs::metadata(path)?;
    if meta.len() != expected_size { return Ok(None); }
    let bytes = std::fs::read(path)?;
    let got = hex_encode(&Sha256::digest(&bytes));
    if got.eq_ignore_ascii_case(expected_hash) {
        Ok(Some(bytes))
    } else {
        Ok(None)
    }
}
```

The loader (`Udpipe::from_bytes`) uses the returned bytes directly. There is no second disk read between verify and load. An attacker with write access who swaps the file between verify and a hypothetical second read cannot affect the loaded model because no second read happens.

**Rule**: a hash-verify function must return the verified bytes, not just a boolean. Never re-read the disk after verify.

### 5. `catch_unwind` panic boundary at C/C++ FFI

`nlp/udpipe.rs::catch_parse_panic` wraps `Model::parse` (the C++ UDPipe call):

```rust
fn catch_parse_panic<F, T>(f: F) -> domain::Result<T>
where F: FnOnce() -> domain::Result<T> {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(result) => result,
        Err(payload) => {
            let message = payload.downcast_ref::<&'static str>()...;
            Err(Error::ParseFailed(format!("udpipe panicked: {message}")))
        }
    }
}
```

Without this wrapper, a panic inside `Model::parse` would abort the host process — interpreter death in Python, trap in WASM. The wrapper converts a C-side panic into `Err(ParseFailed(_))`, which the host can handle.

**Rule**: every C/C++/FFI boundary in matra is wrapped in `catch_unwind`. If the call site is async, use the synchronous variant inside the runtime.

### 6. Cycle-safety in graph walks

`Sentence::tree_depth` uses a HashMap+visited-set walk with memoization:

```rust
pub fn tree_depth(&self) -> usize {
    // For each token, walk to root via head chain.
    // Cycle detection: visited set inside each walk.
    // On cycle, every token in the cycle gets usize::MAX.
    // ...
}
```

Cycles return `usize::MAX` for tokens transitively in the cycle. The malformed parse surfaces loudly rather than silently truncating.

`Sentence::subtree` uses a HashSet visited set to prevent infinite loops in cyclic graphs.

**Rule**: every graph-walk algorithm on user-derived data (parse trees, dep graphs, etc.) has cycle detection. Magic numbers ("stop after 20 hops") are not cycle detection; they are covered-up bugs.

## The Taleb principles

From the antifragility lens:

1. **Single Points of Failure are bugs.** Find and fix; the UDPipe C boundary was an SPOF that the `catch_unwind` boundary fixed.
2. **Bounded inputs.** Unbounded input = unbounded resource use. The cap is a feature, not a limitation.
3. **Fail loud, not silent.** Cycles return `usize::MAX`; mismatched hashes refuse to load; oversized inputs error at the gate. Never silently truncate, downgrade, or proceed.
4. **Atomic over racy.** If two processes can race, use atomic operations (rename, CAS).
5. **Trust anchors are pinned, not configurable.** `ENGLISH_MODEL_SHA256` is a `const` in source, not an env var.

## When you add a new failure mode

Before merging the change, run the audit:

- Does the new code have an entry-point size check?
- Does it reject symlinks (if I/O)?
- Does it use atomic rename (if file-writing)?
- Does it return verified bytes (if hash-verifying)?
- Is it wrapped in `catch_unwind` (if FFI)?
- Does it detect cycles (if graph-walking)?

If any of these is "no" without an explicit reason, the change is incomplete.

## Add a regression test

Every fixed failure mode gets a regression test that the failure cannot recur without somebody noticing. The i2 work shipped these:

- `parse_per_paragraph_*` — the prefix-match defect cannot recur.
- `tree_depth_*` — the magic-ceiling and silently-truncated-cycle cannot recur.
- `read_and_verify_returned_bytes_are_what_was_hashed` — the TOCTOU window cannot recur.
- `catch_parse_panic_converts_*_panic_to_parse_failed` — the panic-aborting-host cannot recur.


## What this skill won't tell you

- Specific panic-recovery patterns at runtime — case-by-case.
- Profiling for memory leaks — that's a separate tool (heaptrack, valgrind) and not covered here.
- Async failure modes — matra is synchronous; if/when async lands, this skill grows.
