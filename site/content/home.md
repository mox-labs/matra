# matra

matra parses English text into sections, paragraphs, sentences and words, and returns measures, summaries and keyphrases over them, from Rust, Python or the command line.

## Quick start

Install matra, save one sentence, and run matra on it.

<example-call name="quickstart" />

It prints JSON; here is the first word:

<example-output name="quickstart" />

Every word, drawn. Each arc runs from a word to the word that depends on it:

<figure-parse input="quickstart" />

## Where to go next

- **[Tutorial](tutorials/first-analysis.md):** run matra on two short paragraphs and find each part of what it returns. About ten minutes.
- **[How-to guides](examples/README.md):** one task each on real texts: summarize a document, extract keyphrases, find negations and modals, compare readability, cluster sentences by meaning.
- **[Explanation](explanation/concepts.md):** why the output has the shape it has, with figures you can step through.
- **[Reference](capabilities.md):** every type and field, the Rust, Python and command-line surfaces, the methods behind each measure, and every error.
