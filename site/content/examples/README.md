# Examples

Six worked examples, each one task on one real text. Every example shows the input, the same call in Rust, Python and the command line, and what matra prints. The three calls print the same result. CI runs all three for every example and compares what they print with the output on the page, so an example cannot drift from what matra does.

| Example | Input | What it reads |
|---|---|---|
| [Parse one sentence](parse-a-sentence.md) | Jane Austen, one sentence | every word's lemma, part of speech, head and relation |
| [Find negations and modals](negations-and-modals.md) | the Constitution, Article I (opening) | the negations and modal verbs of each sentence |
| [Compare readability across paragraphs](readability-by-paragraph.md) | The Federalist No. 10 | the three measures of every paragraph |
| [Summarize a long document](summarize.md) | Darwin, 2,214 words | the three sentences TextRank ranks highest |
| [Extract keyphrases](keyphrases.md) | the same Darwin | the ten phrases RAKE ranks highest |
| [Read a Markdown document's structure](markdown-structure.md) | a short Markdown document | sections, headings, and the quote left unparsed |

## Before you start

Install matra the way you mean to use it: the library, the CLI, or the Python package ([Installation](../tutorials/installation.md)). The Rust calls also need `serde_json` to print, which is the first line of each one: `cargo add matra serde_json`.

Each call reads its input from the directory it runs in, under the file name the page gives. Each page has the command that downloads it. The first call on a machine downloads the English model (about 16 MB), and every later call reads it from disk.

## What is not here

Two tasks are left out because the command line cannot do them, and an example here shows all three calls. The CLI refuses a directory, so analyzing a directory as a corpus is in the [Rust](../guides/rust.md#analyze-a-directory) and [Python](../guides/python.md#analyze-a-directory) guides. The CLI has no command for semantic clusters, which are in their [own guide](../guides/semantic-clusters.md).
