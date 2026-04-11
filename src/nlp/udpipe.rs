//! UDPipe adapter. Implements NlpProvider using the udpipe-rs crate.
//! This file is the ONLY place that imports udpipe_rs.

use std::collections::HashMap;
use std::path::Path;

use udpipe_rs::Model;

use crate::domain::Error;
use crate::domain::{Sentence, Token};

use super::NlpProvider;

/// UDPipe adapter. Validated at construction: if the model is invalid,
/// construction fails. After construction, parse calls are trusted.
pub struct Udpipe {
    model: Model,
}

impl std::fmt::Debug for Udpipe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Udpipe").finish_non_exhaustive()
    }
}

impl Udpipe {
    /// Load from a file path. Fails fast if the model is invalid.
    pub fn from_path(path: impl AsRef<Path>) -> crate::domain::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(Error::ModelNotFound(path.to_path_buf()));
        }
        let model = Model::load(path)
            .map_err(|e| Error::ModelInvalid(e.to_string()))?;
        Ok(Self { model })
    }

    /// Load from bytes (e.g. embedded via include_bytes!).
    pub fn from_bytes(data: &[u8]) -> crate::domain::Result<Self> {
        let model = Model::load_from_memory(data)
            .map_err(|e| Error::ModelInvalid(e.to_string()))?;
        Ok(Self { model })
    }

    /// Download and load the English model.
    pub fn english(model_dir: impl AsRef<Path>) -> crate::domain::Result<Self> {
        let dir = model_dir.as_ref();
        std::fs::create_dir_all(dir)?;
        let path = dir.join("english-ewt-ud-2.5-191206.udpipe");
        if !path.exists() {
            let dir_str = dir.to_str()
                .ok_or_else(|| Error::ModelInvalid("model directory path is not valid UTF-8".into()))?;
            udpipe_rs::download_model("english-ewt", dir_str)
                .map_err(|e| Error::ModelInvalid(e.to_string()))?;
        }
        Self::from_path(&path)
    }
}

impl NlpProvider for Udpipe {
    fn parse(&self, text: &str) -> crate::domain::Result<Vec<Sentence>> {
        let words = self.model.parse(text)
            .map_err(|e| Error::ParseFailed(e.to_string()))?;

        let mut by_sentence: HashMap<i32, Vec<&udpipe_rs::Word>> = HashMap::new();
        for word in &words {
            by_sentence.entry(word.sentence_id).or_default().push(word);
        }

        let mut ids: Vec<i32> = by_sentence.keys().copied().collect();
        ids.sort();

        let sentences = ids
            .into_iter()
            .map(|id| {
                let sent_words = &by_sentence[&id];
                let tokens: Vec<Token> = sent_words
                    .iter()
                    .map(|w| Token {
                        id: usize::try_from(w.id).unwrap_or(0),
                        text: w.form.clone(),
                        lemma: w.lemma.clone(),
                        pos: w.upostag.clone(),
                        xpos: w.xpostag.clone(),
                        feats: w.feats.clone(),
                        dep: w.deprel.clone(),
                        head: usize::try_from(w.head).unwrap_or(0),
                        deps: String::from("_"),
                        misc: w.misc.clone(),
                        is_punct: w.is_punct(),
                    })
                    .collect();

                // Reconstruct original text using SpaceAfter=No from misc field.
                let text = {
                    let mut buf = String::new();
                    for (i, tok) in tokens.iter().enumerate() {
                        buf.push_str(&tok.text);
                        if i + 1 < tokens.len()
                            && !tok.misc.contains("SpaceAfter=No")
                        {
                            buf.push(' ');
                        }
                    }
                    buf
                };

                Sentence { text, tokens }
            })
            .collect();

        Ok(sentences)
    }
}
