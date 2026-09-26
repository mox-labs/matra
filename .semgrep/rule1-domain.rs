//! Fixture for .semgrep/rule1-domain.yml. `semgrep --test` ignores `paths`,
//! so every rule in the matching .yml runs over this whole file.
//! A comment naming crate::nlp::Udpipe or `use regex::Regex;` is not an import.

// ok: boundary-rule1-domain-imports
use std::path::{Path, PathBuf};
// ok: boundary-rule1-domain-imports
use serde::{Deserialize, Serialize};
// ok: boundary-rule1-domain-imports
use thiserror::Error;
// ok: boundary-rule1-domain-imports
use core::fmt;
// ok: boundary-rule1-domain-imports
pub use std::collections::HashMap;

// ruleid: boundary-rule1-domain-imports
use regex::Regex;
// ruleid: boundary-rule1-domain-imports
use chrono::{DateTime, Utc};
// ruleid: boundary-rule1-domain-imports
pub use serde_json::Value;
// ruleid: boundary-rule1-domain-imports
pub(crate) use smallvec::SmallVec;
// ruleid: boundary-rule1-domain-imports
use ::regex::bytes;
// ruleid: boundary-rule1-domain-imports
use crate::nlp::NlpProvider;
// ruleid: boundary-rule1-domain-imports
use crate::{config::Config, nlp};
// ruleid: boundary-rule1-domain-imports
use super::stopwords::is_stop_word;
// ruleid: boundary-rule1-domain-imports
use self::inner::Thing;
// ruleid: boundary-rule1-domain-imports
use {regex::Regex, std::fmt};
// ruleid: boundary-rule1-domain-imports
extern crate regex;
// ruleid: boundary-rule1-domain-imports
use serde_extra as serde2;
// ruleid: boundary-rule1-domain-imports
#[cfg(feature = "x")] use regex::Regex;

#[derive(Debug, Clone, Serialize, Deserialize)]
// ok: boundary-rule1-domain-inline-paths
#[derive(thiserror::Error)]
pub enum Error {
    // ok: boundary-rule1-domain-inline-paths
    Io(#[from] std::io::Error),
}

pub struct Token {
    // ok: boundary-rule1-domain-inline-paths
    pub map: std::collections::HashMap<String, String>,
    // ruleid: boundary-rule1-domain-inline-paths
    pub when: chrono::DateTime<chrono::Utc>,
    // ruleid: boundary-rule1-domain-inline-paths
    pub parser: crate::nlp::udpipe::Udpipe,
}

impl Token {
    pub fn depth(&self) -> usize {
        // ok: boundary-rule1-domain-inline-paths
        let limit = usize::MAX;
        // ok: boundary-rule1-domain-inline-paths
        let s = <Self as serde::Serialize>::serialize;
        // ok: boundary-rule1-domain-inline-paths
        let v: Vec<u8> = Vec::new();
        // A turbofish is not a path root.
        // ok: boundary-rule1-domain-inline-paths
        let mean = v.iter().map(|x| *x as f64).sum::<f64>() / helper::<f64>(v.len());
        // ruleid: boundary-rule1-domain-inline-paths
        let re = regex::Regex::new("x").ok();
        // ruleid: boundary-rule1-domain-inline-paths
        let words = crate::stopwords::is_stop_word("the");
        // ruleid: boundary-rule1-domain-inline-paths
        let j = serde_json::to_string(&self);
        // ruleid: boundary-rule1-domain-inline-paths
        let root = super::Engine::default();
        // ruleid: boundary-rule1-domain-inline-paths
        let f = fmt::Display::fmt;
        // ruleid: boundary-rule1-domain-inline-paths
        somecrate::some_macro!();
        limit
    }
}

// ruleid: boundary-rule1-domain-inline-paths
#[derive(Debug, fancy_derive::Builder)]
pub struct Built;

// A multi-line std group with a comment inside is not an inline path.
// ok: boundary-rule1-domain-imports, boundary-rule1-domain-inline-paths
use std::{
    // formatting, then io
    fmt,
    io::Write,
};

// ruleid: boundary-rule1-domain-imports
use chrono::{
    // a comment inside the group does not hide it
    DateTime,
};

// A string line that starts with "use" is not a use line: no finding below.
pub const HINT: &str = "use the smaller model. Or:
use the other one";
// ruleid: boundary-rule1-domain-inline-paths
pub const AFTER_HINT: Option<regex::Regex> = None;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    // ok: boundary-rule1-domain-imports
    use regex::Regex;

    #[test]
    fn round_trip() {
        // Unbalanced braces in strings, chars and comments: { {
        let open = "{";
        let close = '}';
        let lifetime: &'static str = "x";
        let continued = "first line.\n\
            second line with a { brace";
        let raw_backslash = r"C:\";
        let raw_hashes = r#"a "quoted" { brace"#;
        if true {
            // ok: boundary-rule1-domain-inline-paths
            let json = serde_json::to_string(&1).expect("serialize");
        }
    }
}

// Code after a test module is library code again.
// ruleid: boundary-rule1-domain-inline-paths
pub fn after_tests() -> Option<regex::Regex> {
    None
}
