# How-to guides

Each guide does one task on one real text. Six show the input, the same call in Rust, Python and the command line, and what matra prints; the three calls print the same result, and CI runs all three and compares what they print with the page. The seventh has no command-line call, because the CLI has no command for it.

| Guide | Input | What it reads |
|---|---|---|
| [Parse a sentence into its dependency tree](parse-a-sentence.md) | Jane Austen, one sentence | every word's lemma, part of speech, head and relation |
| [Find negations and modals](negations-and-modals.md) | the Constitution, Article I (opening) | the negations and modal verbs of each sentence |
| [Compare readability across paragraphs](readability-by-paragraph.md) | The Federalist No. 10 | the three measures of every paragraph |
| [Summarize a long document](summarize.md) | Darwin, 2,214 words | the three sentences TextRank ranks highest |
| [Extract a document's keyphrases](keyphrases.md) | the same Darwin | the ten phrases RAKE ranks highest |
| [Read a Markdown document's structure](markdown-structure.md) | a short Markdown document | sections, headings, and the quote left unparsed |
| [Cluster sentences by meaning](../guides/semantic-clusters.md) | your own text | groups of sentences that say the same thing, Rust and Python only |

## Before you start

Install matra the way you mean to use it: the library, the CLI, or the Python package ([Installation](../tutorials/installation.md)). The Rust calls also need `serde_json` to print, which is the first line of each one: `cargo add matra serde_json`.

Each call reads its input from the directory it runs in, under the file name the page gives. Each page has the command that downloads it. The first call on a machine downloads the English model (about 16 MB), and every later call reads it from disk.

## What is not here

Analyzing a directory as a corpus has no guide here, because the CLI refuses a directory and these guides show all three calls. The [Rust](../guides/rust.md#analyze-a-directory) and [Python](../guides/python.md#analyze-a-directory) references cover it.
