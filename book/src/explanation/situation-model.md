# Situation model

## What matra is for

matra reports the structure of text and measurements over it. Text goes in; a typed tree, a set of measures, and a set of ranked extractions come out. Everything it returns is a value your code can read, store, and reason over.

It is a library, not an application. The analysis on top of it is yours to write.

## What it reports and what it leaves to you

matra reports `passive_ratio = 0.4`. Whether 0.4 is too passive for the document at hand is your call, and the answer differs between a statute, a lab report, and a landing page.

That split runs through every tier:

| matra reports | Your code decides |
|---|---|
| a sentence carries `nsubj:pass` | whether the missing agent matters here |
| `reportings` names a verb governing a `ccomp` | whether that verb is evidential and whether the source is credible |
| `hearst_pairs` names a construction that conventionally signals hypernymy | whether the hypernymy relation actually holds |
| `modals` names `must` on an `aux` arc | whether that `must` is obligation or inference |
| a `SemanticCluster` groups sentences whose vectors cleared your threshold | whether those sentences say the same thing |

The reason for the split is reuse. A judgment is fitted to one purpose and is dead weight to the next caller; the arc it was computed from is not.

## Where it sits in a pipeline

Upstream of interpretation, downstream of ingestion.

```text
files or strings  ->  matra  ->  rule evaluation
                                 scoring and dashboards
                                 retrieval and chunking
                                 LLM prompt construction
```

Two properties make it usable that far downstream. Every value is serde-serializable, so the parse crosses into Python as a plain dict and can be stored as JSON. And token ids are sentence-scoped while paragraph and sentence text are verbatim, so a value can always be traced back to the bytes it came from.

## What it needs from the environment

**Nothing has to be set up first.** `Engine::with_defaults()` in Rust, `Matra.english()` in Python, and any `matra` command resolve where models live and fetch what is missing. The directory comes from `MATRA_MODEL_DIR`, else your config file, else the `models` subdirectory of `$XDG_DATA_HOME/matra`. [Programming model](programming-model.md#configuration) has the resolution order.

**Models are fetched, not bundled, and only against a pinned digest.** The UDPipe English model arrives on the first call to `Udpipe::english`, `Udpipe::from_config` or `Matra.english`; the reference embedding model arrives on the first call to `Model2Vec::potion_base_8m` or `Model2Vec::from_config`. Both fetch from URLs written in the source and load nothing whose SHA-256 does not equal a constant written beside them. A file that fails verification is removed and fetched once more, and a second failure raises. Exactly one artifact set can arrive this way for each.

**Bringing your own is a first-class path.** `Udpipe::from_path` and `Model2Vec::from_dir` load what you supply and touch no network, whatever the directory holds. For those, the digest is identity rather than proof: it says which artifacts produced a value, not that they are the ones you meant to fetch.

**Verification and load share one read.** Bytes are hashed in memory and the model is loaded from those same bytes. Nothing re-reads the disk between the two.

Those pinned model fetches are the only network access anywhere in the library.
