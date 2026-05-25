# Methodology

Formulas, definitions, and explicit non-claims for every metric vaani ships. This page is the single source of truth for the mathematics and the scope claims. If a page elsewhere in the docsite cites a formula, it links here.

---

## How to use this page

Each metric section has three parts:

1. **Formula**: the exact computation vaani applies.
2. **Definition**: what the output value means.
3. **Non-claims**: what the metric explicitly does NOT measure or indicate.

The non-claims are not caveats. They are part of the specification. vaani measures; your application decides. Knowing what a metric does not mean is required for using it correctly.

---

## Readability: Flesch-Kincaid

### Flesch Reading Ease

```
FRE = 206.835 - 1.015 * (words / sentences) - 84.6 * (syllables / words)
```

**Definition:** A score on a 0-100 scale. Higher scores indicate material that is easier to read. A score of 60-70 corresponds roughly to plain English; a score below 30 corresponds to professional and academic text.

**Non-claims:**

- FRE does not measure comprehension, clarity, or writing quality. A technically simple text can score high while being poorly organized or misleading.
- FRE does not account for domain vocabulary. A document full of short technical terms may score high while being incomprehensible to a non-specialist.
- FRE syllable counting is an approximation. vaani uses a rule-based syllable counter; edge cases in English morphology produce rounding differences.
- FRE was calibrated on general English prose. Application to non-English text or non-prose (code, legal citations, structured data) produces outputs outside the intended interpretation range.

### Flesch-Kincaid Grade Level

```
FKGL = 0.39 * (words / sentences) + 11.8 * (syllables / words) - 15.59
```

**Definition:** An approximate U.S. school-grade reading level. A value of 8.0 suggests the text is readable by a typical 8th-grade student.

**Non-claims:**

- Same as FRE: grade level is not a measure of quality, appropriateness, or audience fit.
- Grade level can be gamed by using short sentences with long words or vice versa; the metric does not detect that pattern.

---

## Lexical Density

```
lexical_density = lexical_tokens / total_tokens
```

Where `lexical_tokens` are tokens with POS tags in {NOUN, VERB, ADJ, ADV}. `total_tokens` includes function words, punctuation, and all other tokens.

**Definition:** The proportion of content-bearing words in the text. Higher values indicate a more information-dense text; lower values indicate more grammatical scaffolding relative to content.

**Non-claims:**

- Lexical density does not measure information value or semantic richness.
- The distinction between lexical and grammatical tokens depends on the POS tagger's accuracy. vaani uses UDPipe's output; tagging errors affect this metric.
- High lexical density does not imply good writing; academic texts frequently have high lexical density that is appropriate for expert readers.

---

## Vocabulary Type-Token Ratio (TTR)

```
TTR = unique_lemmas / total_lemmas
```

**Definition:** A measure of lexical diversity. A TTR of 1.0 means every word is used exactly once. A lower TTR indicates more repetition.

**Non-claims:**

- TTR is length-sensitive. As document length increases, TTR naturally decreases because common words accumulate. TTR values are not directly comparable across documents of different lengths.
- TTR does not measure vocabulary sophistication or domain coverage.
- vaani uses lemma-based TTR (not surface-form TTR). "runs," "run," and "running" count as one lemma. This reduces TTR's sensitivity to morphological variation.

---

## Nominalization Ratio

```
nominalization_ratio = nominalized_nouns / total_nouns
```

Where `nominalized_nouns` are nouns matching morphological patterns associated with verbal or adjectival derivation (e.g., suffixes `-tion`, `-ment`, `-ness`, `-ity`, `-ance`, `-ence`).

**Definition:** An approximation of the degree to which the text uses noun forms derived from verbs and adjectives. Higher values indicate heavier nominalization, which correlates with formal and bureaucratic registers.

**Non-claims:**

- The suffix heuristic produces false positives (e.g., "station," "government," "distance" match the patterns but are not nominalizations in the derivational sense).
- The heuristic produces false negatives for nominalization patterns not in the suffix list.
- Nominalization ratio is not a measure of clarity, formality, or writing quality. Whether nominalization is appropriate depends entirely on the genre and audience.
- The ratio uses POS tags from UDPipe; POS errors affect which tokens are counted as nouns.

---

## Passive Ratio

```
passive_ratio = passive_sentences / total_sentences
```

Where `passive_sentences` are sentences containing at least one token with dependency relation `aux:pass`.

**Definition:** The proportion of sentences in the document that contain a passive construction. Passive voice is detected via the `aux:pass` dependency label in UDPipe's CoNLL-U output.

**Non-claims:**

- Not all passive constructions are detected by `aux:pass`. Truncated passives ("the proposal was rejected") may or may not carry an `aux:pass` arc depending on UDPipe's parse; this varies by sentence complexity.
- Not all `aux:pass` arcs indicate what is conventionally understood as "passive voice" in all genres.
- Passive ratio does not indicate whether passive voice is appropriate, excessive, or intentional. vaani detects; your application judges.
- The metric counts sentences with at least one passive construction; a single passive auxiliary in a long complex sentence counts the same as a simple passive sentence.

---

## Brotli Compression Ratio

```
compression_ratio = compressed_bytes / original_bytes
```

Where `compressed_bytes` is the size of the text after Brotli compression (quality 11) and `original_bytes` is the UTF-8 byte length of the original text.

**Definition:** A proxy for information-theoretic repetitiveness. More repetitive text compresses more; a lower compression ratio indicates higher redundancy. A compression ratio near 1.0 indicates high entropy (little repetition or predictable structure).

**Non-claims:**

- Compression ratio is an information-theoretic proxy, not a direct measure of any linguistic property.
- Compression ratio is length-sensitive. Short texts compress poorly regardless of content; very long texts compress more due to Brotli's dictionary mechanics.
- A low compression ratio does not indicate poor writing. Highly structured or formulaic text (legal documents, technical specifications) compresses well precisely because its structure is predictable.

---

## Summarization: TF-IDF

```
sentence_score(s) = sum over terms t in s of (
    tf(t, document) * log(N / df(t))
)
```

Where `tf(t, document)` is the frequency of term `t` in the document, `N` is the total number of sentences, and `df(t)` is the number of sentences containing `t`.

**Definition:** Each sentence is scored by the sum of TF-IDF weights of its constituent terms. The top-N sentences by score are returned as the extractive summary.

**Non-claims:**

- TF-IDF summarization selects sentences, not the most informative content. A sentence may score high because it repeats frequent terms, not because it is the most important sentence.
- TF-IDF treats words as independent; it does not capture semantic relationships between terms.
- Capped at `MAX_SENTENCES = 2000` input sentences.

---

## Summarization: TextRank

TextRank builds a weighted graph over sentences, where edge weight between two sentences is their cosine similarity over TF-IDF vectors. The algorithm runs PageRank-style iteration to assign an importance score to each sentence. Top-N sentences by score are returned.

**Definition:** A graph-coherence-based extractive summarization. TextRank tends to favor sentences that are similar to many other sentences in the document, producing summaries that represent the dominant topics.

**Non-claims:**

- TextRank does not detect topic shifts or narrative structure. A document with a clear argument progression may produce summaries that miss the conclusion because early sentences cluster more densely.
- TextRank is deterministic: same input produces same output. It does not incorporate randomness or model-based relevance.
- Capped at `MAX_SENTENCES = 2000`.

---

## Keyphrase Extraction: RAKE

RAKE (Rapid Automatic Keyword Extraction) splits text at stop words and sentence boundaries, scores candidate phrases by the sum of word scores (where word score = degree / frequency in phrase co-occurrences), and returns the top-N phrases by score.

**Definition:** A fast, rule-based keyphrase extraction algorithm. RAKE favors phrases that co-occur frequently and are not separated by stop words.

**Non-claims:**

- RAKE does not use semantic similarity or document context beyond co-occurrence statistics.
- RAKE performance degrades on very short texts (insufficient co-occurrence data).
- Capped at `MAX_TOKENS = 200_000` input tokens.

---

## Keyphrase Extraction: YAKE

YAKE (Yet Another Keyword Extractor) scores candidate terms using five statistical features: position in document, frequency, co-occurrence with context words, sentence diversity, and casing. Lower YAKE scores indicate higher keyphrase quality (scores are costs, not gains).

**Definition:** A statistical keyphrase extraction algorithm with positional weighting. YAKE favors terms that appear early in the document, are contextually distinctive, and are not too common.

**Non-claims:**

- YAKE's positional weighting assumes document-level importance correlates with early position. This assumption does not hold for documents where the introduction is generic (e.g., legal boilerplate followed by specific claims).
- YAKE scores are not comparable across documents of different lengths; they are relative within a document.
- Capped at `MAX_TOKENS = 200_000`.

---

## Model and reproducibility

vaani uses UDPipe with a pinned English model. The SHA-256 of the model file is verified at load time. Providing the same model file and the same input text produces the same output.

The model hash is in `src/nlp/udpipe.rs` (`ENGLISH_MODEL_SHA256` constant). The `scripts/fetch-model-hash.sh` script refreshes this constant when the model version changes. For reproducibility in published research, record the `vaani` crate version and the model SHA alongside your results.

See [installation](../tutorials/installation.md) for the model download procedure.

---

## Planned for this page

The content-rewrite iteration after this restructure will add:

- **Original-paper citations** inline at each metric and algorithm. Specifically: Flesch (1948) and Kincaid et al. (1975) for readability; Salton and McGill (1983) for TF-IDF; Mihalcea and Tarau (2004) for TextRank; Rose et al. (2010) for RAKE; Campos et al. (2020) for YAKE; the UDPipe paper (Straka and Strakova, 2017) for the parser.
- **`CITATION.cff`** at the repo root with BibTeX for citing vaani itself in academic work. Planned to land before v0.1.
- **Worked examples** with deterministic fixed inputs and expected output values, so a researcher can verify their installation produces the same numbers.
- **Edge-case documentation** for known limitations (very short texts, non-English texts, code-heavy texts where lemmatization is ambiguous).
