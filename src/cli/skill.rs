//! `matra --skill`, the agent-facing surface.
//!
//! The text an agent needs is embedded in the program it describes, so
//! the instructions always match the installed version rather than a
//! copy that may have drifted (ADR-0012). The same files under
//! `skills/matra/` are what a plugin distributes, so there is one source
//! and not two.
//!
//! Three shapes, all exit 0:
//!
//! ```text
//! matra --skill              SKILL.md, verbatim
//! matra --skill -r           every reference, one per line, name then summary
//! matra --skill -r <name>    that reference, verbatim
//! ```
//!
//! An unknown name is exit 2 and names the ones that exist.
//!
//! Neither the model nor any input is touched, which is why this is
//! dispatched before the engine is built, ahead even of the subcommand.

use std::io::Write;

use super::{Cli, Fallible, Outcome, write_envelope};

/// The top level, embedded. `--skill` writes these bytes and nothing
/// else, so the output is byte-identical to the file on disk and to the
/// other launcher.
const SKILL: &str = include_str!("../../skills/matra/SKILL.md");

/// The name the top level answers to in `--json`. It is not a reference,
/// so it cannot collide with one.
const SKILL_NAME: &str = "SKILL";

/// Every reference, in the order `--skill -r` lists them: alphabetical by
/// file name, which is also the name a reader passes back.
///
/// The name is the file name because that is the pair `-r <name>` looks
/// up. The summary is not here: it is parsed from each file's own
/// frontmatter by [`summary_of`], so the list a reader sees cannot drift
/// from the file it points at. A test below asserts this table's names
/// against the directory, so a reference added without an entry here
/// fails the suite rather than going missing from the list.
const REFERENCES: &[(&str, &str)] = &[
    (
        "errors",
        include_str!("../../skills/matra/references/errors.md"),
    ),
    (
        "json",
        include_str!("../../skills/matra/references/json.md"),
    ),
    (
        "metrics",
        include_str!("../../skills/matra/references/metrics.md"),
    ),
    (
        "python",
        include_str!("../../skills/matra/references/python.md"),
    ),
    (
        "semantic",
        include_str!("../../skills/matra/references/semantic.md"),
    ),
    (
        "structure",
        include_str!("../../skills/matra/references/structure.md"),
    ),
];

/// One row of `--skill -r`.
#[derive(serde::Serialize)]
struct Entry<'a> {
    name: &'a str,
    summary: &'a str,
}

/// The `result` of `--skill` and of `--skill -r <name>`.
#[derive(serde::Serialize)]
struct Text<'a> {
    name: &'a str,
    body: &'a str,
}

/// The `result` of `--skill -r`.
#[derive(serde::Serialize)]
struct List<'a> {
    references: Vec<Entry<'a>>,
}

pub(super) fn run(cli: &Cli, out: &mut dyn Write) -> Fallible<Outcome> {
    match cli.reference.as_ref() {
        // `--skill`
        None => text(cli, out, SKILL_NAME, SKILL),
        // `--skill -r`
        Some(None) => list(cli, out),
        // `--skill -r <name>`
        Some(Some(name)) => {
            let body = lookup(name)?;
            text(cli, out, name, body)
        }
    }
}

/// One document, verbatim or wrapped in the envelope.
///
/// `--quiet` does not apply, as it does not to `completions`: the text is
/// what the command produces rather than a rendering of a result, and a
/// quiet skill is an empty one.
fn text(cli: &Cli, out: &mut dyn Write, name: &str, body: &str) -> Fallible<Outcome> {
    if cli.json {
        write_envelope(out, "skill", None, Text { name, body })?;
    } else {
        // `write_all` rather than `write!`, so what arrives is the file's
        // own bytes with nothing appended. The parity tests in
        // `tests/cli.rs` and `python/tests/test_cli.py` compare this
        // output against the file itself.
        out.write_all(body.as_bytes())?;
    }
    Ok(Outcome::Found)
}

/// The reference list: name, then the summary its frontmatter declares.
fn list(cli: &Cli, out: &mut dyn Write) -> Fallible<Outcome> {
    let mut entries = Vec::with_capacity(REFERENCES.len());
    for (name, body) in REFERENCES {
        entries.push(Entry {
            name,
            summary: summary_of(name, body)?,
        });
    }

    if cli.json {
        write_envelope(
            out,
            "skill",
            None,
            List {
                references: entries,
            },
        )?;
        return Ok(Outcome::Found);
    }

    let width = entries.iter().map(|e| e.name.len()).max().unwrap_or(0);
    for entry in &entries {
        writeln!(out, "{:<width$}  {}", entry.name, entry.summary)?;
    }
    Ok(Outcome::Found)
}

/// The text behind a name, or a refusal that names the alternatives.
///
/// A misspelled name is the one mistake this command invites, and an
/// agent that gets the list back can correct itself in one step.
fn lookup(name: &str) -> Fallible<&'static str> {
    match REFERENCES.iter().find(|(known, _)| *known == name) {
        Some((_, body)) => Ok(body),
        None => Err(format!("no reference named `{name}`; known references: {}", names()).into()),
    }
}

fn names() -> String {
    REFERENCES
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join(", ")
}

/// The `summary:` line of a reference's frontmatter.
///
/// Parsed from the embedded text rather than held in a second table, so
/// the summary a reader sees is the one the file declares. The failure is
/// loud rather than an empty column: a reference with no summary is a
/// file that has not finished being written, and the unit test below
/// catches it before a reader can.
fn summary_of<'a>(name: &str, body: &'a str) -> Fallible<&'a str> {
    let mut lines = body.lines();
    if lines.next() != Some("---") {
        return Err(format!("reference `{name}` does not open with frontmatter").into());
    }
    for line in lines {
        if line == "---" {
            break;
        }
        if let Some(summary) = line.strip_prefix("summary:") {
            let summary = summary.trim();
            if summary.is_empty() {
                break;
            }
            return Ok(summary);
        }
    }
    Err(format!("reference `{name}` declares no summary in its frontmatter").into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// The directory is the list. A reference file added without an
    /// `include_str!` here would be invisible to `--skill -r` and to
    /// every crate and wheel that ships, so the files on disk are read at
    /// test time and compared against the table.
    #[test]
    fn the_table_is_the_directory() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("skills/matra/references");
        let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
            .map(|entry| entry.expect("dir entry").path())
            .filter(|path| path.extension().is_some_and(|e| e == "md"))
            .map(|path| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .expect("utf8 file name")
                    .to_string()
            })
            .collect();
        on_disk.sort();

        let embedded: Vec<String> = REFERENCES.iter().map(|(n, _)| (*n).to_string()).collect();
        assert_eq!(
            embedded,
            on_disk,
            "the embedded reference table and {} disagree",
            dir.display()
        );

        // Names alone would pass with two entries swapped, so each
        // embedded text is checked against the file it claims to be.
        for (name, body) in REFERENCES {
            let path = dir.join(format!("{name}.md"));
            let on_disk =
                std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {name}: {e}"));
            assert_eq!(*body, on_disk, "`{name}` is embedded from the wrong file");
        }
    }

    /// Every embedded reference carries the summary the list prints, so
    /// the error path in `summary_of` cannot reach a reader.
    #[test]
    fn every_reference_declares_a_summary() {
        for (name, body) in REFERENCES {
            let summary = summary_of(name, body).unwrap_or_else(|e| panic!("{e}"));
            assert!(!summary.is_empty(), "`{name}` has an empty summary");
        }
    }

    /// The top level is the file, byte for byte. `--skill` writes this
    /// constant unchanged, which is the whole of the verbatim promise on
    /// the Rust side.
    #[test]
    fn the_top_level_is_the_file() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("skills/matra/SKILL.md");
        let on_disk = std::fs::read_to_string(&path).expect("read SKILL.md");
        assert_eq!(SKILL, on_disk);
    }

    /// A frontmatter block without a summary is refused rather than
    /// printed as a blank column.
    #[test]
    fn a_reference_without_a_summary_is_refused() {
        let err = summary_of("x", "---\nname: x\n---\n\n# X\n").expect_err("no summary");
        assert!(err.to_string().contains("declares no summary"), "{err}");

        let err = summary_of("x", "# X\n").expect_err("no frontmatter");
        assert!(err.to_string().contains("frontmatter"), "{err}");
    }

    /// An unknown name says which names exist.
    #[test]
    fn an_unknown_name_names_the_known_ones() {
        let err = lookup("jsn").expect_err("no such reference");
        let message = err.to_string();
        assert!(message.contains("jsn"), "{message}");
        for (name, _) in REFERENCES {
            assert!(message.contains(name), "{message} omits `{name}`");
        }
    }
}
