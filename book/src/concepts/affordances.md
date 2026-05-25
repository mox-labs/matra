# What vaani offers

> vaani illuminates the internal structural makeup of text, enabling effective higher-order reasoning on text.

This page maps what vaani gives you today and what is planned. The NLP concepts behind each capability have their own pages; this page is the inventory.

---

## Structured parse ✅

vaani parses every token in a document and returns a typed hierarchy: Document > Section > Paragraph > Sentence > Token.

Each `Token` carries:
- **lemma** (the dictionary form of the word)
- **pos** (part of speech from the 17-tag Universal Dependencies set)
- **dep** (the dependency relation to its head: `nsubj`, `obj`, `nsubj:pass`, `aux:pass`, and so on)
- **head** (the id of the governing token; 0 means the token is the root of its sentence)

The structure is the same whether you call the Rust API, the Python bindings, or the CLI. Every field is stable, serializable, and traversable. If your application needs to ask "who is the agent in this clause?" or "does this sentence carry a passive construction?", those answers are one field lookup, not a pattern match over raw text.

[POS tags and lemmatization](./pos-lemmas.md) explains the Universal POS set and what lemmatization does. [Dependency parsing](./dependency-parsing.md) explains what `dep` and `head` mean structurally.

---

## Document metrics ✅

Six metrics computed over the parsed hierarchy. Each is an instrument, not a judgment. What the number means for your application depends on your domain, your audience, and your threshold choices. vaani measures; your application decides.

**Readability grade** (Flesch-Kincaid, per paragraph). A number. Grade 8 means an eighth-grader can follow it; grade 16 means graduate-level complexity. The formula measures sentence length and syllable load. It does not measure writing quality, clarity, or correctness.

**Lexical density** (per paragraph). The fraction of words that are content words (not stop words). Dense = information-heavy prose. Sparse = procedural or conversational register. vaani measures this; your system decides what that means for your use case.

**Vocabulary TTR** (type-token ratio, document-level). Unique lemmas divided by total lemmas. A proxy for lexical variety. It is sensitive to text length: longer texts will naturally repeat more and score lower, so compare TTR values only across texts of similar length.

**Nominalization ratio** (document-level). The fraction of noun tokens that end in a nominalizing suffix (the suffixes are: -tion, -ment, -ness, -ity, -ence, -ance). A high ratio is a surface signal of nominalization-heavy prose. It is a heuristic: "tion"-class endings can appear in non-nominalized nouns (nation, station, portion).

**Passive ratio** (computed from the dependency parse). The fraction of sentences that contain a passive-voice construction. Detection is dependency-label-based: any sentence with a token carrying `dep = "nsubj:pass"`, `"nsubjpass"`, or `"aux:pass"` is counted as passive.

**Compression ratio** (brotli, per paragraph). The ratio of compressed size to original size. A low ratio means the paragraph compresses well, which correlates with high surface repetition. Useful as a rough proxy for lexical redundancy. Applied only to paragraphs over 50 words; skipped for paragraphs over 256 KiB.

[Readability](./readability.md) explains the Flesch-Kincaid formula in detail, including its limits. [Passive voice and nominalization](./passive-nominalization.md) explains how the dependency-label detection works and what it does not claim.

---

## Summarization ✅

Two extractive algorithms. Both select and return sentences from the original text; neither generates new sentences.

**TF-IDF summarization.** Scores each sentence by how distinctively its terms appear relative to all other sentences. Good for coverage: the top-scored sentences together mention the widest range of the document's key terms. If your application surfaces document excerpts for human review, TF-IDF gives you the sentences that span the document's full vocabulary.

**TextRank summarization.** Builds a similarity graph over sentences and runs a PageRank-style ranking. Good for coherence: highly scored sentences tend to use language shared with many other sentences, making them central to the document's argument. If your application needs to route a document to a handler or identify its core claim, TextRank gives you the sentences the rest of the document echoes.

Both are capped at 2000 input sentences. Above that, they return an error rather than silently produce degraded results.

[TF-IDF and TextRank](./tfidf-textrank.md) explains the algorithm ideas, what each is optimized for, and why the cap exists.

---

## Keyphrase extraction ✅

Two algorithms. Both return a ranked list of phrases with scores.

**RAKE.** Rule-based. Finds noun-phrase candidates (runs of NOUN, ADJ, and PROPN tokens split at stop words) and scores them by a co-occurrence ratio. Fast and deterministic.

**YAKE.** Statistical. Scores individual terms by position, frequency, and context diversity, then builds 1-to-3-word candidates. Better at finding statistically unusual phrases; slower than RAKE.

Both are capped at 200,000 input tokens.

[RAKE and YAKE](./rake-yake.md) explains the algorithm ideas, the tradeoffs between them, and when to choose one over the other.

---

## HTML report ✅

`Document::to_html_report()` (Rust), `Vaani.report(text, format="html")` (Python), and `vaani report essay.md --format html` (CLI) produce an HTML summary of the full analysis. The report is the visual inspection surface: readable in a browser, renderable inline in Jupyter, usable as supplementary material in a paper.

[HTML report reference](../reference/html-report.md) documents the exact method signatures and output format.

---

## Rule evaluation 🛠️

Planned v0.2+. Rule evaluation over the parsed structure will enable: relation extraction, schema extraction, modality detection, and speech act classification. It is the capability that closes the gap between vaani's parse output and structured interpretation of text.

Rule evaluation lands in a later iteration. The dependency parse and metrics ship today and are the substrate it builds on.

See [future direction](../architecture/future-direction.md) for the planned scope and trigger conditions.

---

## What vaani is not

vaani measures structure. It does not judge. The passive ratio is a number; whether passive constructions are appropriate in a given text is a decision for your application. The readability grade is a formula output; whether grade 12 is too complex depends on your audience. No metric vaani produces carries an inherent quality judgment.

vaani is not a generative NLP system. It produces structured output from existing text; it does not write, summarize in the generative sense, or predict next tokens. The summarization algorithms are extractive: they select sentences, they do not compose them.

The distinction between vaani and transformer-based systems is architectural. Transformers learn statistical associations over large corpora and produce embeddings or generated text. vaani runs a deterministic grammar-based parse and returns the structural output. Both are useful; they answer different questions.
