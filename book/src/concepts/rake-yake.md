# Keyphrase extraction: RAKE and YAKE

vaani provides two keyphrase extraction algorithms. Both return a ranked list of phrases with scores. They use different signals and produce different results on the same input.

---

## What keyphrase extraction does

Keyphrase extraction identifies noun phrases that represent the key topics of a text. The output is a list of short phrases (typically one to three words) ranked by relevance. Unlike summarization, which returns whole sentences, keyphrase extraction returns the terms themselves.

The phrase list is useful when you need: a topic index over a document set, a signal for routing or classification, or a quick scan of what a document is about without reading it.

---

## RAKE: rule-based, fast, deterministic

RAKE (Rapid Automatic Keyword Extraction, Rose et al., 2010) is a rule-based algorithm. It does not learn from data; it applies a fixed procedure.

The procedure:

1. Split the text at stop words and punctuation. What remains between splits are candidate phrases.
2. Filter candidates to runs of `NOUN`, `ADJ`, and `PROPN` tokens (vaani uses POS tags from the dependency parse to filter here; plain RAKE uses only stop-word splitting).
3. For each word in the candidate set, count how often it appears in candidates (frequency) and how many total word co-occurrences it participates in across all candidates (degree).
4. Score each word: degree / frequency. A word that appears in long phrases scores high relative to a word that appears only in isolation.
5. Score each phrase: the sum of its words' scores.

The phrase "machine learning algorithms" scores higher than "machine" alone because each word in the multi-word phrase accumulates degree from the others.

RAKE in `rake-nltk` (Python) and similar packages applies the same algorithm without POS filtering. vaani's POS filter reduces noise from verb phrases and adverbs that appear between stop words.

**When to use RAKE:** when you need fast, reproducible keyphrase extraction and the text is reasonably well-formed English. RAKE is deterministic: the same input always produces the same output with the same scores.

**When RAKE underperforms:** short texts (few candidates to build co-occurrence statistics from) and texts where the important terms are not noun-phrase-shaped.

---

## YAKE: statistical, position-aware

YAKE (Yet Another Keyword Extractor, Campos et al., 2018) is a statistical algorithm. It does not use rules or POS tags for scoring; it uses the statistical properties of each word across the text.

The per-term score combines three signals:

1. **Positional.** Terms appearing earlier in the document score as more important. This reflects the editorial convention that important terms are introduced early.
2. **Frequency.** Terms that appear more often score as more important.
3. **Context diversity.** A term surrounded by many different neighboring words has a high context diversity score. A term that always appears next to the same word (low diversity) scores lower.

The raw term score is `(positional_score + context_diversity) / (frequency_score + 1)`. Lower raw score means more relevant. vaani inverts this before returning results so that higher score always means more relevant in the output.

Candidate phrases are 1-gram, 2-gram, and 3-gram windows over content tokens. The phrase score is the product of its component term scores (in the raw, lower-is-better scale). The best (lowest) score across all occurrences of a phrase is kept.

The original YAKE paper describes the Python package `yake`. vaani's implementation follows the same algorithmic approach.

**When to use YAKE:** when positional and statistical signals matter more than syntactic structure. Technical documents, academic abstracts, and content where important terms recur with contextual variety. YAKE can surface multi-word phrases that RAKE would miss because they include non-noun parts.

**When YAKE underperforms:** very short texts where statistical signals are unreliable, and texts with unusual term repetition patterns that confuse the positional weighting.

---

## Choosing between them

| | RAKE | YAKE |
|---|---|---|
| Basis | Structural (POS, co-occurrence) | Statistical (position, frequency, context) |
| Speed | Faster | Slower |
| Deterministic | Yes | Yes |
| Phrase length | 1+ words (NOUN/ADJ/PROPN runs) | 1-3 words |
| Handles non-noun phrases | No | Yes |
| Works on short text | Poorly | Poorly |

Both return the same output shape: a `Vec<Keyphrase>` with `phrase` and `score`. Both cap at 200,000 input tokens.

If you are unsure which to use, run both and compare the top-10 results. Phrases that appear in both ranked lists have two independent signals behind them. Phrases that appear in only one list tell you something about what that algorithm values that the other does not.

---

## The 200,000-token cap

Both algorithms are capped at 200,000 input tokens. Above this limit, they return an error (`Error::InputTooLarge`).

The cap is on tokens, not sentences. A document with 50,000 single-token sentences fits within the cap just as comfortably as a document with 200 sentences of 200 tokens each. The token-based cap reflects the actual memory cost model: the dominant data structure in RAKE is a per-word co-occurrence map bounded by total tokens, and in YAKE it is a per-term context vector with the same bound.

[Methodology](../reference/methodology.md) documents the scoring formulas in full. [TF-IDF and TextRank](./tfidf-textrank.md) covers the summarization algorithms. [Affordances](./affordances.md) covers the full capability list.
