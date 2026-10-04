# EPR-0005: Embeddings and semantic clusters

- Feature Name: `embeddings_and_semantic_clusters`
- Start Date: 2026-10-04
- Proposal PR: [#133](https://github.com/mox-labs/matra/pull/133)
- Tracking issue: none (a baseline proposal describes code that already ships, so there is no milestone to track)
- Status: proposed (only the owner changes it to `accepted`)
- Pinned at: `4fcfb4adc85524243c5e392becd4f490e3e42252`

## Summary

matra can group a document's sentences that say the same thing in different
words. It does so through the `Embedder` port, whose one built-in adapter,
`Model2Vec`, loads a static embedding model in the model2vec format behind
the `model2vec` feature. Because an embedding is a model's opinion rather
than structure a caller can check against the text, nothing derived from
one ever becomes a field on `Document` or any type the deterministic
pipeline returns. Semantic results arrive from separate calls, as
`SemanticClusters`, which carry the identity of the model that produced
them and the threshold the caller chose.

This is a baseline proposal. It describes the code at the pinned commit and
carries forward the reasons that still hold from RFC-0010 and EP-0009, with
the provisioning that RFC-0011 and RFC-0015 added described in
[EPR-0003](0003-distribution-and-provisioning.md). A later proposal that
changes any of it says so. Unmarked statements about the code are observed
at the pinned commit through the link beside them; a claim that is
inferred or assumed says so.

## Motivation

Lexical overlap cannot see a paraphrase: "the committee approved the
proposal without debate" and "the proposal was approved by the committee
with no discussion" share few words. Sentence embeddings can. But two
models give two geometries, and no caller can verify a cosine score against
the source bytes, which every other output of matra allows
([EP-0009](../legacy/eps/0009-embeddings-adapter.md), "The tier line is the
design"). The capability is admitted only on terms that keep that
difference visible:

- **A separate channel.** [RFC-0008](../legacy/rfcs/0008-structural-primitives-are-fields.md)'s
  rule that derivations cross as fields applies to derivations of the
  parse. An embedding is not one, so it gets its own types and calls
  ([RFC-0010](../legacy/rfcs/0010-embeddings-adapter.md), decision 1).
- **Provenance in the result.** Every result names the model whose geometry
  produced it, read from the embedder itself so it cannot be the wrong one
  (RFC-0010, M5 amendment).
- **A port, so the model is replaceable.** A caller with embeddings from a
  service or a library matra does not ship can supply them, in Rust or in
  Python.

## Guide-level explanation

### In Rust

```rust
use matra::embed::model2vec::Model2Vec;

let cfg = matra::config::Config::resolve()?;
let model = Model2Vec::from_config(&cfg)?; // downloads the pinned model once
let engine = matra::Engine::from_config(&cfg)?;
let raw = matra::domain::RawDocument::new(text, None, matra::domain::Format::PlainText);
let doc = engine.annotate(&raw)?;

let clusters = matra::embed_and_cluster(&doc, &model, 0.8)?;
for c in &clusters.clusters {
    // c.members: sentence indices in document order
    // c.edges: the pairs that cleared 0.8, with their scores
}
assert_eq!(clusters.model_hash, model.model_hash());
```

A caller who already has vectors calls
`matra::extraction::semantic_clusters(&embeddings, threshold, model_hash)`
directly.

### In Python

```python
from matra import Matra, Model2Vec

m = Matra.english()
model = Model2Vec.potion_base_8m()
result = m.semantic_clusters(text, 0.8, model)
```

`model` may be any object with `embed(texts) -> list[list[float]]` and
`identity() -> str`; `matra.types.Embedder` is the protocol. A module-level
`semantic_clusters(embeddings, threshold, model_hash)` clusters vectors the
caller already holds.

There is no command-line form.

### Reading the result

- **Clusters are connected components.** If a restates b and b restates c,
  all three are one cluster even when a and c did not clear the threshold
  together. Co-membership is not pairwise similarity; the `edges` say which
  pairs actually cleared it.
- **A sentence with no edge is in no cluster.** Singletons are excluded, so
  "in no cluster" is a meaningful count.
- **The threshold is yours.** Published paraphrase cutoffs run from 0.67 to
  0.9 with no consensus, so matra has no default; every call takes it as an
  argument. An edge is kept when its score is at or above it.
- **Scores belong to one model.** Compare clusters only across results with
  the same `model_hash`.

## Reference-level explanation

### The port and its carrier

[`Embedder`][embed-port] lives in `src/embed/mod.rs`, imports only
`domain`, and requires `Send`. `embed` returns one vector per input, in
order, all of one dimension, or an error; `identity` returns a stable
identifier for the geometry, and two embedders that can disagree must not
share one. The name follows the agent-noun pattern of `Decomposer` and
`Source` (RFC-0010, decision 2).

The carrier [`Embedding`][embedding] is `pub struct Embedding(pub Vec<f32>)`
in `domain.rs`, because a port may name only domain types. It serializes as
a bare array. It is the one public type with public fields that is not
`#[non_exhaustive]`, deliberately: on a tuple struct the attribute makes
the constructor private to the crate, and an external `Embedder` must
construct these values. RFC-0010 records the departure so review does not
"fix" it.

### The static adapter

[`Model2Vec`][m2v-struct] loads `model.safetensors`, `tokenizer.json` and
`config.json`, and embeds by tokenizing, dropping the unknown token,
gathering rows of the matrix, applying optional per-token weights, mean
pooling, and normalizing when the config says so. Its identity is the
SHA-256 over the three files in order ([`Embedder` impl][m2v-embedder]).
`src/embed/model2vec.rs` is the only file that names `safetensors` and
`tokenizers`, checked by the rule 4 analog in `.semgrep/`
([EP-0014](../legacy/eps/0014-architecture-guardrails.md)). A panic in
either crate's parsing is caught and returned as `Error::ModelInvalid`
([`catch_embed_panic`][m2v-panic]). `tokenizers` is built with
`default-features = false` and `unstable_wasm`, which keeps the closure
pure Rust ([`Cargo.toml`][cargo-tokenizers]).

Static first, transformer later, for one decisive reason and three
supporting ones (RFC-0010, decision 3): a static embedding is a table gather
and a mean, with no kernel dispatch, so its vectors are bit-identical
across targets and across bindings, which lets conformance assert exact
vectors rather than tolerances. The quality cost is about ten percent
against a small transformer at a third of the size; the dependency closure
stays two crates; and a candle BERT adapter can arrive later behind the same
port. Those quality and size figures are RFC-0010's, from a survey outside
this repository, and are assumed here rather than re-measured.

How that is checked at the pinned commit:

| Property | Check |
|---|---|
| Same bits on x86_64 and aarch64 | [`vectors_are_bit_identical_across_targets`][bit-parity], against constants produced on aarch64, in the `model2vec` lane CI runs on Ubuntu and macOS ([`ci.yml`][ci-rust]) |
| Matches the Python reference implementation | [`matches_python_reference_on_pinned_inputs`][py-parity], to 1e-6, on a constructed model; the Python reference computes in float64 |
| Exact vectors for the pinned reference model | [`spec/tests/semantic/reference-model.json`][reference-model] pins the model digest and a SHA-256 over the vectors; its runner is `#[ignore]` and runs through `just conformance`, not in CI ([`justfile`][justfile-conformance]) |
| No C in the closure on wasm32 | `cargo check --no-default-features --features model2vec --target wasm32-unknown-unknown` ([`ci.yml`][ci-wasm]) |

`Model2Vec::from_dir` loads a caller's directory and never touches the
network. `Model2Vec::potion_base_8m` and `Model2Vec::from_config` provision
the pinned reference model, potion-base-8M (256 dimensions), as
[EPR-0003](0003-distribution-and-provisioning.md) describes.

### Clustering

[`semantic_clusters`][semantic-clusters] is a plain function in
`extraction/`, over domain values, so it never calls the port (boundary
rule 5) and tests without a model. It refuses a non-finite threshold, a
non-finite value, or vectors that disagree on dimension
(`InvalidInput`), and more than 2000 vectors (`InputTooLarge`,
`what = "semantic_clusters"`), the same O(n²) bound TextRank uses, imported
rather than copied. It computes every pairwise cosine, keeps the pairs at
or above the threshold, joins them with union-find (cycle-safe by
construction), and returns clusters with sorted members and edges, ordered
by their first member. A zero vector, which is what an empty sentence
embeds to, has no defined cosine and gets no edge.

[`embed_and_cluster`][embed-and-cluster] in the composition root holds both
halves: it collects the document's sentence texts, refuses more than the
cap before running the embedder, embeds, checks the embedder returned one
vector per sentence, and clusters with `embedder.identity()` as the
`model_hash`. Sentence text has already passed the pipeline's size cap in
`annotate`, so no second byte cap applies.

[`SemanticClusters`][clusters-types] carries `model_hash`, `threshold` and
`clusters`; each [`SemanticCluster`][clusters-types] carries `members` and
`edges`; each `SemanticEdge` carries `a < b` and `score`. All three are
`#[non_exhaustive]` and serialize as plain fields.

### From Python

`Matra.semantic_clusters` uses the built-in adapter directly when given a
`Model2Vec`, and otherwise wraps the object in [`PyEmbedder`][py-embedder],
which reads `identity()` once at construction (so one result cannot carry
two identities), calls `embed` under the GIL, narrows each float to `f32`,
and turns a Python exception or a malformed return into
`Error::InvalidInput`, which raises `ValueError`. The wrap happens before
the parse, so an object that cannot name its geometry costs one method
call, not a document's parse ([`semantic_clusters`][py-semantic]).

### No longer in force

- **"No network in the library" for the embedding model.**
  [EP-0009](../legacy/eps/0009-embeddings-adapter.md) and RFC-0010 decision
  6 as first written. [RFC-0011](../legacy/rfcs/0011-out-of-the-box.md)
  amended it to "no unpinned network"; the reference model now downloads
  from pinned URLs against a pinned digest
  ([EPR-0003](0003-distribution-and-provisioning.md)).
- **A configured default threshold.** EP-0009's surface sketch and
  RFC-0011 gave `Config` a semantic threshold.
  [RFC-0020](../legacy/rfcs/0020-deprecate-unread-config-keys.md) deprecated
  it because no call read it; every clustering call takes the threshold as
  an argument.
- **`semantic_clusters` taking the sentence slice.** EP-0009's first sketch
  passed the sentences too; RFC-0010's M5 amendment removed them, since only
  their count was read, and moved the length check to
  `embed_and_cluster`.

## Drawbacks

- **A quality ceiling, chosen.** Paraphrases a transformer would catch near
  the threshold are missed (RFC-0010; the size of the gap is assumed from
  its survey).
- **`Embedding` is public surface** although most callers only see
  `SemanticClusters`, because the port needs a domain carrier.
- **The format is a third party's.** A change to the model2vec artifact
  format would surface as a digest mismatch or a load failure, not
  silently, but would need adapter work.
- **The exact reference-model check is not in CI.** The bit-identity test
  in CI uses a constructed model; drift specific to the real artifact would
  show only when someone runs `just conformance`.
- **Quadratic in sentences.** Clustering compares every pair, which is why
  the 2000-sentence cap exists.

## Rationale and alternatives

These are the alternatives RFC-0010 and EP-0009 weighed.

- **A candle BERT adapter first**: viable and verified for wasm32, but
  kernel dispatch costs bit-parity and the closure is a whole inference
  stack. Designated as the second adapter, behind the same port.
- **Depending on the existing model2vec Rust crate**: rejected for its
  closure (command-line dependencies as library dependencies, version skew,
  licence metadata that trips scanning). The adapter is matra's own.
- **ONNX Runtime (`ort`, fastembed)**: C FFI, which would close the wasm32
  path for every downstream caller.
- **A feature named `embeddings`**: rejected because a feature gates an
  adapter and so carries the adapter's name, on the `udpipe` precedent; a
  capability name would collide with the next adapter's flag.
- **Cliques instead of connected components**: would split chained
  restatement, which is the pattern callers look for. The cost of
  components, transitive co-membership, is made visible by the edges.
- **Semantic results as fields on `Document`**: rejected by decision 1,
  above; it would put model output behind a surface whose every other value
  a caller can verify.

Could this live in a caller's code? The clustering could, and the
vectors-in function makes that easy. matra ships it so the threshold, the
provenance and the component semantics are one implementation across Rust
and Python.

## Prior art

- model2vec (Minish Lab) defines the static embedding format and publishes
  potion-base-8M; its Python implementation is the parity reference.
- Union-find connected components over a thresholded similarity graph is
  the standard construction; matra's TextRank already builds a sentence
  similarity graph of the same size.
- [RFC-0010](../legacy/rfcs/0010-embeddings-adapter.md) and
  [EP-0009](../legacy/eps/0009-embeddings-adapter.md) hold the full history.

## Unresolved questions

None. The open questions about the embedding model's licence and its
silent first run are recorded in
[EPR-0003](0003-distribution-and-provisioning.md), where provisioning
lives.

## Future possibilities

- A transformer adapter behind the same port, with a feature named for its
  backend, when a caller shows the static model's ceiling costs them
  clusters that matter.
- A command-line form, which would need the threshold as an argument, since
  configuration does not select it.
- The same port used by a binding for TypeScript, which the wasm32 check
  keeps open for the embedding half.

[embed-port]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/embed/mod.rs#L1-L43
[embedding]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L78-L93
[clusters-types]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/domain.rs#L95-L145
[m2v-struct]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/embed/model2vec.rs#L115-L140
[m2v-embedder]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/embed/model2vec.rs#L598-L606
[m2v-panic]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/embed/model2vec.rs#L1186-L1209
[py-parity]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/embed/model2vec.rs#L1490-L1542
[bit-parity]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/embed/model2vec.rs#L1544-L1580
[semantic-clusters]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/extraction/semantic.rs#L1-L155
[embed-and-cluster]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L360-L399
[py-semantic]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L601-L636
[py-embedder]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/src/lib.rs#L890-L981
[cargo-tokenizers]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/Cargo.toml#L61-L67
[reference-model]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/spec/tests/semantic/reference-model.json#L1-L37
[justfile-conformance]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/justfile#L166-L174
[ci-rust]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/.github/workflows/ci.yml#L18-L68
[ci-wasm]: https://github.com/mox-labs/matra/blob/4fcfb4adc85524243c5e392becd4f490e3e42252/.github/workflows/ci.yml#L102-L120
