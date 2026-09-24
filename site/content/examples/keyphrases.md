# Extract keyphrases

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

- Each item has `phrase` and `score`, highest score first, and a higher score means more relevant. RAKE's score sums each word's co-occurrence degree over its frequency.
- Phrases are built from lemmas, so the parser's lemma errors show through. `great destruction fal` is "great destruction falling", with `falling` read as `fal`, and `slow -breeding man` is "slow-breeding man" split at the hyphen. RAKE takes the words between stopwords and punctuation, which is why `larger domestic animal tend` runs "animals tends" together.
- Two phrases tie at 9.0, and equal scores can come back in either order from one run to the next. A cut that falls inside a tie can also change which phrases make the list. This example asks for ten, where the tenth scores 5.972 and the eleventh 4.636, so the list itself does not change.
- The figure shows how little RAKE and YAKE agree on a text like this one. Neither method is right; they rank by different evidence ([Methodology](../reference/methodology.md)).
