//! `matra config show` and `matra config init`.
//!
//! Neither action touches the model or any input document, which is why
//! both are dispatched before the engine is built.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::config::{Config, ValueSource};

use super::{Cli, ConfigAction, Fallible, Outcome, write_envelope};

/// The defaults, embedded a second time so `config init` can write the
/// exact bytes the crate falls back to. It is the same file
/// `config::DEFAULT_TOML` reads, so the two cannot disagree.
const DEFAULT_TOML: &str = include_str!("../../config/default.toml");

pub(super) fn run(cli: &Cli, action: &ConfigAction, out: &mut dyn Write) -> Fallible<Outcome> {
    match action {
        ConfigAction::Show => show(cli, out),
        ConfigAction::Init { force } => init(cli, *force, out),
    }
}

// ---------------------------------------------------------------------------
// show
// ---------------------------------------------------------------------------

fn show(cli: &Cli, out: &mut dyn Write) -> Fallible<Outcome> {
    let cfg = super::resolve_config(cli)?;
    let path = Config::config_file_path().unwrap_or_else(|| PathBuf::from("config.toml"));
    let path = path.display().to_string();

    if cli.json {
        let mut object = serde_json::Map::new();
        for (key, source) in cfg.sources() {
            object.insert(
                key.to_string(),
                json!({
                    "value": value_of(&cfg, key)?,
                    "source": kind_of(&source),
                    "origin": origin_of(&source),
                }),
            );
        }
        write_envelope(out, "config", &path, Value::Object(object))?;
        return Ok(Outcome::Found);
    }

    if cli.quiet {
        return Ok(Outcome::Found);
    }

    // One key per line, the value, then the origin as a comment, which is
    // the shape `cargo config get --show-origin` prints.
    for (key, source) in cfg.sources() {
        writeln!(
            out,
            "{key} = {} # {}",
            value_of(&cfg, key)?,
            describe(&source)
        )?;
    }
    Ok(Outcome::Found)
}

/// The effective value behind one of [`Config::sources`]'s keys.
///
/// The unknown arm is loud rather than skipped: a key that `Config`
/// reports and this function does not know is a key `config show` would
/// silently omit, and a value nobody can see is a value nobody can
/// debug. The test below walks every key `Config` reports, so the gap
/// fails the suite rather than the user's terminal.
fn value_of(cfg: &Config, key: &str) -> Fallible<Value> {
    Ok(match key {
        "data_dir" => json!(cfg.data_dir().display().to_string()),
        "model_dir" => json!(cfg.model_dir().display().to_string()),
        "models.udpipe" => json!(cfg.udpipe_model()),
        "models.embedding" => json!(cfg.embedding_model()),
        "semantic.threshold" => json!(decimal(cfg.semantic_threshold())),
        "summarize.n" => json!(cfg.summarize_n()),
        "summarize.algorithm" => json!(cfg.summarize_algorithm()),
        "keyphrases.n" => json!(cfg.keyphrases_n()),
        "keyphrases.algorithm" => json!(cfg.keyphrases_algorithm()),
        other => {
            return Err(format!(
                "config show cannot render `{other}`: the key is resolved but has no reader here"
            )
            .into());
        }
    })
}

/// The `f32` a user wrote, not the `f64` its bits widen to.
///
/// `f64::from(0.85_f32)` is `0.8500000238418579`, a number nobody typed
/// and nobody can compare against their own config file. Going through
/// `f32`'s shortest round-trip decimal returns `0.85`.
fn decimal(value: f32) -> f64 {
    value
        .to_string()
        .parse()
        .unwrap_or_else(|_| f64::from(value))
}

/// The origin, spelled for a person.
fn describe(source: &ValueSource) -> String {
    match source {
        ValueSource::Argument => "command line".to_string(),
        ValueSource::Environment(name) => format!("environment variable `{name}`"),
        ValueSource::File(path) => path.display().to_string(),
        ValueSource::Default => "default".to_string(),
    }
}

/// The origin's rung, spelled for a program. `#[non_exhaustive]` binds
/// outside the crate, not in here, so a new rung fails to compile until
/// it is named.
fn kind_of(source: &ValueSource) -> &'static str {
    match source {
        ValueSource::Argument => "argument",
        ValueSource::Environment(_) => "environment",
        ValueSource::File(_) => "file",
        ValueSource::Default => "default",
    }
}

/// What the rung points at: the variable name, the file path, or nothing.
fn origin_of(source: &ValueSource) -> Value {
    match source {
        ValueSource::Environment(name) => json!(name),
        ValueSource::File(path) => json!(path.display().to_string()),
        ValueSource::Argument | ValueSource::Default => Value::Null,
    }
}

// ---------------------------------------------------------------------------
// init
// ---------------------------------------------------------------------------

fn init(cli: &Cli, force: bool, out: &mut dyn Write) -> Fallible<Outcome> {
    let path = Config::config_file_path()
        .ok_or("cannot locate a config file: set MATRA_CONFIG_FILE, XDG_CONFIG_HOME, or HOME")?;
    write_defaults(&path, force)?;

    let shown = path.display().to_string();
    if cli.json {
        write_envelope(out, "config", &shown, json!({ "path": shown }))?;
    } else if !cli.quiet {
        writeln!(out, "{shown}")?;
    }
    Ok(Outcome::Found)
}

/// Write the shipped defaults to `path`, atomically.
///
/// The bytes land in a temporary file in the same directory and arrive
/// at `path` under one rename, so a concurrent reader sees either the
/// old file or the whole new one and never a half-written config.
///
/// Without `force` the arrival is a hard link instead, because
/// `hard_link` fails when the destination exists and `rename` does not.
/// Checking `exists()` and then renaming would leave a window in which
/// another process creates the file and this one silently destroys it;
/// the link closes that window in the kernel rather than narrowing it.
fn write_defaults(path: &Path, force: bool) -> Fallible<()> {
    // The early check exists for the message, not for the guarantee. The
    // guarantee is the link below.
    if !force && path.exists() {
        return Err(format!(
            "{} already exists; pass --force to overwrite it",
            path.display()
        )
        .into());
    }

    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;

    let temp = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("config.toml"),
        std::process::id()
    ));
    // `create_new` refuses a leftover temp from a crashed run rather than
    // overwriting whatever is there.
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    let written = file
        .write_all(DEFAULT_TOML.as_bytes())
        .and_then(|()| file.sync_all());
    drop(file);
    if let Err(e) = written {
        let _ = std::fs::remove_file(&temp);
        return Err(Box::new(e));
    }

    let arrived = if force {
        std::fs::rename(&temp, path)
    } else {
        std::fs::hard_link(&temp, path).and_then(|()| std::fs::remove_file(&temp))
    };
    if let Err(e) = arrived {
        let _ = std::fs::remove_file(&temp);
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            return Err(format!(
                "{} already exists; pass --force to overwrite it",
                path.display()
            )
            .into());
        }
        return Err(Box::new(e));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every key `Config` resolves must have a reader in `value_of`.
    /// Adding a key to the config without teaching `config show` how to
    /// print it fails here, not in a user's terminal.
    #[test]
    fn every_resolved_key_can_be_rendered() {
        let cfg = Config::from_sources(
            |key| match key {
                "HOME" => Some("/tmp/matra-test-home".to_string()),
                _ => None,
            },
            None,
        )
        .expect("defaults resolve");
        for (key, _) in cfg.sources() {
            value_of(&cfg, key).unwrap_or_else(|e| panic!("{key}: {e}"));
        }
    }

    #[test]
    fn an_unknown_key_is_loud() {
        let cfg = Config::from_sources(
            |key| match key {
                "HOME" => Some("/tmp/matra-test-home".to_string()),
                _ => None,
            },
            None,
        )
        .expect("defaults resolve");
        assert!(value_of(&cfg, "not.a.key").is_err());
    }

    #[test]
    fn defaults_written_once_and_refused_twice() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("nested").join("config.toml");

        write_defaults(&path, false).expect("first write");
        assert_eq!(
            std::fs::read_to_string(&path).expect("read back"),
            DEFAULT_TOML
        );

        let refused = write_defaults(&path, false).expect_err("second write refused");
        assert!(refused.to_string().contains("--force"), "{refused}");

        write_defaults(&path, true).expect("forced write");
        assert_eq!(
            std::fs::read_to_string(&path).expect("read back"),
            DEFAULT_TOML
        );
    }

    /// The temp file is an implementation detail and must not survive the
    /// call, on either the taken or the refused path.
    #[test]
    fn no_temp_file_is_left_behind() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("config.toml");
        write_defaults(&path, false).expect("first write");
        let _ = write_defaults(&path, false);

        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .expect("list")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "left behind: {leftovers:?}");
    }
}
