# Parse one sentence

The first sentence of Pride and Prejudice, parsed: every word with its lemma, its part of speech, the word it depends on, and the relation between them.

## Input

<example-input name="parse-a-sentence" />

## The call

<example-call name="parse-a-sentence" />

## Output

<example-output name="parse-a-sentence" />

The same 25 tokens as a table, then as arcs drawn from each word's head:

<figure-parse input="austen-first-sentence" />

## What to notice

- A document is sections of paragraphs of sentences, even when it is one sentence, so the tokens are at `sections[0].paragraphs[0].sentences[0].tokens`. In Rust, `doc.sentences()` walks every sentence in document order.
- Each token carries its CoNLL-U columns under matra's names: `id` (from 1 within the sentence), `text`, `lemma`, `pos` (the Universal Dependencies tag), `head` (the `id` of the word it depends on, 0 for the root) and `dep` (the relation). Here `truth` is the root, and `It` is its `nsubj`.
- `lemma` is the dictionary form: `is` becomes `be`, and `acknowledged` becomes `acknowledge`.
- The parse is the model's, reported as the model returned it. The model reads `that` (token 8) as a pronoun and the object of `want`, where a grammar would call it the conjunction that opens the clause. matra does not correct the parser; [Methodology](../reference/methodology.md#the-parse-layer) names the model and its limits.
- The whole output also carries the structural primitives read off this tree: a modal (`must`, governing `want`) and a reporting (`acknowledge`, whose clause is headed by `want`). The [next example](negations-and-modals.md) reads those across a document.
