//! Fixture for .semgrep/rule4-single-importer.yml. `semgrep --test` ignores
//! `paths`, so the outside-adapter rules and the exposed rules all run over
//! this file. A doc comment may say udpipe_rs::Model or model.safetensors.

// ok: boundary-rule4-udpipe-rs-outside-adapter, boundary-rule4-udpipe-rs-exposed
use crate::nlp::udpipe::Udpipe;
// ok: boundary-rule4-model2vec-crates-outside-adapter
let tokenizer_count = 3;

// ruleid: boundary-rule4-udpipe-rs-outside-adapter
use udpipe_rs::Model;
// ruleid: boundary-rule4-udpipe-rs-outside-adapter
use udpipe_rs::{Model, Sentence as UdSentence};
// ruleid: boundary-rule4-udpipe-rs-outside-adapter
use ::udpipe_rs::model::Model;
// ruleid: boundary-rule4-udpipe-rs-outside-adapter
extern crate udpipe_rs;
// ruleid: boundary-rule4-udpipe-rs-outside-adapter
use {std::fmt, udpipe_rs::Model};

fn inline() {
    // ruleid: boundary-rule4-udpipe-rs-outside-adapter
    let m = udpipe_rs::Model::load_from_memory(&bytes);
    // ruleid: boundary-rule4-udpipe-rs-outside-adapter
    let s: Vec<udpipe_rs::Sentence> = Vec::new();
}

// ruleid: boundary-rule4-udpipe-rs-outside-adapter, boundary-rule4-udpipe-rs-exposed
pub use udpipe_rs::Model;
// ruleid: boundary-rule4-udpipe-rs-outside-adapter, boundary-rule4-udpipe-rs-exposed
pub(crate) type RawModel = udpipe_rs::Model;
// ruleid: boundary-rule4-udpipe-rs-outside-adapter, boundary-rule4-udpipe-rs-exposed
pub fn raw(&self) -> &udpipe_rs::Model {
    &self.model
}
pub struct Holder {
    // ruleid: boundary-rule4-udpipe-rs-outside-adapter, boundary-rule4-udpipe-rs-exposed
    pub model: udpipe_rs::Model,
    // ruleid: boundary-rule4-udpipe-rs-outside-adapter
    model2: udpipe_rs::Model,
    // A comma inside the field's generics does not end the field.
    // ruleid: boundary-rule4-udpipe-rs-outside-adapter, boundary-rule4-udpipe-rs-exposed
    pub by_name: HashMap<String, udpipe_rs::Model>,
    // ruleid: boundary-rule4-udpipe-rs-outside-adapter, boundary-rule4-udpipe-rs-exposed
    pub nested: HashMap<Vec<u8>, Vec<(u8, udpipe_rs::Model)>>,
    // ruleid: boundary-rule4-udpipe-rs-outside-adapter, boundary-rule4-udpipe-rs-exposed
    pub(crate) third: Triple<A, Vec<B>, udpipe_rs::Sentence>,
    // ok: boundary-rule4-udpipe-rs-exposed
    pub names: HashMap<String, Vec<u8>>,
    // Tuple and fn-pointer types are walked through their parens.
    // ruleid: boundary-rule4-udpipe-rs-outside-adapter, boundary-rule4-udpipe-rs-exposed
    pub pair: (u8, udpipe_rs::Model),
    // ruleid: boundary-rule4-udpipe-rs-outside-adapter, boundary-rule4-udpipe-rs-exposed
    pub loader: fn(u8) -> udpipe_rs::Model,
    // ruleid: boundary-rule4-udpipe-rs-outside-adapter, boundary-rule4-udpipe-rs-exposed
    pub parse: fn(&str, udpipe_rs::Sentence) -> u8,
    // ruleid: boundary-rule4-udpipe-rs-outside-adapter, boundary-rule4-udpipe-rs-exposed
    pub boxed: Box<dyn Fn(u8, &str) -> udpipe_rs::Model>,
    // ruleid: boundary-rule4-udpipe-rs-outside-adapter, boundary-rule4-udpipe-rs-exposed
    pub nested_pair: Option<(String, Vec<udpipe_rs::Sentence>)>,
    // ok: boundary-rule4-udpipe-rs-exposed
    pub plain_pair: (u8, String),
    // ok: boundary-rule4-udpipe-rs-exposed
    pub plain_fn: fn(u8, u16) -> Vec<u8>,
    // ok: boundary-rule4-udpipe-rs-exposed
    pub plain_boxed: Box<dyn Fn(u8) -> String>,
    // ruleid: boundary-rule4-udpipe-rs-outside-adapter
    pub count: u8, private: udpipe_rs::Model,
}
// ruleid: boundary-rule4-udpipe-rs-outside-adapter, boundary-rule4-udpipe-rs-exposed
pub struct OneLine { pub model: HashMap<String, udpipe_rs::Model> }
pub struct Embeddings {
    // ruleid: boundary-rule4-model2vec-crates-outside-adapter, boundary-rule4-model2vec-crates-exposed
    pub by_name: HashMap<String, tokenizers::Tokenizer>,
    // ruleid: boundary-rule4-model2vec-crates-outside-adapter, boundary-rule4-model2vec-crates-exposed
    pub tensors: BTreeMap<u32, Vec<safetensors::tensor::TensorView<'static>>>,
    // ok: boundary-rule4-model2vec-crates-exposed
    pub dims: HashMap<String, usize>,
    // ruleid: boundary-rule4-model2vec-crates-outside-adapter, boundary-rule4-model2vec-crates-exposed
    pub pair: (u32, tokenizers::Encoding),
    // ruleid: boundary-rule4-model2vec-crates-outside-adapter, boundary-rule4-model2vec-crates-exposed
    pub load: fn(&[u8]) -> safetensors::SafeTensors<'static>,
}
// ruleid: boundary-rule4-udpipe-rs-exposed
pub(super) fn load(
    // ruleid: boundary-rule4-udpipe-rs-outside-adapter
    bytes: &[u8], model: udpipe_rs::Model,
) -> Result<()> {
    Ok(())
}
// ok: boundary-rule4-udpipe-rs-exposed
pub fn parse(&self, text: &str) -> Result<Vec<Sentence>> {
    // ruleid: boundary-rule4-udpipe-rs-outside-adapter
    let raw = udpipe_rs::parse(text);
}

// ruleid: boundary-rule4-model2vec-crates-outside-adapter
use safetensors::SafeTensors;
// ruleid: boundary-rule4-model2vec-crates-outside-adapter
use tokenizers::{Tokenizer, Encoding};
fn m2v() {
    // ruleid: boundary-rule4-model2vec-crates-outside-adapter
    let t = tokenizers::Tokenizer::from_bytes(&b);
}
// ruleid: boundary-rule4-model2vec-crates-outside-adapter, boundary-rule4-model2vec-crates-exposed
pub use tokenizers::Tokenizer;
// ruleid: boundary-rule4-model2vec-crates-outside-adapter, boundary-rule4-model2vec-crates-exposed
pub fn tensors(&self) -> &safetensors::SafeTensors<'_> {
    &self.t
}
// ok: boundary-rule4-model2vec-crates-exposed
pub fn embed(&self, text: &str) -> Result<Embedding> {
    // ruleid: boundary-rule4-model2vec-crates-outside-adapter
    let enc = tokenizers::encode(text);
}

#[cfg(test)]
mod tests {
    // Test modules are not exempt from rule 4.
    // ruleid: boundary-rule4-udpipe-rs-outside-adapter
    use udpipe_rs::Model;
}
