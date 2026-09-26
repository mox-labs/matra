//! Run one docsite example's Rust call (EP-0012, M6).
//!
//! Each worked example in the docsite's Examples section shows the same call
//! in Rust, Python and the CLI. The Rust tab is the file
//! `site/examples/<name>/main.rs`, a whole program a reader can paste into a
//! new project. This binary compiles every one of those files as written, with
//! `include!`, and runs the one it is asked for, in the current directory:
//!
//!   cargo run --example docsite_examples -- <name>
//!
//! `site/scripts/check-examples.ts` runs it from a directory holding the
//! example's input, beside the Python and CLI calls, and compares all three
//! with the output committed beside the example. So a Rust tab that no longer
//! compiles fails the build, and one whose output drifted fails the gate.
//!
//! Adding an example adds a line to `examples!` below; the check fails on an
//! example directory this binary does not know.

use std::process::ExitCode;

type Outcome = Result<(), Box<dyn std::error::Error>>;

/// Each example's program, in a module of its own, reached through `run`
/// because its `main` is private to that module.
macro_rules! examples {
    ($($name:literal => $module:ident),* $(,)?) => {
        $(
            mod $module {
                include!(concat!("../site/examples/", $name, "/main.rs"));

                pub(super) fn run() -> super::Outcome {
                    main()
                }
            }
        )*

        const NAMES: &[&str] = &[$($name),*];

        fn run(name: &str) -> Option<Outcome> {
            match name {
                $($name => Some($module::run()),)*
                _ => None,
            }
        }
    };
}

examples! {
    "parse-a-sentence" => parse_a_sentence,
    "negations-and-modals" => negations_and_modals,
    "readability-by-paragraph" => readability_by_paragraph,
    "summarize" => summarize,
    "keyphrases" => keyphrases,
    "markdown-structure" => markdown_structure,
    "quickstart" => quickstart,
    "first-analysis" => first_analysis,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [name] = args.as_slice() else {
        eprintln!(
            "usage: docsite_examples <name>, one of: {}",
            NAMES.join(", ")
        );
        return ExitCode::from(2);
    };
    match run(name) {
        Some(Ok(())) => ExitCode::SUCCESS,
        Some(Err(e)) => {
            eprintln!("docsite_examples: {name}: {e}");
            ExitCode::FAILURE
        }
        None => {
            eprintln!(
                "docsite_examples: no example called {name:?}; known: {}",
                NAMES.join(", ")
            );
            ExitCode::from(2)
        }
    }
}
