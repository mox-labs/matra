# Compare readability across paragraphs

The three paragraph measures of The Federalist No. 10, to see where the essay is harder going and where it eases.

## Input

<example-input name="readability-by-paragraph" />

## The call

<example-call name="readability-by-paragraph" />

## Output

<example-output name="readability-by-paragraph" />

Each measure over the essay's 23 paragraphs, one panel each:

<figure-metrics input="federalist-10" />

## What to notice

- Each paragraph carries `readability_grade`, `lexical_density` and `compression_ratio`. The document carries `vocabulary_ttr`, `nominalization_ratio` and `passive_ratio` beside its `sections`.
- The grade runs from 10.69 (paragraph 3) to 32.06 (paragraph 18). It is the Flesch-Kincaid formula over sentence length and syllables per word ([Methodology](../reference/methodology.md#flesch-kincaid-grade-level)), so an eighteenth-century paragraph of a few long sentences scores far above any school grade. Compare paragraphs with each other, not with a grade band.
- `compression_ratio` is `null` for four paragraphs, the ones of 50 words or fewer. `null` means the measure declined to run, not that it measured zero; which measure needs what is in the [Rust guide](../guides/rust.md#why-a-metric-slot-is-none).
