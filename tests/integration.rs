//! Integration test. Requires UDPipe English model at /tmp/matra-models/.
//! Run with: cargo test --test integration -- --ignored

#[cfg(feature = "udpipe")]
mod with_model {
    use matra::nlp::NlpProvider;
    use matra::nlp::udpipe::Udpipe;

    fn model() -> Udpipe {
        Udpipe::from_path("/tmp/matra-models/english-ewt-ud-2.5-191206.udpipe")
            .expect("UDPipe model not found. Download to /tmp/matra-models/")
    }

    #[test]
    #[ignore] // requires model file
    fn full_pipeline_plain_text() {
        let nlp = model();
        let analysis = matra::analyze(
            "The cat sat on the mat. The dog chased the cat quickly.",
            &nlp,
        )
        .unwrap();

        assert!(analysis.total_sentences() >= 2);
        assert!(analysis.total_words() > 0);
    }

    #[test]
    #[ignore]
    fn full_pipeline_markdown() {
        let nlp = model();
        let md = "---\ntitle: Test\n---\n\n## Introduction\n\nFirst paragraph with several words in it.\n\n## Body\n\nSecond paragraph here.\n\n> A blockquote that should be skipped.";
        let analysis = matra::analyze_markdown(md, &nlp).unwrap();

        assert_eq!(analysis.sections.len(), 2);
        assert_eq!(
            analysis.sections[0].heading.as_deref(),
            Some("Introduction")
        );

        // Blockquote paragraph exists but is not enriched
        let bq_count = analysis.paragraphs().filter(|p| p.in_blockquote).count();
        assert_eq!(bq_count, 1);
    }

    #[test]
    #[ignore]
    fn nlp_parse_produces_pos_and_deps() {
        let nlp = model();
        let sentences = nlp.parse("Engineers build systems.").unwrap();

        assert!(!sentences.is_empty());
        let tokens = &sentences[0].tokens;

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
        let nlp = model();
        let analysis = matra::analyze(
            "The system was built by the team. The team shipped the product.",
            &nlp,
        )
        .unwrap();

        assert!(analysis.sentences().any(|s| s.is_passive()));
        assert!(analysis.sentences().any(|s| !s.is_passive()));
    }

    #[test]
    #[ignore]
    fn compression_ratio_computed_for_long_paragraphs() {
        let nlp = model();
        let long_para = "The quick brown fox jumps over the lazy dog. ".repeat(10);
        let analysis = matra::analyze(&long_para, &nlp).unwrap();

        let has_ratio = analysis.paragraphs().any(|p| p.compression_ratio.is_some());
        assert!(has_ratio, "long paragraph should have compression ratio");
    }

    #[test]
    #[ignore]
    fn error_on_missing_model() {
        let result = Udpipe::from_path("/nonexistent/model.udpipe");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, matra::domain::Error::ModelNotFound(_)));
    }
}
