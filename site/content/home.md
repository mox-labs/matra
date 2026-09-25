# matra

matra reads text into its structure and measures it, from Rust, Python or the command line; what the structure means is yours to decide.

## One passage, and what matra returns

Ten short sentences, some saying the same thing in different words. matra turns each into a vector and groups the sentences whose similarity clears a threshold. The table lists the groups at every threshold; the figure draws one threshold at a time, and you can step it.

<figure-clusters input="paraphrases" />

This is one of the things matra returns, and it comes with the Python package and with the Rust library's `model2vec` feature; the command line has no clusters command. The [Examples](examples/README.md) show the parse, the measures, summaries and keyphrases the same way, on real texts.

## Where to go next

- **[Install](tutorials/installation.md):** the Rust library, the command line, or the Python package, and one call that proves the install works.
- **[Examples](examples/README.md):** six tasks on real texts, each with the call in Rust, Python and the command line, and the output it prints.
- **[Concepts](explanation/concepts.md):** the shape of what matra returns, from the document down to each word, and the grammatical facts matra reads off each sentence.
