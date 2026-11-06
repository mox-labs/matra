//! Integration test through the public surface. Requires the UDPipe
//! English model, which `Engine::new` downloads into the resolved model
//! directory on first use (`MATRA_MODEL_DIR` names one).
//! Run with: cargo test --test integration -- --ignored

#[cfg(feature = "udpipe")]
mod with_model {
    use matra::domain::{Document, Format};
    use matra::{Engine, Input};

    fn engine() -> Engine {
        Engine::new().expect("the UDPipe model loads")
    }

    fn analyze(text: &str, format: Format) -> Document {
        engine().analyze(Input::text(text, format)).unwrap()
    }

    #[test]
    #[ignore] // requires the model
    fn full_pipeline_plain_text() {
        let analysis = analyze(
            "The cat sat on the mat. The dog chased the cat quickly.",
            Format::PlainText,
        );

        assert!(analysis.metrics.total_sentences >= 2);
        assert!(analysis.metrics.total_words > 0);
    }

    #[test]
    #[ignore]
    fn full_pipeline_markdown() {
        let md = "---\ntitle: Test\n---\n\n## Introduction\n\nFirst paragraph with several words in it.\n\n## Body\n\nSecond paragraph here.\n\n> A blockquote that should be skipped.";
        let analysis = analyze(md, Format::Markdown);

        assert_eq!(analysis.sections.len(), 2);
        assert_eq!(
            analysis.sections[0].heading.as_deref(),
            Some("Introduction")
        );

        // Blockquote paragraph exists but is not enriched
        let bq_count = analysis
            .paragraphs()
            .filter(|p| p.kind == matra::domain::ParagraphKind::Quote)
            .count();
        assert_eq!(bq_count, 1);
    }

    #[test]
    #[ignore]
    fn the_parse_carries_pos_and_deps() {
        let analysis = analyze("Engineers build systems.", Format::PlainText);
        let tokens = &analysis.sentences().next().expect("a sentence").tokens;

        let engineers = tokens.iter().find(|t| t.text == "Engineers").unwrap();
        assert_eq!(engineers.pos, "NOUN");
        assert_eq!(engineers.dep, "nsubj");

        let build = tokens.iter().find(|t| t.text == "build").unwrap();
        assert_eq!(build.pos, "VERB");
        assert_eq!(build.dep, "root");
    }

    #[test]
    #[ignore]
    fn passive_voice_detected() {
        let analysis = analyze(
            "The system was built by the team. The team shipped the product.",
            Format::PlainText,
        );

        assert!(analysis.sentences().any(|s| s.passive));
        assert!(analysis.sentences().any(|s| !s.passive));
    }

    #[test]
    #[ignore]
    fn compression_ratio_computed_for_long_paragraphs() {
        let long_para = "The quick brown fox jumps over the lazy dog. ".repeat(10);
        let analysis = analyze(&long_para, Format::PlainText);

        let has_ratio = analysis
            .paragraphs()
            .any(|p| p.metrics.compression_ratio.is_some());
        assert!(has_ratio, "long paragraph should have compression ratio");
    }

    /// Spans count UTF-8 bytes through the real parser too: after the
    /// first character outside ASCII, each sentence's span sliced from
    /// the source as bytes is still the sentence's text.
    #[test]
    #[ignore]
    fn non_ascii_sentence_spans_slice_as_bytes() {
        let text = "Café ☕ is open. Naïve plan.";
        let analysis = analyze(text, Format::PlainText);
        let sentences: Vec<_> = analysis.sentences().collect();
        assert_eq!(sentences.len(), 2);
        for sentence in sentences {
            let span = sentence.span;
            assert_eq!(&text[span.byte_start..span.byte_end], sentence.text);
        }
    }

    /// The outcome functions read the analysed document and need no
    /// engine: a summary and keyphrases of a real parse.
    #[test]
    #[ignore]
    fn outcomes_over_a_real_parse() {
        let analysis = analyze(
            "Rivers carry silt to the sea. The silt builds a delta. A delta needs silt.",
            Format::PlainText,
        );
        let summary = matra::summarize(&analysis, 2).unwrap();
        assert_eq!(summary.sentences.len(), 2);
        assert_eq!(summary.provenance.method.as_deref(), Some("tfidf"));
        let phrases = matra::keyphrases(&analysis, 5).unwrap();
        assert!(phrases.phrases.iter().any(|p| p.phrase.contains("silt")));
        assert_eq!(phrases.provenance.method.as_deref(), Some("rake"));
    }
}
