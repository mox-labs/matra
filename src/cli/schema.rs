//! `matra --schema`, the published JSON Schemas.
//!
//! The schemas are compiled in from `spec/schema/v2/`, the files every
//! binding's output is validated against, so the program prints the
//! contract of the version that is installed rather than a copy that may
//! have drifted from it. A schema is how a program learns what a command
//! returns without reading matra's source.
//!
//! Three shapes, all exit 0:
//!
//! ```text
//! matra --schema              every schema, one per line, name then title
//! matra --schema <name>       that schema, verbatim
//! matra --schema <name> --json  that schema inside the envelope
//! ```
//!
//! An unknown name is exit 2 and names the ones that exist. Neither the
//! model nor any input is touched.

use std::io::Write;

use serde_json::Value;

use super::{Cli, Failure, Fallible, Outcome, write_envelope};

/// Every schema, by the name `--schema <name>` takes: the file name
/// without `.schema.json`, in file-name order, which is the order the
/// list prints. A test holds the table to the directory.
const SCHEMAS: &[(&str, &str)] = &[
    (
        "blocks",
        include_str!("../../spec/schema/v2/blocks.schema.json"),
    ),
    (
        "clusters",
        include_str!("../../spec/schema/v2/clusters.schema.json"),
    ),
    (
        "document",
        include_str!("../../spec/schema/v2/document.schema.json"),
    ),
    (
        "error",
        include_str!("../../spec/schema/v2/error.schema.json"),
    ),
    (
        "keyphrases",
        include_str!("../../spec/schema/v2/keyphrases.schema.json"),
    ),
    (
        "summary",
        include_str!("../../spec/schema/v2/summary.schema.json"),
    ),
];

/// One row of `--schema`.
#[derive(serde::Serialize)]
struct Entry<'a> {
    name: &'a str,
    title: String,
}

/// The `result` of `--schema`.
#[derive(serde::Serialize)]
struct List<'a> {
    schemas: Vec<Entry<'a>>,
}

/// The `result` of `--schema <name>`.
#[derive(serde::Serialize)]
struct Named<'a> {
    name: &'a str,
    schema: Value,
}

pub(super) fn run(cli: &Cli, name: Option<&str>, out: &mut dyn Write) -> Fallible<Outcome> {
    match name {
        None => list(cli, out),
        Some(name) => one(cli, name, out),
    }
}

/// One schema, verbatim or inside the envelope.
///
/// `--quiet` does not apply: the schema is what the command produces
/// rather than a rendering of a result, as the skill's text is.
fn one(cli: &Cli, name: &str, out: &mut dyn Write) -> Fallible<Outcome> {
    let text = lookup(name)?;
    if cli.json {
        let schema = parsed(name, text)?;
        write_envelope(out, "schema", None, Named { name, schema })?;
    } else {
        // `write_all`, so what arrives is the file's own bytes.
        out.write_all(text.as_bytes())?;
    }
    Ok(Outcome::Found)
}

/// The list: name, then the title the schema itself declares.
fn list(cli: &Cli, out: &mut dyn Write) -> Fallible<Outcome> {
    let mut entries = Vec::with_capacity(SCHEMAS.len());
    for (name, text) in SCHEMAS {
        entries.push(Entry {
            name,
            title: title_of(name, text)?,
        });
    }

    if cli.json {
        write_envelope(out, "schema", None, List { schemas: entries })?;
        return Ok(Outcome::Found);
    }
    let width = entries.iter().map(|e| e.name.len()).max().unwrap_or(0);
    for entry in &entries {
        writeln!(out, "{:<width$}  {}", entry.name, entry.title)?;
    }
    Ok(Outcome::Found)
}

/// The text behind a name, or a refusal that names the alternatives.
fn lookup(name: &str) -> Fallible<&'static str> {
    match SCHEMAS.iter().find(|(known, _)| *known == name) {
        Some((_, text)) => Ok(text),
        None => Err(Failure::refused(format!(
            "no schema named `{name}`; known schemas: {}",
            SCHEMAS
                .iter()
                .map(|(name, _)| *name)
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

/// A schema as JSON. Each is checked to parse by the tests below, so the
/// failure is for a file that has not finished being written.
fn parsed(name: &str, text: &str) -> Fallible<Value> {
    serde_json::from_str(text)
        .map_err(|e| Failure::refused(format!("schema `{name}` is not valid JSON: {e}")))
}

/// The `title` a schema declares.
fn title_of(name: &str, text: &str) -> Fallible<String> {
    parsed(name, text)?
        .get("title")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| Failure::refused(format!("schema `{name}` declares no title")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// The directory is the table. A schema added to `spec/schema/v2/`
    /// without an entry here would be published and never printed.
    #[test]
    fn the_table_is_the_directory() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("spec/schema/v2");
        let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
            .map(|entry| entry.expect("dir entry").file_name())
            .filter_map(|name| {
                name.to_str()
                    .and_then(|n| n.strip_suffix(".schema.json"))
                    .map(str::to_string)
            })
            .collect();
        on_disk.sort();
        let embedded: Vec<String> = SCHEMAS.iter().map(|(n, _)| (*n).to_string()).collect();
        assert_eq!(embedded, on_disk);

        for (name, text) in SCHEMAS {
            let on_disk = std::fs::read_to_string(dir.join(format!("{name}.schema.json")))
                .unwrap_or_else(|e| panic!("read {name}: {e}"));
            assert_eq!(*text, on_disk, "`{name}` is embedded from the wrong file");
        }
    }

    #[test]
    fn every_schema_parses_and_declares_a_title() {
        for (name, text) in SCHEMAS {
            let title = title_of(name, text).unwrap_or_else(|e| panic!("{e}"));
            assert!(!title.is_empty(), "`{name}` has an empty title");
        }
    }

    #[test]
    fn an_unknown_name_names_the_known_ones() {
        let message = lookup("documnet").expect_err("no such schema").to_string();
        assert!(message.contains("documnet"), "{message}");
        for (name, _) in SCHEMAS {
            assert!(message.contains(name), "{message} omits `{name}`");
        }
    }
}
