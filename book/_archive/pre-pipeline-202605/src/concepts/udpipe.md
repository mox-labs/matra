# UDPipe and CoNLL-U

🛠️ This page is a stub. Full content lands in a follow-up iteration.

vaani's structured parse is produced by UDPipe, a trained NLP pipeline from the Institute of Formal and Applied Linguistics at Charles University. UDPipe processes raw text and produces CoNLL-U formatted annotation: one row per token, with fields for surface form, lemma, part-of-speech tag, dependency relation, and head position.

---

## What UDPipe is

UDPipe is a statistical NLP pipeline trained on Universal Dependencies treebanks. It performs tokenization, POS tagging, morphological analysis, and dependency parsing in a single pass. vaani's `udpipe` feature flag wraps UDPipe behind the `NlpProvider` port trait. The wrapper catches panics at the FFI boundary so that malformed input cannot abort the host process.

The model file is fixed at load time. vaani pins the model by SHA-256 and verifies the hash before using the model. See [tutorials/installation.md](../tutorials/installation.md) for the download procedure.

---

## CoNLL-U format

CoNLL-U is the standard annotation format used by the Universal Dependencies project. Each token in a sentence occupies one row with ten tab-separated fields:

| Field | Name | Example |
|---|---|---|
| 1 | ID | 3 |
| 2 | FORM (surface form) | approved |
| 3 | LEMMA | approve |
| 4 | UPOS (universal POS) | VERB |
| 5 | XPOS (language-specific POS) | VBD |
| 6 | FEATS (morphological features) | Mood=Ind|Tense=Past|VerbForm=Fin |
| 7 | HEAD (position of head token) | 0 |
| 8 | DEPREL (dependency relation) | root |
| 9 | DEPS (enhanced dependencies) | _ |
| 10 | MISC | _ |

vaani maps CoNLL-U fields to its `Token` domain type. See [reference/domain-types.md](../reference/domain-types.md) for the Rust field names.

---

## Model SHA pinning

vaani records the SHA-256 of the UDPipe model file in `src/nlp/udpipe.rs` as `ENGLISH_MODEL_SHA256`. The hash is verified at load time via `read_and_verify`. If the model file does not match the expected hash, vaani returns an error rather than loading a potentially substituted file.

For reproducibility: record the vaani crate version and the model SHA when publishing results derived from vaani's parse. The `scripts/fetch-model-hash.sh` script updates the constant when the model version changes.

---

## Planned for this page

A follow-up iteration will add:

- The Universal Dependencies relation label inventory (with examples from vaani's actual parse output)
- An explanation of how UDPipe's tokenization interacts with vaani's paragraph-level parsing
- Guidance on what happens when UDPipe cannot parse a sentence (the error path and recovery behavior)

See [dependency-parsing.md](./dependency-parsing.md) for what the dependency relations mean and how to read a dependency tree.
