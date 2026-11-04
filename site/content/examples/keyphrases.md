# Extract a document's keyphrases

The ten phrases RAKE ranks highest in the same Darwin text as the [summary example](summarize.md).

## Input

<example-input name="keyphrases" />

## The call

<example-call name="keyphrases" />

## Output

<example-output name="keyphrases" />

RAKE against YAKE on the same text, as ranks:

<figure-keyphrases input="origin-struggle" />

## What to notice

- The result carries the text's `provenance` with `method` `rake` and `parameters` `{"n": 10}`. Each of its `phrases` has `phrase`, `score` and `occurrences`, the span of every place in the input it was found, highest score first, and a higher score means more relevant. RAKE's score sums each word's co-occurrence degree over its frequency.
- Phrases are built from lemmas, so the parser's lemma errors show through. `great destruction fal` is "great destruction falling", with `falling` read as `fal`, and `slow -breeding man` is "slow-breeding man" split at the hyphen. RAKE takes the words between stopwords and punctuation, which is why `larger domestic animal tend` runs "animals tends" together.
- Two phrases tie at 9.0. Equal scores come back in the order the phrases first occur, the same on every run, so a cut that falls inside a tie is decided by position in the text, not by merit. This example asks for ten, where the tenth scores 5.972 and no phrase outside the list ties it, so no tie decides the list.
- The figure shows how little RAKE and YAKE agree on a text like this one. Neither method is right; they rank by different evidence ([Methodology](../reference/methodology.md)).
