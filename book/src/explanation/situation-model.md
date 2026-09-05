# Situation model

## What matra is for

matra illuminates the internal structural makeup of text. Text goes in; a typed tree, a set of measures, and a set of ranked extractions come out. Everything it returns is a value your code can read, store, and reason over.

It is a substrate. The higher-order reasoning is built on top of it, by you.

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

The reason for the split is reuse. A verdict is fitted to one purpose and is dead weight to the next consumer; the arc it was computed from is not.

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

**Models are supplied, not bundled.** The UDPipe English model is downloaded on the first call to `Udpipe::english` (or `Matra.english` in Python) into a directory you name, then verified against a pinned SHA-256. A cached file that fails verification is removed and downloaded again. `Udpipe::from_path` loads a model you already have and touches no network.

**Embedding models are placed by hand.** `Model2Vec::from_dir` reads `model.safetensors`, `tokenizer.json`, and `config.json` from a directory. It never downloads. The SHA-256 over those three files is the model identity that every derived result carries.

**Verification and load share one read.** Bytes are hashed in memory and the model is loaded from those same bytes. Nothing re-reads the disk between the two.

That first-use UDPipe download is the only network access anywhere in the library.
