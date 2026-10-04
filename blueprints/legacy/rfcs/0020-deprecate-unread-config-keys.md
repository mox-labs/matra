# RFC-0020: Deprecate the config keys nothing reads

- Feature Name: `deprecate_unread_config_keys`
- Start Date: 2026-10-02
- RFC PR: [#128](https://github.com/mox-labs/matra/pull/128)
- Tracking EP: none
- Status: accepted
- Decider(s): project owner (2026-10-02, the decision), maintainer (the shape)

## Summary

`models.udpipe` and `semantic.threshold` stop shipping in
`config/default.toml`, so `matra config init` no longer writes them and
`matra config show` no longer lists them. A config file that still sets
either keeps loading: the keys are accepted, deprecated, and ignored, and
`config show` names each one it finds on stderr. In Rust,
`Config::udpipe_model` and `Config::semantic_threshold` are deprecated, and
`Config::deprecated_keys` reports which deprecated keys the file set.

## Motivation

RFC-0011 gave `Config` five defaults besides the paths. Two of them never
selected anything. The UDPipe model file is pinned in `nlp/udpipe.rs`
beside its digest, so `models.udpipe` names no file anyone opens; every
clustering call takes its threshold as an argument and there is no
clustering command, so `semantic.threshold` reaches no call. Only
`config show` read them, and it printed them as though they were settings.

The cost was documentation that had to explain, on four pages and in the
agent skill, that two of the printed values change nothing, and a user who
edits `semantic.threshold` and reasonably expects some clustering to move.
The standards pass (#127) corrected every page that presented them as
settings; this removes the reason the pages needed the caveat.

They cannot simply be deleted from the schema. The file parser rejects
unknown keys (`deny_unknown_fields`), and every user who ran `config init`
on 0.2.x has a file containing both. Deleting the fields would turn that
file into an `InvalidInput` error on the next run.

## Guide-level explanation

A fresh `matra config init` writes the summary and keyphrase defaults and
`models.embedding`, and nothing else. `matra config show` lists `data_dir`,
`model_dir`, `models.embedding` and the four summary and keyphrase keys.

A config file written by 0.2.x still loads. When it sets either deprecated
key, `config show` prints the settings as above and adds a line on stderr:

```text
matra: /home/you/.config/matra/config.toml sets `semantic.threshold`, which is deprecated and ignored; every clustering call takes its threshold as an argument. Remove the line.
```

`--json` keeps stdout one object, without the deprecated keys; the note
still goes to stderr. `--quiet` silences it, as it silences the download
notice. No other command prints it, because `config show` is where matra
reports where its configuration came from.

In Rust, `cfg.semantic_threshold()` and `cfg.udpipe_model()` still compile
and still return the file's value, or 0.85 and
`english-ewt-ud-2.5-191206` when the file sets none, with a deprecation
warning at the call site. A program that read the threshold from the config
file passes the number itself instead.

## Reference-level explanation

- `config/default.toml` loses `[models] udpipe` and the `[semantic]`
  table. `config init` writes that file, so it follows.
- `FileConfig` keeps both fields as `Option`, so `deny_unknown_fields`
  still accepts them. A value of the wrong type is still `InvalidInput`,
  and a non-finite `semantic.threshold` is still refused, as in 0.2.x.
- `Config::sources` no longer yields `models.udpipe` or
  `semantic.threshold`. Every key it yields is one something reads.
- `Config::deprecated_keys(&self) -> impl Iterator<Item = &'static str>`
  yields the deprecated keys the config file set, in a fixed order.
- `Config::udpipe_model` and `Config::semantic_threshold` carry
  `#[deprecated]`. They return the file's value, or the former default.
- `matra config show` writes one stderr line per deprecated key, naming
  the file, unless `--quiet` is set. Its stdout, text and JSON, lists the
  keys `Config::sources` yields.

The CLI reaches all of this through `config`, which boundary rule 7 allows.
Tests: a 0.2.x config file loads, reports both keys as deprecated and keeps
them out of `sources` (`src/config.rs`); `config init` writes neither key,
and `config show` over an old file exits 0, omits them from stdout and
names them on stderr (`tests/cli.rs`).

The removal of the two accessors and the two fields is not scheduled here.
It is a breaking change and takes its own RFC.

## Drawbacks

A deprecated key is accepted and ignored, which is the silence
`deny_unknown_fields` exists to prevent. The stderr note narrows it to the
one command that reports configuration, rather than closing it: a user who
never runs `config show` is not told. Printing it on every command would
put a line on stderr of every analysis run by anyone with a 0.2.x file.

`config show --json` loses two keys. A program that read them gets nothing
where it got a value. Nothing in matra read them, and the values were never
in force.

## Rationale and alternatives

- **Delete the keys from the schema.** Breaks every 0.2.x config file at
  load. Rejected.
- **Accept them silently.** Smallest change, but a user keeps editing a key
  that does nothing, with no signal anywhere. The note costs one loop in
  `config show`.
- **Make the keys act.** `models.udpipe` would select a model file whose
  digest matra does not pin, which RFC-0015's provisioning discipline
  rules out. A default threshold in `Config` would let the configuration
  decide what a clustering call computes, which RFC-0011 rules out.
- **Keep listing them in `config show`, marked deprecated.** That adds a
  field to the JSON result and a marker to the text, both new shapes, to
  display values that are not in force.

## Prior art

Cargo warns about unused manifest keys on stderr and keeps building
(`warning: unused manifest key`). RFC-0011 set the per-key provenance that
`config show` prints; this keeps it, and narrows it to keys that act.

## Unresolved questions

None.

## Future possibilities

Removing the two accessors and fields in a later breaking release, by a
new RFC.
