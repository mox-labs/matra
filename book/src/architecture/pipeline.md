# The pipeline

Text moves through five stages. The names of those stages were chosen carefully: each is honest about what it does, and none collides with vocabulary reserved for future use. The naming was settled in ADR-0002 after rejecting two alternatives whose names created semantic problems.

**ingest → decompose → parse → measure** (with **extract** as a peer to measure)

## The five verbs and what maps to them

**ingest** is what `Source::read` does. A `FileSource` or `DirectorySource` reads bytes from disk and returns `Vec<RawDocument>`. Before reading, it checks file size against `MAX_INPUT_BYTES` (8 MiB) and rejects symlinks. The 8 MiB ceiling is not arbitrary: UDPipe's per-token allocations cross approximately 1 GiB resident at that input size on a typical workstation. The gate fires before any bytes hit the NLP provider. If the input is text passed directly to `analyze()` or `parse()`, the same check runs: `check_input_size` is the first operation in every public entry point.

**decompose** is what `Decomposer::decompose` does. A `MarkdownDecomposer` or `PlainTextDecomposer` takes a `&str` and returns `Vec<Section>`. Each `Section` has an optional heading, a depth level, and a list of `Paragraph` entries. The decomposer also sets `in_blockquote = true` for paragraphs inside blockquote blocks; those paragraphs are skipped by the parse and measure stages. Decompose is infallible: malformed input is interpreted as best-effort, not returned as an error.

**parse** is what `NlpProvider::parse` does. The UDPipe adapter takes a `&str` and returns `Vec<Sentence>`. Each `Sentence` contains its verbatim text and an ordered `Vec<Token>`, where every `Token` carries the full CoNLL-U annotation: lemma, POS tag (`pos`), language-specific POS tag (`xpos`), morphological features (`feats`), head position, dependency relation (`dep`), enhanced dependency graph (`deps`), and a derived `is_punct` flag. This is the stage that turns text into the queryable structure that the four faces read.

**measure** is what the metric suite does. Four metric functions run in sequence: Flesch-Kincaid grade per paragraph, lexical density per paragraph, vocabulary TTR and nominalization ratio over the whole document (using the flat sentence slice), and brotli compression ratio per paragraph. Metrics write into the `Option<f64>` slots on `Paragraph` and `Document`; a `None` slot means the metric's threshold was not met (e.g., a paragraph with fewer than 11 words does not receive a readability grade). Measure is an aggregation operation: it produces scalars, not selections.

**extract** is what the extraction functions do: `tfidf_summarize`, `textrank_summarize`, `rake_keyphrases`, `yake_keyphrases`. These are called by the consumer directly; they are not part of the `analyze()` call path. Extract is a selection operation: it returns ranked subsets of sentences or phrases, not aggregated scalars.

## Why measure and extract are peers, not nested

Measure produces aggregations: a scalar grade, a ratio, a count-derived number. Extract produces selections: a ranked list of sentences, a ranked list of phrases.

These are ontologically different operations. Coupling them into a single post-parse stage would mean that every call to `analyze()` also runs TextRank's O(n^2) sentence-similarity matrix, even when you only need a readability grade. A document with 500 sentences would force a 500x500 similarity matrix on every `analyze()` call, whether or not you ever asked for a summary.

The peer relationship lets consumers call measure via `analyze()` and extract independently. The typical pattern for consumers that need both:

```rust
let sentences = vaani::parse(text, &nlp)?;
let summary = vaani::extraction::tfidf_summarize(&sentences, 3)?;
let phrases = vaani::extraction::rake_keyphrases(&sentences, 10)?;
```

Parse once, pass to multiple consumers. The `parse()` free function in `src/lib.rs` exists precisely for this pattern. The `analyze_from()` function supports the corresponding case: pre-decompose, pre-parse, then run the measure suite over the pre-parsed sentences without double-parsing.

## Per-paragraph parse, not whole-document

The previous implementation joined all paragraphs into a single string, parsed once, then tried to wire parsed sentences back to their originating paragraphs by matching sentence-text prefixes against paragraph text. This approach produced three documented defects.

**Prefix-collision (FM1).** If two paragraphs shared their first characters ("The system processes input now. Tail one." and "The system processes input now. Tail two."), the prefix-match wiring could assign both paragraphs' first sentence to the same paragraph and silently drop the other. The regression test `parse_per_paragraph_scopes_sentences_to_originating_paragraph` in `src/lib.rs` demonstrates this: two paragraphs with identical prefixes each retain their distinct tail sentences only with the per-paragraph approach.

**Inner-substring theft (FM2).** If paragraph A contained paragraph B's first-sentence prefix as a mid-text substring, the greedy prefix match could pull B's sentence into A. The regression test `parse_per_paragraph_no_inner_substring_theft` demonstrates this with "Outer talks about the special phrase processes input now" as paragraph A and "The special phrase processes input now. End B." as paragraph B; paragraph B's "End B" sentence must stay in B.

**Empty paragraph handling (FM3).** A trailing whitespace on an empty paragraph confused the prefix-match wiring. With the per-paragraph approach, the empty paragraph receives an empty `sentences` vec and the next paragraph's parse is entirely independent.

The current implementation (`run_analysis` in `src/lib.rs`) calls `nlp.parse(&para.text)` for each non-blockquote paragraph individually and attaches the resulting sentences directly to that paragraph via `para.sentences = parsed`. There is no wiring step, no prefix matching, no ambiguity. Each paragraph's sentences come from exactly one parse call on exactly that paragraph's text.

The cost is multiple NLP parse calls instead of one. The benefit is correctness by construction: the relationship between a paragraph and its sentences is established at parse time, not recovered by heuristic matching after the fact.

## Bounds that travel with the pipeline

Every stage that can be computationally abusive has a cap. Each cap fires `Error::InputTooLarge` with a `what` discriminant that names which gate triggered, so the caller can route errors specifically:

| Stage | Cap constant | Value | `what` discriminant |
|---|---|---|---|
| ingest (file) | `MAX_INPUT_BYTES` | 8 MiB | `"file_source"` |
| ingest (text) | `MAX_INPUT_BYTES` | 8 MiB | `"input"` |
| extract (TF-IDF) | `MAX_SENTENCES` | 2,000 | `"tfidf"` |
| extract (TextRank) | `MAX_SENTENCES` | 2,000 | `"textrank"` |
| extract (RAKE) | `MAX_TOKENS` | 200,000 | `"rake"` |
| extract (YAKE) | `MAX_TOKENS` | 200,000 | `"yake"` |

TextRank's 2,000-sentence cap is driven by its O(n^2) similarity matrix: at 2,000 sentences the matrix is approximately 32 MB of `f64`. TF-IDF shares the number but has a different cost model: its working set is bounded by `HashMap` entries proportional to total content tokens, not a quadratic matrix. The constants are kept separate intentionally (a Chesterton fence: a future TextRank tuning should not silently change TF-IDF behavior).

RAKE's cap is on total token count across all sentences (200,000), not on sentence count, because its worst-case is proportional to total unique phrase candidates, which is bounded by token count times mean candidate-phrase length. YAKE shares the same cap for the same reason.

See [Ports and adapters](./ports-adapters.md) for the trait contracts. See [Errors](../reference/errors.md) for the full error variant reference.
