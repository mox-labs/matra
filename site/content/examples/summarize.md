# Summarize a long document

The three sentences TextRank ranks highest in the opening of Darwin's chapter on the struggle for existence, 2,214 words.

## Input

<example-input name="summarize" />

## The call

<example-call name="summarize" />

## Output

<example-output name="summarize" />

Every sentence's score in document order, with the three the summary keeps marked:

<figure-textrank input="origin-struggle" />

## What to notice

- The summary carries the text's `provenance` with what made it: `method` is `textrank` and `parameters` is `{"n": 3}`. Each of its `sentences` has `index`, the sentence's place among the document's sentences counting from 0, its `span` in the input, its `text` and its `score`. The three come back in document order (indices 32, 33 and 58), not by score: the highest score, 0.0308, is the second. The figure numbers sentences from 1, so there they are 33, 34 and 59.
- All three calls hand matra the text rather than the file, so the provenance records no `path`: the Python method takes text, and the three must record the same input.
- The summary is extractive: the sentences are as Darwin wrote them, and nothing is rephrased.
- matra has a second summarizer, TF-IDF, and it is the default: `matra::summarize` in Rust, and the command line's `summary.method` as shipped. This example names TextRank in all three calls (`SummaryMethod::TextRank`, `method="textrank"`, `--method textrank`), so they agree whatever a configuration file says.
