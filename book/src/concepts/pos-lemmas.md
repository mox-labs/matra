# Parts of speech and lemmas

🛠️ This page is a stub. Full content lands in a follow-up iteration.

Every token vaani produces carries two annotations that many metrics depend on: a Universal POS (UPOS) tag and a lemma. Understanding what these are and how vaani uses them clarifies what the metrics are actually measuring.

---

## Universal POS tags

vaani uses the UPOS tag set from the Universal Dependencies project. UPOS assigns every token to one of 17 categories:

| Tag | Category | Examples |
|---|---|---|
| `NOUN` | Noun | committee, proposal, document |
| `VERB` | Verb | approved, submitted, measures |
| `ADJ` | Adjective | structural, passive, internal |
| `ADV` | Adverb | quickly, however, not |
| `PRON` | Pronoun | it, they, this |
| `DET` | Determiner | the, a, this |
| `ADP` | Adposition (preposition/postposition) | in, by, without |
| `AUX` | Auxiliary | was, is, should |
| `CCONJ` | Coordinating conjunction | and, or, but |
| `SCONJ` | Subordinating conjunction | that, because, if |
| `PROPN` | Proper noun | vaani, UDPipe, English |
| `NUM` | Numeral | three, 11.2, first |
| `PUNCT` | Punctuation | . , ( ) |
| `SYM` | Symbol | $, %, @ |
| `PART` | Particle | not, 's |
| `INTJ` | Interjection | oh, yes |
| `X` | Other | foreign words, typos |

**How vaani uses UPOS:** Lexical density counts tokens with tags `NOUN`, `VERB`, `ADJ`, `ADV` as content-bearing. Nominalization ratio looks specifically at `NOUN` tokens. See [reference/methodology.md](../reference/methodology.md) for formulas.

---

## Lemmas

A lemma is the base or dictionary form of a word. UDPipe's lemmatizer maps each surface token to its lemma:

- "approved" → "approve"
- "committees" → "committee"
- "structural" → "structural" (adjectives often do not change)
- "was" → "be"

**How vaani uses lemmas:** Vocabulary TTR is computed over lemmas, not surface forms. "run," "runs," and "running" count as one lemma type. This reduces sensitivity to morphological variation; a text that uses many inflected forms of the same root is not penalized for lexical poverty.

Keyphrases (RAKE, YAKE) and summarization (TF-IDF, TextRank) use surface forms by default, with lemma normalization as an option.

---

## Limitations

UDPipe's POS tagger and lemmatizer are statistical models. They make errors, particularly on:

- Rare or domain-specific vocabulary
- Ambiguous tokens (e.g., "light" as noun vs verb vs adjective)
- Very short sentences (insufficient context for disambiguation)

POS tagging errors propagate to any metric derived from POS tags. The passive ratio, nominalization ratio, and lexical density are all affected by tagger accuracy. vaani does not expose a tagger confidence score; errors are silent.

---

## Planned for this page

A follow-up iteration will add:

- Worked examples showing the UPOS assignments for the canonical committee sentence
- How to inspect POS tags directly via the `Token.upos` field
- The interaction between UPOS tags and the `dep` (dependency relation) field
