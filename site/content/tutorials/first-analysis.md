# Your first analysis

You will run matra on two short paragraphs of meeting minutes, then find four things in what it returns: the document's structure, the grammar it read off each sentence, the measures of each paragraph, and the measures of the whole document. You need matra installed; [Installation](installation.md) covers every route.

## Save the text

<example-input name="first-analysis" />

## Run matra

Pick the language you will use. Each tab installs matra and runs the same analysis.

<example-call name="first-analysis" />

## Read the output

Every tab prints the same value. The words of each sentence are shown here as a count; the next section finds them.

<example-output name="first-analysis" />

## Find the structure

The output is a tree. The document holds `sections`; this one has a single section, because the text has no headings. The section holds `paragraphs`, two of them, one for each block of text separated by a blank line. Each paragraph holds its `sentences`, and each sentence its `tokens`.

The tokens are shown above only as a count. Each one carries its `text`, its `lemma`, its part of speech (`pos`), the `id` of the word it depends on (`head`) and the relation between them (`dep`). The [quick start](../home.md#quick-start) shows one sentence's tokens in full.

## Find the grammar

matra reads a few facts off each sentence's parse and stores them on the sentence.

1. In the first sentence, `negations` holds one entry. Its `cue_lemma` is `not`, and its `head_id` is 5: `not` negates the fifth word, `approve`.
2. In the second sentence, `modals` holds `may`, over word 9, `follow`. `reportings` holds `say`: the chair (`subject_lemma`) said something, and what was said is the clause headed by word 9.
3. The third sentence is passive: the amendments were submitted. matra does not mark a passive sentence with a field; it counts passive sentences for the document, below.

## Find the paragraph measures

Each paragraph carries three measures. `readability_grade` is a grade level, and `lexical_density` is the share of content words. `compression_ratio` is `null` for both paragraphs: matra computes it only for a paragraph of more than 50 words, and these are shorter. A `null` means the measure declined to run, never that it came out as zero.

## Find the document measures

Three measures describe the whole document. `passive_ratio` is 0.25: one sentence of the four is passive. `vocabulary_ttr` is the share of distinct lemmas among all of them. `nominalization_ratio` is the share of words that are nouns ending in a suffix such as `-tion` or `-ment`. It is 0 here: the one candidate, `amendments`, ends in an `s`, and the test reads the word as written ([Methodology](../reference/methodology.md#nominalization-ratio) has the rule).

## Where to go next

- [How-to guides](../examples/README.md): the same calls on longer, real texts, one task each.
- [Concepts](../explanation/concepts.md): why the output is a tree, and what each part of it records.
- [Domain model](../reference/domain-types.md): every field named above, with its type.
