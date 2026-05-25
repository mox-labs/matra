# Part-of-speech tags and lemmatization

Every token in vaani's output has two categorical annotations: a part-of-speech (POS) tag and a lemma. This page explains what both mean and where they come from.

---

## Part-of-speech tags

vaani uses the Universal Dependencies (UD) UPOS tag set. There are 17 tags, and they cover every word in every language that UD annotates:

| Tag | Category | Examples |
|---|---|---|
| `NOUN` | Common noun | proposal, committee, rate |
| `VERB` | Verb | approved, submitted, analyze |
| `ADJ` | Adjective | quick, structural, main |
| `ADV` | Adverb | quickly, however, also |
| `PRON` | Pronoun | it, they, which |
| `DET` | Determiner | the, a, this |
| `ADP` | Adposition (preposition / postposition) | of, by, without |
| `AUX` | Auxiliary verb | was, were, should, have |
| `CCONJ` | Coordinating conjunction | and, but, or |
| `SCONJ` | Subordinating conjunction | because, if, that |
| `PROPN` | Proper noun | London, vaani, Python |
| `NUM` | Number | three, 42, first |
| `PART` | Particle | not, 's, to (infinitival) |
| `INTJ` | Interjection | oh, yes, hello |
| `PUNCT` | Punctuation | . , : — |
| `SYM` | Symbol | $, %, © |
| `X` | Other (foreign words, typos, unknown) | |

The `pos` field on a `Token` contains one of these 17 strings exactly as UDPipe assigns it. vaani does not alter or filter them.

The tag matters for downstream reasoning. RAKE's candidate extraction uses `pos` directly: a candidate phrase is a run of `NOUN`, `ADJ`, or `PROPN` tokens. The nominalization heuristic checks `pos == "NOUN"` before testing the suffix. Passive detection checks the `dep` label (not `pos`), but the POS of the passive subject is typically `NOUN` or `PROPN`. If your application reads vaani's token output to build its own rules or filters, `pos` is the field those rules run against.

---

## Lemmatization

The lemma is the dictionary form of a word. It strips inflection: verb conjugations, noun plurals, adjective comparatives, and so on.

Some examples:

| Surface form | Lemma |
|---|---|
| approved | approve |
| amendments | amendment |
| submitted | submit |
| running | run |
| better | good |
| was | be |

The `lemma` field on each `Token` contains this form as UDPipe computes it. vaani passes it through unchanged.

Lemmatization matters for the metrics and extraction algorithms. TF-IDF and YAKE score on lemmas, not surface forms, so "submitted", "submitting", and "submit" all count as the same term. The vocabulary TTR is computed over lemmas as well: a text that uses "run", "runs", "ran", and "running" has one unique lemma, not four, which gives a more accurate picture of lexical variety than counting surface forms.

---

## What lemmatization does not do

Lemmatization is not stemming. Stemming strips characters from word endings to produce a stem that may not be a real word ("running" becomes "run", "better" becomes "bet" in some stemmers). Lemmatization produces the actual dictionary form by looking up the word's morphological features.

The UDPipe lemmatizer is statistical: it learns lemmas from annotated treebanks. It is accurate on standard written English but will produce incorrect lemmas on highly domain-specific vocabulary, proper nouns that look like common words, or misspelled text.

Lemmatization also does not normalize word sense. "Bank" (financial institution) and "bank" (river bank) have the same lemma. The lemma does not tell you which sense is in play; that requires semantic analysis beyond what UDPipe provides.

---

## The `xpos` field

Each token also has an `xpos` field: a language-specific POS tag (Penn Treebank tags for English, such as `NN`, `VBD`, `DT`). These are more granular than UPOS tags and carry language-specific distinctions (singular vs plural noun, past vs present tense verb).

vaani preserves `xpos` in the token output, but the metrics and extraction algorithms use `pos` (the UPOS tag). `xpos` is available if your application needs finer-grained distinctions.

[Dependency parsing](./dependency-parsing.md) explains how `pos` and `dep` work together to represent sentence structure. [Domain types reference](../reference/domain-types.md) documents all token fields.
