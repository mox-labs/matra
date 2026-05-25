# Keyphrase extraction algorithms

🛠️ This page is a stub. Full content lands in a follow-up iteration.

vaani provides two keyphrase extraction algorithms: RAKE and YAKE. Both return a ranked list of phrases extracted from the document. Neither generates new text.

---

## The two algorithms

**RAKE** (Rapid Automatic Keyword Extraction) splits the text at stop words and sentence boundaries to produce candidate phrases. It then scores each word by its degree (how many other words it co-occurs with in candidate phrases) divided by its frequency. A phrase's score is the sum of its word scores. RAKE is fast, rule-based, and requires no model.

**YAKE** (Yet Another Keyword Extractor) scores candidate terms using five statistical features: position in the document (earlier terms score lower, indicating higher importance), frequency, co-occurrence with surrounding context words, sentence diversity (how many different sentences contain the term), and casing (terms that are capitalized in mid-sentence may be proper nouns or technical terms). YAKE scores are costs: lower scores indicate higher keyphrase quality. YAKE is more expensive to compute than RAKE but produces more context-sensitive rankings.

For the exact scoring formulas, see [reference/methodology.md](../reference/methodology.md#keyphrase-extraction-rake).

---

## When to use each

- **RAKE** is appropriate when speed matters and the text has clear keyword density patterns (technical documentation, news articles, structured reports).
- **YAKE** is appropriate when you need positionally-aware ranking or when the document structure makes early-sentence terms more significant (research abstracts, executive summaries).
- Both are capped at `MAX_TOKENS = 200_000` input tokens.

---

## What keyphrase extraction does not do

Both algorithms extract phrases from the source text. They do not:

- Understand the semantic meaning of phrases
- Distinguish between topic phrases and incidental phrases with similar surface statistics
- Normalize synonyms or related terms ("machine learning" and "ML" are separate candidates)
- Generate or paraphrase keyphrases

RAKE and YAKE return ranked keyphrases. Your application decides what a "keyphrase" means for its purpose.

---

## Planned for this page

A follow-up iteration will add:

- A worked example: the same paragraph processed by RAKE and YAKE, with annotations showing why the rankings differ
- The original RAKE paper citation (Rose et al., 2010) and YAKE paper citation (Campos et al., 2020)
- Guidance on the `max_keyphrases` parameter
