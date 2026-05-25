# Readability measures

🛠️ This page is a stub. Full content lands in a follow-up iteration.

vaani computes two readability metrics: Flesch Reading Ease (FRE) and Flesch-Kincaid Grade Level (FKGL). Both derive from two surface statistics: average sentence length and average syllable count per word.

---

## What the metrics capture

Flesch-Kincaid metrics were designed in the 1940s-1970s to predict whether a text would be accessible to a given reading level. The intuition: shorter sentences and shorter words correlate with easier reading. The metrics operationalize that intuition as a formula applied to counts.

**Flesch Reading Ease** produces a 0-100 score. Higher = easier. A score near 60 is roughly "plain English." Below 30 is professional or academic.

**Flesch-Kincaid Grade Level** produces a U.S. school-grade approximation. A value of 8.0 suggests the text is accessible to a typical 8th-grade reader.

For the exact formulas, inputs, and non-claims, see [reference/methodology.md](../reference/methodology.md#readability-flesch-kincaid).

---

## What the metrics do not capture

Readability scores are surface statistics, not comprehension measurements. A document full of short, simple sentences can be confusing. A long sentence with a complex subordinate clause can be perfectly clear to a specialist reader.

vaani computes the score. Whether the score is appropriate for the document's audience and genre is a decision for the application consuming vaani's output.

Common traps:

- Optimizing text to hit a target readability score can degrade actual clarity.
- Readability scores were calibrated on general English prose; technical documentation, legal text, and code comments are outside the calibration domain.
- vaani's syllable counter uses a rule-based approximation; results differ slightly from human syllable counts.

---

## Planned for this page

A follow-up iteration will add:

- Worked example: the committee sentence annotated with word-by-word syllable counts and the resulting FRE/FKGL values
- Guidance on interpreting grade level for technical documentation vs narrative prose
- Links to the original Flesch (1948) and Kincaid et al. (1975) papers that introduced the formulas
