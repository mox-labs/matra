# Readability: Flesch-Kincaid

vaani computes Flesch-Kincaid grade level for each paragraph. This page explains what the formula measures, how vaani implements it, and where it stops.

---

## The formula

Flesch-Kincaid grade level (Kincaid et al., 1975):

```
grade = 0.39 * (words / sentences) + 11.8 * (syllables / words) - 15.59
```

Two inputs: average words per sentence and average syllables per word. The coefficients were fitted to Navy technical manuals to predict the U.S. school grade level at which a reader could be expected to comprehend the text.

A grade of 8 maps to eighth-grade reading level. A grade of 12 maps to high school. A grade of 16 maps to college senior. The scale is not strictly bounded: very short, very long, or very dense text can produce values below 0 or above 20.

---

## What vaani measures

vaani counts words by whitespace splitting, not by NLP tokenization. This matches the formula's original specification: the metric was designed before statistical NLP tokenizers existed, and uses whitespace as the word boundary. The NLP token count (which excludes punctuation and handles contractions) would give a different number.

Sentence boundaries are counted by punctuation characters (`.`, `!`, `?`). If the text has no sentence-ending punctuation, vaani treats it as one sentence. This is a safe fallback but will produce misleading grades for headings, bullet points, or other non-sentential text.

Syllables are counted by a heuristic: vowel groups are counted, silent trailing `e` is subtracted when there is more than one vowel group, and every word is guaranteed at least one syllable.

The metric is applied to paragraphs with more than 10 words that are not in a blockquote. Short paragraphs and blockquotes are excluded; their `readability_grade` field is `None`.

---

## What the grade means for your application

The grade is a single number computed from two surface signals. It correlates with text complexity in the original domain (Navy manuals), and it is widely used as a proxy for general readability.

A document analysis system targeting a general consumer audience might flag paragraphs above grade 12 for review. A system for academic publishing might expect grades of 16 or higher. The grade itself is neutral. vaani measures; your application decides.

What it captures: the relationship between sentence length, word length, and reading difficulty. Long sentences with polysyllabic words score high. Short sentences with monosyllabic words score low.

What it does not capture:

- **Coherence.** A paragraph of unrelated simple sentences scores the same as an equivalently simple paragraph with a clear argument.
- **Vocabulary difficulty.** "The cat sat on the mat" and "The pes sat on the mat" (pes = Latin for foot) have identical FK scores. The formula cannot detect unknown vocabulary.
- **Writing quality.** A grade of 14 is not better or worse than a grade of 9. The appropriate grade depends entirely on the intended audience.
- **Discourse structure.** Paragraph transitions, topic shifts, and argument construction are invisible to the formula.
- **Non-prose text.** Code blocks, tables, and bulleted lists are not prose. FK scores on these will be numerically computed but linguistically meaningless.

---

## External reference

The original paper: Kincaid, J.P., Fishburne, R.P., Rogers, R.L., and Chissom, B.S. (1975). *Derivation of new readability formulas for Navy enlisted personnel*. Research Branch Report 8-75, Chief of Naval Technical Training.

The formula is also documented in [Methodology](../reference/methodology.md), which is the canonical formula reference for all vaani metrics.

[Passive voice and nominalization](./passive-nominalization.md) covers the other structural metrics. [Affordances](./affordances.md) lists all metrics vaani computes.
