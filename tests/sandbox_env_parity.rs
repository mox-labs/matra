//! The sandbox script names matra's resolution tiers by hand. This is the
//! mechanical link that keeps the transcription honest.
//!
//! `scripts/e2e-sandbox.sh snapshot` fingerprints the union of every location
//! any resolution tier could name, and it names those tiers as a hand-written
//! list of environment variables. There is no compiler between that list and
//! the resolvers in `src/config.rs`, and the transcription has drifted twice.
//! Both times the effect was the same: a tier went unwatched, and the snapshot
//! reported a clean pair for a location it had never looked at.
//!
//! So this asserts that the set of `MATRA_*`, `XDG_*` and `HOME` variables the
//! resolvers actually read is exactly the set the script names. Adding a tier
//! to `src/config.rs` now fails a gate instead of quietly narrowing the union.
//!
//! It lives in `tests/` rather than in the shell test script because
//! `cargo test` runs on every push and `scripts/test-e2e-sandbox.sh` runs from
//! `just check`, which no CI workflow invokes. The whole diagnosis behind this
//! pass was "no test and no gate", and a check that fires only when someone
//! types `just check` is the weaker half of that. The behavioural cases stay
//! in the shell script, where they can build unreadable directories and
//! symlinks; this one is pure text and belongs where the pushes are.
//!
//! The shape is the one `tests/skill.rs` already uses: assert the same fact two
//! ways, so a broken extractor cannot pass by finding nothing on both sides.

use std::collections::BTreeSet;
use std::path::Path;

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

/// The sentinels in `scripts/e2e-sandbox.sh` that bracket the union list.
/// Reading between them rather than scanning the whole file keeps the
/// variables the sandbox *exports* (`XDG_CACHE_HOME`, which no resolver reads)
/// out of a comparison that is only about what gets fingerprinted.
const BLOCK_START: &str = "# parity: union targets begin";
const BLOCK_END: &str = "# parity: union targets end";

/// The floor. Six today: three `MATRA_*`, two `XDG_*` and `HOME`. An
/// extractor that stopped matching would return an empty set on both sides and
/// the equality assertion alone would pass, so the count is asserted too.
const MINIMUM_VARIABLES: usize = 6;

fn read(relative: &str) -> String {
    let path = Path::new(MANIFEST_DIR).join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()))
}

/// Whether a name is one of the environment variables matra resolves through.
fn is_resolution_variable(name: &str) -> bool {
    name == "HOME" || name.starts_with("MATRA_") || name.starts_with("XDG_")
}

/// Every variable read through the `env` closure the resolvers in
/// `src/config.rs` are handed.
///
/// The resolvers take `&dyn Fn(&str) -> Option<String>` so a test can inject
/// an environment, which is what makes them readable this way: every real
/// lookup is spelled `env("NAME")`. The unit tests in the same file build
/// their fixtures with `env_of(&[("NAME", ...)])`, a different spelling, so a
/// variable that only a test mentions does not count as one the library reads.
fn variables_read_by_the_resolvers() -> BTreeSet<String> {
    let source = read("src/config.rs");
    let mut found = BTreeSet::new();
    for (index, _) in source.match_indices("env(\"") {
        // `non_empty(env("HOME"))` counts; a hypothetical `read_env("HOME")`
        // does not, because the character before the match is part of a longer
        // identifier.
        if index > 0 {
            let before = source[..index].chars().next_back().unwrap_or(' ');
            if before.is_ascii_alphanumeric() || before == '_' {
                continue;
            }
        }
        let rest = &source[index + "env(\"".len()..];
        let Some(end) = rest.find('"') else { continue };
        let name = &rest[..end];
        if is_resolution_variable(name) {
            found.insert(name.to_string());
        }
    }
    found
}

/// Every variable named inside the union block of `scripts/e2e-sandbox.sh`.
fn variables_named_by_the_sandbox_script() -> BTreeSet<String> {
    let script = read("scripts/e2e-sandbox.sh");
    let start = script
        .find(BLOCK_START)
        .unwrap_or_else(|| panic!("scripts/e2e-sandbox.sh has no `{BLOCK_START}` sentinel"));
    let end = script[start..]
        .find(BLOCK_END)
        .unwrap_or_else(|| panic!("scripts/e2e-sandbox.sh has no `{BLOCK_END}` sentinel"))
        + start;

    let mut found = BTreeSet::new();
    // `$NAME` and `${NAME:-}` are the only two spellings the block uses.
    for piece in script[start..end].split('$').skip(1) {
        let piece = piece.strip_prefix('{').unwrap_or(piece);
        let name: String = piece
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if is_resolution_variable(&name) {
            found.insert(name);
        }
    }
    found
}

#[test]
fn the_sandbox_script_names_every_tier_the_resolvers_read() {
    let read_by_matra = variables_read_by_the_resolvers();
    let named_by_script = variables_named_by_the_sandbox_script();

    let unwatched: Vec<_> = read_by_matra.difference(&named_by_script).collect();
    let stale: Vec<_> = named_by_script.difference(&read_by_matra).collect();

    assert!(
        unwatched.is_empty(),
        "src/config.rs resolves through {unwatched:?}, which \
         scripts/e2e-sandbox.sh does not fingerprint. A pass could write there \
         and the before/after diff would be clean. Add the target to the union \
         block between the parity sentinels."
    );
    assert!(
        stale.is_empty(),
        "scripts/e2e-sandbox.sh fingerprints {stale:?}, which no resolver in \
         src/config.rs reads. Either a resolver was removed and the script was \
         not updated, or the script is walking a tree matra never touches."
    );
}

#[test]
fn the_extraction_is_not_vacuous() {
    let read_by_matra = variables_read_by_the_resolvers();
    let named_by_script = variables_named_by_the_sandbox_script();

    assert!(
        read_by_matra.len() >= MINIMUM_VARIABLES,
        "found only {} resolution variables in src/config.rs ({read_by_matra:?}); \
         expected at least {MINIMUM_VARIABLES}. Either a tier was deleted, or \
         the resolvers stopped spelling their lookups `env(\"NAME\")` and this \
         test now reads nothing while appearing to pass.",
        read_by_matra.len(),
    );
    assert!(
        named_by_script.len() >= MINIMUM_VARIABLES,
        "found only {} variables in the union block of scripts/e2e-sandbox.sh \
         ({named_by_script:?}); expected at least {MINIMUM_VARIABLES}.",
        named_by_script.len(),
    );
    // HOME is the tier both sides are most likely to drop, because it is the
    // implicit one: it is the last fallback in every resolver and it is spelled
    // without a prefix in the script.
    assert!(
        read_by_matra.contains("HOME"),
        "src/config.rs stopped reading HOME"
    );
    assert!(
        named_by_script.contains("HOME"),
        "the union block stopped naming HOME"
    );
}
