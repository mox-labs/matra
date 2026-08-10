# I6 — Post-publish: OTel, PDF/DOCX, `rumi-nlp` patterns, possibly the reactor

**Status:** not-started
**Boundary:** post-0.1.0
**Depends on:** I5 (MLP shipped, 0.1.0 published)

## Why this iteration exists

I0 through I5 ship 0.1.0. Some work was deliberately deferred:

- **OTel export** (Wolf PR2/PR3) — `tracing-opentelemetry` is heavy (~30 transitive crates). Substrate library doesn't bundle it; consumers opt in.
- **PDF/DOCX adapters** — half-shipping a PDF adapter would lock a bad shape into the public surface (PDF is a format family). User originally said "any kind of file"; we deferred deliberately, documented the gap.
- **`rumi-nlp` pattern content** — at 0.1.0 the bridge crate ships with primitives only (one `DataInput<Sentence>` smoke test). Domain-specific patterns (SVO, copular, prepositional, passive, nominal modifier; stance classification; relation extraction) land here, driven by real consumer needs rather than speculation.
- **The reactor pattern** — Erlang and K converged on defer; the streaming iterator covers the load. Reactor returns only if named triggers fire.

This plan is a holding pattern: each sub-iteration ships only when its specific trigger fires.

## Sub-iterations (each ships independently)

### I6a — `otel` feature

**Trigger:** at least one downstream consumer requests OTel export, OR a 0.1.0 ship-readiness review identifies the OTel story as a publish blocker.

**Files:** workspace `Cargo.toml`, `src/lib.rs` or `src/obs.rs` (new), `examples/observability_otel.rs`.

**Why (Wolf):** "`tracing-opentelemetry` together pulls ~30 crates. Never default. Consumers turn it on; library publishes to crates.io with default features only."

**Steps:**

1. Add `tracing-opentelemetry` and `opentelemetry` as optional deps gated on `otel` feature (skeleton already added in I3 task B; verify and complete).
2. Implement an `init_otel(endpoint)` helper or document the consumer-side init pattern.
3. `examples/observability_otel.rs`: end-to-end example with OTLP collector setup.
4. Document the network surface: `tracing-opentelemetry` exports to a configured endpoint. Consumer is responsible for endpoint validation, TLS, and auth.

**Acceptance:**
- `cargo build --features otel` succeeds.
- Example runs against a local Jaeger / Honeycomb / OTLP collector.
- README has an "OTel Export" subsection under Observability.
- `cargo build` (default) does **not** pull `tracing-opentelemetry`. Verified by `cargo tree`.

### I6b — Per-extractor and per-source spans (Wolf PR2)

**Trigger:** a consumer reports they cannot debug an extraction issue from the I3 instrumentation.

**Files:** `src/extraction/{tfidf,textrank,rake,yake}.rs`, `src/source/{file,directory}.rs`.

**Why (Wolf, deferred from I3):** finer-grained spans per extractor algorithm and per-file in directory processing.

**Steps:**

1. Per-extractor INFO spans were added in I3 task C — extend with per-iteration TRACE events for TextRank (PageRank delta convergence).
2. Per-file DEBUG events inside `analyze_directory_iter`: `tracing::debug!(?path, "matra.document.parsed")` on success.
3. Update `examples/observability.rs`.

**Acceptance:** TextRank trace at `RUST_LOG=matra=trace` shows iteration deltas. Per-file debug event present.

### I6c — `PdfDecomposer` adapter

**Trigger:** at least one consumer commits to needing PDF support.

**Files:** `src/decompose/pdf.rs` (new), `the crate/Cargo.toml` (new feature `pdf`).

**Why:** the library's `Format::Pdf` returns `UnsupportedFormat` today. PDF is a common input format and the gap closes once a consumer commits. Half-shipping was deferred deliberately (K, 2026-04-28: "PDF is a format family, not a format. Defer cleanly, don't half-ship.").

**Steps:**

1. Choose a PDF parsing crate. Candidates (audit before commit): `pdf-extract`, `lopdf`, `pdfium-render`. Each has tradeoffs.
2. Implement `PdfDecomposer: Decomposer`. Map PDF text into `Section`/`Paragraph`. Preserve heading hierarchy where extractable; fall back to plain text.
3. Gate behind `pdf` feature flag. The default build does not pull a PDF parser.
4. Tests: 3-page PDF fixture (no images, just text) decomposes into expected paragraph count.
5. Edge cases: encrypted PDF (refuse with `Error::ParseFailed { kind: MalformedInput, .. }`), empty PDF, scanned-image PDF (returns one paragraph or fails gracefully).

**Acceptance:** PDF feature opt-in, basic PDF fixture decomposed. CHANGELOG documents the support surface.

### I6d — `DocxDecomposer` adapter

**Trigger:** same as I5c but for DOCX.

**Files:** `src/decompose/docx.rs` (new), `the crate/Cargo.toml` (new feature `docx`).

**Steps:** parallel to I5c but for DOCX. Likely candidate crate: `docx-rs` or `zip` + custom XML parsing.

**Acceptance:** parallel to I5c.

### I6e — `rumi-nlp` pattern content

**Trigger:** at least one consumer commits to needing rule-based extraction over `Sentence` data, OR a clear pattern emerges across multiple consumer requests.

**Files:** `crates/rumi-nlp/src/inputs/`, `crates/rumi-nlp/src/matchers/`, `crates/rumi-nlp/src/compile.rs` (new), conformance tests under `crates/rumi-nlp/spec/`.

**Why:** matra 0.1.0 ships `rumi-nlp` with a skeleton (one `DataInput<Sentence>` smoke test) so the architecture locks. Domain-specific content lands incrementally, driven by real consumer needs, not speculation. The barbell argument from the prior session: "Matra ships a built-in extractor (safe side: five patterns, deterministic, ~300 lines, bounded precision)" was redirected — relation extraction belongs in domain extension crates, not in matra-core. With `rumi-nlp` colocated in matra's workspace, the patterns live there when they land.

**Possible scope (each its own plan when triggered):**

1. **Tree-walk DataInputs.** `PosInput`, `LemmaInput`, `DepInput`, `HeadInput`, `SubtreeInput`, `ChildByLabelInput`. Each navigates the dep tree internally and returns flat `MatchingData`.
2. **The five extraction patterns.** SVO, copular, prepositional, passive, nominal modifier. As `Matcher<Sentence, Triplet>` configurations using the DataInputs above. Conformance test suite (YAML fixtures, like rumi-http's).
3. **`compile_nlp_rules()` config compiler.** Takes user-friendly YAML/TOML rule configs and produces `Matcher<Sentence, A>` trees.
4. **Stance classification.** Nine-rule epistemic cascade (potential / assertoric / directive) as a matcher list. Reads `Token.feats`, `dep`, `lemma`. **Note:** stance is at the boundary of matra's substrate scope vs consumer's interpretive scope. May land in a separate downstream crate, not in `rumi-nlp`. Decide when triggered.

Each sub-item is its own plan. None of them ship in 0.1.0.

**Acceptance:** depends on the sub-item. Each lands with conformance fixtures.

### I6f — The reactor (only if triggered)

**Trigger:** **any one** of:
1. A consumer needs incremental re-analysis on file change (push semantics).
2. A corpus consumer reports more than 100k documents in regular use.
3. A second `Source` arrives that is inherently push (websocket, filesystem watch, message queue).

**Files:** depends on the trigger. Likely: `src/runtime.rs` (new), workspace `Cargo.toml` (new feature `async`, new optional dep `tokio`).

**Why (Erlang + K, deferred from 2026-04-28):** "A reactor without a parallel sink is a bigger queue, not more throughput. Hot path for all three downstream consumers is `analyze(doc)` on a single document. ~140 lines of glue do not justify an async dependency tree."

**Steps (sketch only; actual design happens at trigger time):**

1. Convene the guild before any code lands. Erlang and K must re-evaluate against the trigger condition.
2. Likely shape: `tokio` runtime, channel between `decompose` and `parse` (Erlang's identified backpressure boundary), worker pool sized to NLP provider count.
3. UDPipe still `unsendable`; reactor can only parallelize across instances, not across threads sharing one model.
4. Public surface: a new `EngineAsync` or `Reactor` type. Existing `Engine` and free functions stay (sync path remains the 90% case).

**Acceptance:** depends on the trigger. The acceptance gate cannot be defined in advance.

## Cross-cutting: post-ship loop closure

**Files:** `scratch/post-ship-0.1.0.md` (new at I4 ship; updated as triggers fire).

**Why (Ixian):** "Within 2 weeks of 0.1.0 publish, check crates.io download count, GitHub issues with label `panic`/`crash`/`oom`/`hang`. Record in `scratch/post-ship-0.1.0.md`."

**Steps:**

1. After 0.1.0 publishes (separate explicit approval), open the file.
2. Capture initial measurements: download count, open issues by label, observed consumer adoption.
3. Update at week 2, week 4, week 8.
4. Each I5 sub-iteration's trigger condition is evaluated against this file. If a trigger fires, the corresponding sub-iteration starts.

**Acceptance:** the file exists. It is not optional.

## Validation

Per sub-iteration, not for I5 as a whole.

## Acceptance gate

I5 has no overall acceptance gate. Each sub-iteration ships independently when its trigger fires.

## Risks

- **Risk:** I5a (OTel) becomes urgent and ships without a clear feature-flag boundary. Ends up bloating the default build.
  - **Mitigation:** verify `cargo tree` at landing.

- **Risk:** I5c/I5d (PDF/DOCX) ship a half-built adapter that locks a bad shape.
  - **Mitigation:** the trigger is "consumer commits to needing this." Without a real consumer, the surface design is speculative. Don't ship speculative.

- **Risk:** I5e (reactor) is implemented because someone in the conversation thinks it sounds nice, not because a trigger fired.
  - **Mitigation:** the trigger conditions are named. If none have fired, the reactor does not ship. Period.

- **Consult:** Erlang and K before any I5e work begins. The 2026-04-28 deferral is binding until a trigger fires; reopening requires the same lens.
