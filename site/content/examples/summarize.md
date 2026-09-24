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

- Each item has `text`, `score` and `position`, the sentence's place in the document counting from 0. The three come back in document order (positions 32, 33 and 58), not by score: the highest score, 0.0308, is the second. The figure numbers sentences from 1, so there they are 33, 34 and 59.
- The summary is extractive: the sentences are as Darwin wrote them, and nothing is rephrased.
- matra has a second summarizer, `tfidf_summarize`, and it is the CLI's default (`summarize.algorithm` in the configuration). This example names TextRank in all three calls, so they agree whatever a configuration file says.
