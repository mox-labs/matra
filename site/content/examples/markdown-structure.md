# Read a Markdown document's structure

A short Markdown document with two headings and a quote, read into sections and paragraphs.

## Input

<example-input name="markdown-structure" />

## The call

<example-call name="markdown-structure" />

## Output

<example-output name="markdown-structure" />

The same document at each stage of the pipeline, step by step:

<figure-pipeline input="field-notes" />

## What to notice

- `sections` follow the headings: each has `heading`, the heading's text, and `level`, the number of `#` marks.
- The quote is kept as a paragraph with `in_blockquote` set to `true`, and it is never parsed: its `sentences` list is empty and all three of its measures are `null`.
- The Markdown reader is chosen by the `.md` extension in the Rust call and the CLI, and by name in Python (`analyze_markdown`). Read as plain text, the same file is one section with no heading, and the heading lines and the quote are parsed as sentences, `#` marks and all.
