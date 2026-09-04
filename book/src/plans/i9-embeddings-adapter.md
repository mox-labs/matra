# I9: Embeddings as a specialist adapter

**Boundary:** first post-publish capability. Additive only; 0.1.0 froze the surface, so nothing here may change an existing signature.

**Origin:** the self-similarity roadmap entry (trigger fired 2026-08-21) names two halves. The deterministic half (lexical clusters, redundancy ratio, rep-n, skeleton repetition) needs no new capability. The semantic half, paraphrase restatement with different vocabulary, is out of reach of lexical overlap by design and needs sentence embeddings, which sit above the verifiable tier. The roadmap's scoping principle already states the only acceptable arrival: a specialist adapter behind its own feature flag, with its tier stated plainly.

---

## Why this shape and not another

### The tier line is the design

Everything on `Document` today is deterministic and grounds back to the bytes it came from. An embedding does not: two runs of two different models give two different geometries, and no consumer can check a cosine score against the source text. So the one thing this iteration must not do is put Tier 2 output behind a Tier 1 surface.

The consequence is structural, not documentary. Semantic similarity results never become fields on `Document`, `Sentence`, or any type the deterministic pipeline returns. They arrive as a separate value, from a separate call, whose type names its tier. ADR-0008's rule (derivations cross FFI as fields) applies to derivations of the parse; an embedding is not a derivation of the parse, it is another model's opinion, and it gets another channel.

### Pure Rust is the WASM decision, made now

`cargo check --no-default-features --target wasm32-unknown-unknown` passes today; UDPipe's C FFI is the only thing keeping the TS crust hypothetical. The embeddings backend is chosen so that stays true: candle (pure Rust, compiles to wasm32, no C in the dependency closure) and not ort or fastembed (ONNX Runtime, C FFI, would close the WASM path for every downstream consumer, forever). This constraint was settled 2026-08-21 and is not reopened here; the plan's job is to hold it, which is why the WASM check is a milestone gate and not a hope.

### The port comes with its proving consumer

A port with no consumer is speculation, and I7's lesson was that shape is pulled from real use. So this iteration ships the trait together with the one consumer the roadmap already names: semantic-equivalence clustering over sentences, the second half of the redundancy family. The deterministic half is independent (it rides TextRank's existing similarity matrix) and can land before, after, or beside this work; nothing here blocks it.

---

## The surface

```
// domain.rs: the carrier, so ports can name it (rule 2)
pub struct Embedding(pub Vec<f32>);

// embed/mod.rs: the port, importing only domain (rules 2, 3)
pub trait Embedder: Send {
    /// Embed each text. Postcondition: output length equals input length,
    /// and every vector has the same dimension.
    fn embed(&self, texts: &[&str]) -> domain::Result<Vec<Embedding>>;
}

// embed/candle.rs: the adapter, behind the `embeddings` feature.
// The ONLY file importing candle crates (rule 4 analog).

// extraction (or metrics): a pure function over domain types (rule 5 holds)
pub fn semantic_clusters(
    sentences: &[Sentence],
    embeddings: &[Embedding],
    threshold: f32,
) -> domain::Result<SemanticClusters>
```

The load-bearing move is the last one. Rule 5 says `metrics/` and `extraction/` import only `domain` and `stopwords`, so the clustering function cannot call the port. It does not need to: it takes precomputed vectors, which are domain values, and stays a total function testable without any model. Whoever holds both the `Document` and the `Embedder` runs one, then the other. That is the composition root's job, exactly as rule 7 wants, and it is the same split the pipeline already uses (`annotate` touches the provider, `compose` is total).

`SemanticClusters` names its tier in the type: it carries the model identity (the caller-supplied model hash), the threshold used, and each cluster as its member sentence indices together with the above-threshold edges that connect them (index pairs with their scores). Clusters are connected components, so co-membership is transitive: two sentences can share a cluster without sharing an edge. The type reports only the edges that cleared the threshold, which is why they travel alongside the members; a consumer must never read co-membership as pairwise similarity, and the type's docs say so. It is serde-visible so it can cross FFI, and it is a standalone value, never attached to `Document`. Whether a cluster constitutes restatement is the consumer's reading; the word fluff appears nowhere.

Threshold is caller-supplied. matra does not know a universal similarity cutoff, and pretending to would be a fixed opinion wearing a constant's clothing.

## Model supply, same discipline as UDPipe

No network in the library. The caller supplies model files (weights, tokenizer, config) and the adapter verifies bytes by SHA-256 through the same read-then-consume pattern `read_and_verify` established: hash the bytes in memory, load from those same bytes, never re-read disk between verify and load (TOCTOU stays closed). The reference model for conformance fixtures is pinned by hash in `spec/`, because the model is part of the contract exactly as the UDPipe model is.

The candidate reference model is all-MiniLM-L6-v2 (384 dimensions, BERT family, the sentence-transformers standard recipe: mean pooling over token embeddings, then L2 normalization, both implemented in the adapter since candle-transformers hands back token-level output). Candidate, not commitment: the milestone verifies candle-transformers loads it cleanly on the pinned candle version before the spec pins anything.

## Milestones

Each leaves the tree green. Names settle before code moves (ontology first).

| M | What | Gate |
|---|---|---|
| 1 | Names and ADR-0010 | ADR records tier channel, port shape, model discipline, pure-Rust constraint |
| 2 | `Embedding` in domain, `Embedder` port | additive; no existing signature changes; rules 1 to 3 hold |
| 3 | candle adapter behind `embeddings` feature | pins settled; sole importer; panic boundary; wasm32 check passes with the feature on |
| 4 | `semantic_clusters` + `SemanticClusters` | pure over domain; postconditions tested modelless |
| 5 | Composition root wiring + Python surface | rule 7; FFI fields-not-methods; size cap honored |
| 6 | Conformance fixtures + docs lockstep | spec pins the reference model by hash; book, CHANGELOG, ROADMAP updated |

### M1: names and the ADR

`Embedder`, `Embedding`, `SemanticClusters`, feature name `embeddings`. Run the names through the naming gate before any code. ADR-0010 records: Tier 2 output travels in its own types and never as `Document` fields; ports name only domain types; the adapter is the sole candle importer; models are caller-supplied and hash-verified; the backend must stay pure Rust while the WASM path is open; clusters are connected components reporting their above-threshold edges, not cliques (chained restatement is the consumer pattern and cliques would split it; the cost, co-membership without direct similarity, is visible because the edges travel in the type); and `Embedding` departs from the `#[non_exhaustive]` convention (see M2), with the departure recorded. Update the boundary-rules reference: `embed/mod.rs` joins the port list, candle crates join the sole-importer rule.

### M2: domain carrier and port

Purely additive. `Embedding` derives what `domain.rs` already derives (serde-visible), and it deliberately departs from the `#[non_exhaustive]` convention: the attribute is legal on a tuple struct, but its effect there is to make the constructor crate-private, and external `Embedder` implementors must be able to construct `Embedding` values, which is the port's whole purpose. Struct shape (tuple vs named field) is decided at M1; ADR-0010 records the departure and its reason. The port module contains only the trait and the feature-gated adapter declaration, mirroring `nlp/mod.rs` line for line.

**Rubric.** `cargo check --no-default-features` clean. No port imports another port. The trait contract (length preservation, uniform dimension) is written on the trait, because it is what M4 tests against.

### M3: the candle adapter

Version pins are settled here against the live crates (candle-core, candle-transformers, tokenizers), following the pin rules; do not trust remembered versions. The adapter owns tokenization, forward pass, pooling, and normalization. candle is pure Rust, so its panics are Rust panics, not C aborts; a malformed model file must surface as `domain::Error`, not a crash, so the same catch_unwind seam UDPipe uses wraps the load-and-forward path with the panic converted at the boundary.

**Rubric.** `scripts/check-boundaries.sh` extended: only `embed/candle.rs` imports candle. `cargo check --no-default-features --features embeddings --target wasm32-unknown-unknown` passes, which is the entire justification for the backend choice, enforced mechanically. Feature stays additive: enabling `embeddings` changes nothing about existing behavior.

### M4: the proving consumer

`semantic_clusters` is total over its inputs and returns `Err` only on contract violation (length mismatch between sentences and embeddings, dimension disagreement). Clustering over a pairwise cosine matrix with a caller threshold; connected components above threshold, the same graph discipline as TextRank, cycle-safe by construction.

**Rubric.** Tested without any model: hand-built vectors with known geometry produce known clusters, including the chained case (a near b, b near c, a far from c) asserting one component whose edge list omits the far pair. Postconditions: every sentence index appears in at most one cluster; every reported edge score is above the threshold; cluster membership equals the transitive closure of the reported edges; empty input yields empty clusters, not an error.

### M5: wiring and the Python crust

The composition root grows the one function that holds both halves (embed the sentence texts, then cluster). The Python surface exposes it behind the same feature; `SemanticClusters` crosses as serialized fields per ADR-0008's channel discipline. Sentence text reaching the embedder has already passed the pipeline's size cap because it came out of `annotate`; no second cap is introduced.

**Rubric.** No PyO3 method on a type that should be data. `From<domain::Error> for PyErr` stays exhaustive if M2 added variants. `maturin develop` then the Python suite passes. A modelless shape fixture lands in `spec/tests/` in the same change as the crossing (serialized `SemanticClusters` from M4's hand-built vectors), because ADR-0008's lockstep is fixture-with-crossing, not fixture-eventually; M6 then pins only the reference-model conformance fixture.

### M6: conformance and docs

Spec fixtures pin input sentences, the reference model hash, and expected cluster membership (scores asserted with tolerance, since float geometry varies by target); the shape fixture already landed with M5. Book gains an embeddings page stating the tier in the first paragraph; the ROADMAP redundancy entry is updated to record which half shipped; CHANGELOG under Unreleased.

---

## Costs, named

1. **The dependency closure grows substantially.** candle plus tokenizers is the largest addition since UDPipe. It is feature-gated and additive, but `cargo deny` and the license audit must pass on the full closure, and build time with the feature on will be felt.
2. **A second model artifact.** Consumers of the semantic half now manage two caller-supplied models. The discipline is identical, which is the mitigation.
3. **Float nondeterminism across targets.** Same model, same input, slightly different scores on different hardware. The spec handles it with tolerances, and cluster membership near the threshold can genuinely differ; fixtures must not sit near their own threshold.
4. **`Embedding` in domain is a commitment.** Rule 2 forces the carrier into `domain.rs`, so the type is public surface from M2 onward even though most consumers only ever see `SemanticClusters`.

## Risks

**candle API churn.** candle is pre-1.0 and moves. The pin rules apply, the adapter is one file, and the port means a replacement backend is an adapter swap, not a surface change. That is what the port is for.

**The reference model bet.** If candle-transformers does not load the candidate model cleanly, M3 picks another sentence-embedding model from candle's supported set. The trait and the consumer are model-agnostic; only `spec/` cares which one is pinned.

**Scope pull toward the deterministic half.** The lexical redundancy family will be tempting to fold in here. It has no dependency on any of this and deserves its own plan against the rule-vocabulary question the roadmap poses (metric family, extractor, or first rule pack). Keeping it out keeps this iteration one thing.
