# Read a Markdown document's structure

A short Markdown document with two headings and a quote, read into sections and paragraphs.

## Input

<example-input name="markdown-structure" />

## The call

<example-call name="markdown-structure" />

## Output

<example-output name="markdown-structure" />

[Programming model](../explanation/programming-model.md#two-values-input-into-engine) steps this same document through each stage of the pipeline, to show what each stage adds.

## What to notice

- `sections` follow the headings: each has `heading`, the heading's text, `level`, the number of `#` marks, and `span`, where the heading lies in the file, `#` marks included.
- The quote is kept as a paragraph whose `kind` is `quote`, and it is never parsed: its `sentences` list is empty and all three of its measures are `null`. Its `span` covers the `>` markers its text leaves out.
- The Markdown reader is chosen by the `.md` extension in all three calls. Read as plain text, the same file is one section with no heading, and the heading lines and the quote are parsed as sentences, `#` marks and all.
