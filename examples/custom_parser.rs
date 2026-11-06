//! A parser of your own, running through the engine.
//!
//! `Whitespace` is a toy: it splits a paragraph into sentences at `.`,
//! `!` and `?`, splits each sentence into words at whitespace, and
//! attaches every word to the sentence's first. It knows no grammar, so
//! the dependency arcs are flat and the tags are `X`, but it meets the
//! parser contract (`matra::nlp::NlpProvider`): ids `1..=n`, one root,
//! no cycle, and every span where its word lies in the paragraph. That
//! is all the engine needs to build sections, sentences, spans and the
//! measures over them, and every outcome function then runs on the
//! result.
//!
//! A real parser (a transformer, a WebAssembly build of another toolkit)
//! has the same shape: implement `parse` and `identity`, and hand it to
//! `Engine::with_parser`. No `udpipe` feature is needed:
//!
//! ```bash
//! cargo run --example custom_parser --no-default-features
//! ```

use matra::domain::{Format, Identity, Result, Span, Token};
use matra::nlp::NlpProvider;
use matra::{Engine, Input, summarize};

struct Whitespace;

impl NlpProvider for Whitespace {
    fn parse(&self, text: &str) -> Result<Vec<Vec<Token>>> {
        let mut sentences = Vec::new();
        let mut tokens: Vec<Token> = Vec::new();
        let mut cursor = 0;
        for word in text.split_whitespace() {
            // Spans are UTF-8 byte offsets into `text`, the paragraph the
            // engine handed in; the engine maps them into the document.
            let start = cursor + text[cursor..].find(word).unwrap_or(0);
            cursor = start + word.len();
            let id = tokens.len() + 1;
            let head = if id == 1 { 0 } else { 1 };
            let token = Token::builder(
                id,
                word.to_string(),
                word.to_lowercase(),
                "X".to_string(),
                head,
                if head == 0 { "root" } else { "dep" }.to_string(),
            )
            .span(Span::new(start, cursor))
            .build();
            tokens.push(token);
            if word.ends_with(['.', '!', '?']) {
                sentences.push(std::mem::take(&mut tokens));
            }
        }
        if !tokens.is_empty() {
            sentences.push(tokens);
        }
        Ok(sentences)
    }

    fn identity(&self) -> Identity {
        Identity::new("whitespace", Some("toy".to_string()), None, None)
    }
}

fn main() -> Result<()> {
    let text = "# Field notes\n\n\
                The river rose overnight. By morning the low road was under water.\n\n\
                The bridge held. Nobody crossed it until noon.\n";
    let engine = Engine::with_parser(Whitespace);
    let doc = engine.analyze(Input::text(text, Format::Markdown))?;

    println!(
        "parser: {} ({})",
        doc.provenance.parser.name,
        doc.provenance.parser.model.as_deref().unwrap_or("-")
    );
    println!(
        "{} sentences, {} words",
        doc.metrics.total_sentences, doc.metrics.total_words
    );
    for sentence in doc.sentences() {
        let source = &text[sentence.span.byte_start..sentence.span.byte_end];
        println!(
            "  {:>3}..{:<3} depth {:?}  {source}",
            sentence.span.byte_start, sentence.span.byte_end, sentence.tree_depth
        );
    }

    let summary = summarize(&doc, 1)?;
    for picked in &summary.sentences {
        println!("summary: {}", picked.text);
    }
    Ok(())
}
