# 0010. Embeddings: a Tier-2 channel behind an Embedder port, static adapter first

- **Status:** Accepted
- **Date:** 2026-09-04
- **Decider(s):** project maintainer; design in the i9 plan, grounded by the 2026-09-04 landscape survey and a Karman naming review

## Context

The self-similarity roadmap entry (trigger fired 2026-08-21) needs sentence embeddings for its semantic half: paraphrase restatement that lexical overlap cannot see. Embeddings are model opinion, not verifiable structure, so the roadmap's scoping principle admits them only as a specialist adapter with the tier stated plainly. The i9 plan carries the milestones and rubrics; this ADR locks the decisions the plan builds on.

## Decisions

### 1. Tier 2 travels in its own types, never as `Document` fields

ADR-0008's rule (derivations cross FFI as fields) applies to derivations of the parse, which are checkable against the source bytes. An embedding is not a derivation of the parse; no consumer can verify a cosine score against the text. Semantic results therefore never become fields on `Document`, `Sentence`, or any type the deterministic pipeline returns. They arrive as standalone values from separate calls, and each such type carries its provenance: the model hash and the parameters that produced it.

### 2. The port is `Embedder`; the carrier is `domain::Embedding`

`Embedder` follows the majority port pattern (agent noun from capability: `Decomposer`, `Source`; `NlpProvider` is the forced exception, not the rule). The trait lives in `embed/mod.rs`, imports only `domain`, and names one method, `embed(&[&str]) -> Result<Vec<Embedding>>`, with the contract on the trait: output length equals input length, uniform dimension.

`Embedding` sits in `domain.rs` because ports name only domain types (boundary rule 2). Its shape is a newtype tuple struct, `Embedding(pub Vec<f32>)`: the type name itself is the constructor external implementors call, and serde serializes a newtype transparently as the bare array, which is the compact wire form a vector deserves (a named field would wrap every vector in an object for no information gain). It deliberately departs from the `#[non_exhaustive]` convention: on a tuple struct the attribute makes the constructor crate-private, and external `Embedder` implementors must construct `Embedding` values, which is the port's whole purpose. The departure is this ADR's to record so review does not "fix" it later.

### 3. The first adapter is static (model2vec format); the transformer comes later behind the same port

The 2026-09-04 survey verified both paths compile for wasm32-unknown-unknown with zero C in the closure. The static path wins the first slot on one decisive property plus three supporting ones: a static embedding is a table gather plus mean-pool, with no kernel dispatch, so vectors are bit-identical across Rust, Python, and a future WASM crust, turning cross-crust conformance from tolerance assertions into exact ones; the quality cost is roughly ten percent against a small transformer at a third of the size; the dependency closure stays two crates; and the port makes a candle BERT adapter a later addition, not a redesign. The adapter is matra-owned (the existing model2vec Rust crate carries CLI deps as library deps, a stale closure, and license metadata that trips scanning); a parity fixture against the Python reference pins format compatibility.

### 4. Adapter features name their backend

The feature is **`model2vec`**, not `embeddings`, and the adapter file is **`embed/model2vec.rs`**, on the `udpipe` precedent now stated as a rule: a cargo feature gates an adapter, so it carries the adapter's name. A capability-named feature schedules its own collision: the moment the candle adapter arrives behind its own feature, `embeddings` would permanently mean "the static adapter specifically" while claiming the whole capability. This rule answers the candle feature name in advance (`candle`, or the model family it speaks for). The crate-name shadow over the third-party `model2vec` crate is the same shadow `udpipe` casts over `udpipe-rs`.

### 5. Semantic clusters are connected components carrying their edges

`SemanticClusters` (the inner per-cluster type is reserved as `SemanticCluster`) holds the model hash, the caller-supplied threshold, and each cluster as member sentence indices plus the above-threshold edges connecting them. Components, not cliques: chained restatement (a restates b, b restates c) is the consumer pattern, and cliques would split it. The cost is visible by construction: co-membership is transitive and must never be read as pairwise similarity, which is why the edges travel in the type; a sentence with no above-threshold edge appears in no cluster, so singletons are excluded and "unclustered" is a meaningful count. "Semantic" in the name is the tier label, not a verdict: it declares model opinion in the one place that fact must be unmissable, and it disambiguates from the lexical similarity TextRank already computes. The threshold is caller-supplied because the literature's published cutoffs span 0.67 to 0.9 with no consensus.

### 6. Model supply keeps the UDPipe discipline

Caller-supplied files, SHA-256 verified through the read-then-consume pattern (hash bytes in memory, load from those bytes, never re-read disk between verify and load). The reference model is pinned by hash in `spec/`, part of the conformance contract exactly as the UDPipe model is. The backend stays pure Rust with no C FFI while the WASM path is open; `cargo check --no-default-features --features model2vec --target wasm32-unknown-unknown` becomes a CI gate when the adapter lands.

## Amendments

**2026-09-05 (M5).** Two signature refinements surfaced by the wiring milestone, neither changing any decision above. `Embedder` gains a second required method, `identity(&self) -> &str`: provenance is part of the port contract, because the composition root cannot otherwise attribute scores to the model that produced them, and a caller-carried hash could be the wrong one. And `extraction::semantic_clusters` takes `(embeddings, threshold, model_hash)` rather than also taking the sentence slice: the slice was only ever read for its length, and the document-to-embeddings correspondence check belongs to `embed_and_cluster`, the composition-root function that holds both halves. The plan's sketches are updated in lockstep.

## Options considered and rejected

- **candle BERT as the first adapter** (the plan's original shape): viable and verified, but loses bit-parity to kernel dispatch, and its closure is the whole inference stack. It remains the designated second adapter.
- **Depending on `model2vec-rs`**: rejected for closure hygiene (CLI deps as library deps, version skew, license metadata).
- **ONNX Runtime paths (ort, fastembed)**: C FFI; closes the WASM path for every downstream consumer, forever.
- **Feature named `embeddings`**: rejected by the naming review for the collision in Decision 4.
- **Cliques for clustering**: splits chained restatement, the consumer pattern.

## Consequences

- The `abstract` seam stays empty: this is adapter plus consumer work, not rule evaluation.
- The i9 plan text is amended to the settled names; boundary-rules gain `embed/mod.rs` (port list) and `embed/model2vec.rs` (sole-importer rule) when the code lands, in the same PR, per docs-lockstep.
- A vocabulary drift found during the naming review is recorded on ADR-0006's deferred list rather than fixed here: `CorpusEntry.analysis` still carries the name ADR-0006 rejected, now SemVer-major to rename; the free half (metric function parameter names) may be fixed any time.

## Validation

The decisions rest on falsifiable claims, and each has a check:

- **Bit-parity** (the reason static won the first slot): M3's fixture asserts identical vectors on x86_64 and aarch64, and M6's conformance fixture asserts exact vectors rather than tolerances. If exactness cannot be held, the static-first rationale loses its decisive property and this ADR should be revisited, not patched around.
- **Format compatibility:** the parity fixture against the Python model2vec reference on pinned inputs. Failure means the adapter does not actually speak the format it claims.
- **The WASM constraint:** `cargo check --no-default-features --features model2vec --target wasm32-unknown-unknown` in CI from M3 onward. A C-FFI crate entering the closure fails this loudly.
- **Revisit trigger for static-first:** a consumer demonstrating that the roughly ten percent quality ceiling causes missed paraphrase clusters that matter to their use. The answer is the candle adapter behind the same port (a new ADR is not required; this one already designates it), not a change to the surface.
- **Revisit trigger for the feature-naming rule:** if a second adapter arrives whose natural backend name collides with an existing feature, the rule needs a tiebreak amendment.

## References

- [i9 plan](../../book/src/plans/i9-embeddings-adapter.md): milestones, rubrics, and the survey-driven amendment trail
- [ADR-0006](0006-abstract-tier-vocabulary-lock.md): the vocabulary lock this ADR's naming review extends, and the deferred list that gained `CorpusEntry.analysis`
- [ADR-0007](0007-one-pipeline.md): the pipeline whose annotate/compose split the embed-then-cluster shape mirrors
- [ADR-0008](0008-structural-primitives-are-fields.md): the FFI channel rule this ADR scopes to parse derivations
- Landscape survey, 2026-09-04 (an internal survey, not in this repository): the verified wasm32 compilation results, the static-vs-transformer evidence, and the threshold-spread literature
