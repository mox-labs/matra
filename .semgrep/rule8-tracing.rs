//! Fixture for .semgrep/rule8-tracing.yml. A comment about tracing, or
//! `use tracing::info;` in a doc comment, is not an import.

// ok: boundary-rule8-no-tracing
use crate::domain;
// ok: boundary-rule8-no-tracing
let retracing = 1;

// ruleid: boundary-rule8-no-tracing
use tracing::info;
// ruleid: boundary-rule8-no-tracing
use tracing;
// ruleid: boundary-rule8-no-tracing
use {std::fmt, tracing::debug};
// ruleid: boundary-rule8-no-tracing
extern crate tracing;

// ruleid: boundary-rule8-no-tracing
#[tracing::instrument(skip(self))]
fn parse(&self) {
    // ruleid: boundary-rule8-no-tracing
    tracing::debug!("parsed");
}

#[cfg(test)]
mod tests {
    // Test modules are not exempt from rule 8.
    // ruleid: boundary-rule8-no-tracing
    use tracing_test::traced_test as _; use tracing::info;
}
