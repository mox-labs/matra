# UDPipe and CoNLL-U

vaani uses UDPipe to parse text into structured annotations. This page explains what UDPipe is, what CoNLL-U is, and how vaani loads and verifies the model.

---

## What UDPipe is

UDPipe is an NLP pipeline from the Institute of Formal and Applied Linguistics at Charles University (Prague). It reads raw text and produces tokenized, lemmatized, POS-tagged, and dependency-parsed output in CoNLL-U format. The core is a trained statistical model: the model was built from treebanks annotated by linguists and learns to assign the most likely structural analysis to new text.

The model vaani ships with is `english-ewt-ud-2.5-191206.udpipe`: the English Web Treebank (EWT) corpus, Universal Dependencies version 2.5, released 2019-12-06. EWT was built from web text including weblogs, news, reviews, and social media, which gives it broad coverage of everyday written English.

spaCy and Stanza are comparable tools in the Python ecosystem. Both produce dependency parses and POS tags; both have their own training pipelines and model formats. vaani uses UDPipe because the `udpipe-rs` bindings give it a zero-Python-overhead parse path that is consistent across the Rust core and the Python/WASM crusts. The parse output is identical regardless of which language you call from.

---

## What CoNLL-U is

CoNLL-U is a tab-separated text format for annotated sentences. Each token occupies one line; each line has ten columns:

```
# text = The committee approved the proposal.
1   The         the         DET     DT   Definite=Def|PronType=Art   2   det     _   _
2   committee   committee   NOUN    NN   Number=Sing                  3   nsubj   _   _
3   approved    approve     VERB    VBD  Mood=Ind|Tense=Past|...      0   root    _   _
4   the         the         DET     DT   Definite=Def|PronType=Art   5   det     _   _
5   proposal    proposal    NOUN    NN   Number=Sing                  3   obj     _   _
6   .           .           PUNCT   .    _                            3   punct   _   _
```

The columns are: id, surface form, lemma, UPOS tag, XPOS tag, morphological features, head id, dependency relation, enhanced deps, and misc annotations.

This is the data behind every structured operation vaani supports. When vaani reports that "committee" is the `nsubj` of "approved," the raw record is row 2 of this table: head id 3 (pointing to "approved"), dependency relation `nsubj`. Nothing is inferred after the parse; the table is the parse.

vaani's `Token` struct preserves all ten columns. The fields you will use most often are `pos` (the UPOS tag), `dep` (the dependency relation), and `head` (the id of the governing token). Head id 0 means the token is the root of its sentence.

[POS tags and lemmatization](./pos-lemmas.md) covers the UPOS tags. [Dependency parsing](./dependency-parsing.md) explains the head-dependent structure.

---

## Model identity and SHA pinning

The exact model vaani loads is fixed in the source:

- **File:** `english-ewt-ud-2.5-191206.udpipe`
- **Size:** 16,309,608 bytes
- **SHA-256:** `784bd0fa85e3d831fd02a55290d0acfd05c953159dc38cc33d52e1b28add9957`

Before loading, vaani checks the file size (fast-fail) and then computes the SHA-256 of the bytes. The bytes that pass the hash check are the same bytes passed directly to the model loader. There is no second disk read between verify and load, which closes the window where a file swap could substitute a different model.

This matters for anyone whose work depends on reproducible output: a fixed model hash means the same input text always produces the same parse. If you run vaani on a document today and again in six months, you get the same tokens, lemmas, POS tags, and dependency labels, because the model has not changed. When the model does change, the hash constant in `src/nlp/udpipe.rs` must be updated explicitly, making the change visible in source control and in any system that pins its dependency.

If the cached file fails the hash check, vaani deletes it and re-downloads once. If the re-downloaded file also fails, the load fails with an error. A file that mismatches the pinned hash is treated as untrusted.

---

## Atomic download and loading

The first call to `Vaani.english(model_dir)` downloads the model (~16 MB) if it is not already cached. Download writes to a per-process temporary subdirectory, then atomically renames the file into place. Concurrent processes downloading to the same directory do not corrupt each other; each writes to its own `.tmp.download.<pid>` subdirectory.

The Python CLI (`python -m vaani`) performs the download automatically on first use. Subsequent calls load from the cached file with no network access.

---

## What UDPipe does not provide

UDPipe is a statistical parser, not a semantic one. It does not perform named entity recognition, coreference resolution, or semantic role labeling. The output is syntactic structure: grammatical relations and morphological features. What those relations mean for the content of a text is for your application to determine.

The model is also English-only in vaani's current configuration. Multi-language support would require additional models and is not in the current scope.
