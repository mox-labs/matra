# RECOVERED-FROM-READ source=[claude-project-path]/[session-id]/subagents/[agent-transcript].jsonl timestamp=2026-04-09T13:02:36.636Z original_path=[path]/src/source/mod.rs
//! Source port. Reads documents from paths.
//!
//! This module contains the Source trait (port). Adapters live in
//! submodules: file.rs, directory.rs.

pub mod directory;
pub mod file;

use std::path::Path;

use crate::domain;

/// Reads documents from a path. Paths only, not strings.
pub trait Source: Send {
    fn read(&self, input: &Path) -> domain::Result<Vec<domain::RawDocument>>;
    fn accepts(&self, input: &Path) -> bool;
}

[result-id: r7]