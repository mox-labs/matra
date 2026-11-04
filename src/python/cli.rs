//! `cli_main`: the command line, reached from `python/matra/cli.py`.
//!
//! A launcher only. It collects arguments, calls [`crate::cli::run`], and
//! hands the bytes back to Python's own streams, so one program answers
//! to the Rust binary and to the Python entry point alike.

use pyo3::prelude::*;

/// Run the matra command line and return its exit code.
///
/// `argv` is the argument vector *without* the program name, which
/// `cli::run` supplies itself so `--help` reads the same from this
/// launcher and from the Rust binary.
///
/// The command renders into byte buffers, which are then written to
/// Python's own `sys.stdout` and `sys.stderr` and flushed. Handing
/// `cli::run` the real file descriptors would interleave badly with
/// anything Python has already buffered on the same stream; going
/// through `sys` keeps one writer per stream.
///
/// A failure writing those buffers out is swallowed rather than
/// raised. It is a broken pipe in practice (`matra ... | head`), and
/// the Rust launcher treats that as success too; there is nowhere
/// left to report it to in any case.
///
/// The arguments arrive as arbitrary Python objects and go through
/// `os.fsencode`, not through `String`. A `sys.argv` entry is not
/// guaranteed to be text: on unix a path is bytes, and Python decodes
/// an undecodable one with surrogate escapes rather than failing.
/// Extracting a `String` would reject exactly those arguments, so a
/// file the Rust binary opens without comment would be unreachable
/// through this launcher. `os.fsencode` reverses the escape and hands
/// back the original bytes, which become an `OsString` unchanged. A
/// conversion failure is raised rather than swallowed: an argument
/// that is not a string, bytes, or a path is a caller's error, and
/// silently dropping it would run a different command than the one
/// that was asked for.
#[pyfunction]
#[allow(unreachable_pub)] // pyo3 macro requirement, same as _core below.
pub fn cli_main(py: Python<'_>, argv: Vec<Bound<'_, PyAny>>) -> PyResult<i32> {
    use std::ffi::OsString;

    let fsencode = py.import("os")?.getattr("fsencode")?;
    let mut args: Vec<OsString> = Vec::with_capacity(argv.len() + 1);
    args.push(OsString::from("matra"));
    for arg in argv {
        let bytes: Vec<u8> = fsencode.call1((arg,))?.extract()?;
        args.push(os_string_of(bytes));
    }

    // stdout is buffered and written at the end, so the rendered
    // result arrives as one piece. stderr is written through as it
    // is produced, because the first thing a cold run writes there
    // is "downloading the model", and a diagnostic that explains a
    // twenty-second wait is worthless once the wait is over.
    let mut out: Vec<u8> = Vec::new();
    let mut err = PyStream { py, name: "stderr" };
    let code = crate::cli::run(args, &mut out, &mut err);

    let _ = write_stream(py, "stdout", &out);
    Ok(i32::from(code))
}

/// A [`std::io::Write`] that forwards each write straight to a `sys`
/// stream instead of accumulating it.
///
/// The GIL is held for the whole of `cli_main`, so calling back into
/// Python here is the same access the surrounding function already
/// has.
struct PyStream<'py> {
    py: Python<'py>,
    name: &'static str,
}

impl std::io::Write for PyStream<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        write_stream(self.py, self.name, buf)
            .map(|()| buf.len())
            .map_err(|e| std::io::Error::other(e.to_string()))
    }

    /// `write_stream` flushes the Python stream on every write, so
    /// there is nothing held here to flush.
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// The bytes `os.fsencode` produced, as an `OsString`.
///
/// On unix an `OsString` is bytes and the filesystem is bytes, so
/// this is exact: the argument reaches `cli::run` as the same bytes
/// `std::env::args_os` would have delivered to the Rust binary.
#[cfg(unix)]
fn os_string_of(bytes: Vec<u8>) -> std::ffi::OsString {
    use std::os::unix::ffi::OsStringExt;
    std::ffi::OsString::from_vec(bytes)
}

/// Elsewhere there is no lossless byte route into an `OsString`, so
/// an argument that is not valid UTF-8 loses the offending bytes to
/// the replacement character rather than reaching the command line
/// intact. Naming the limit here is the honest version of it.
#[cfg(not(unix))]
fn os_string_of(bytes: Vec<u8>) -> std::ffi::OsString {
    std::ffi::OsString::from(String::from_utf8_lossy(&bytes).into_owned())
}

/// Say on `sys.stderr` that a model is about to download, in the two
/// lines the command line writes ([`crate::cli::notice_text`]).
///
/// A first `matra.Engine()` waits seconds on a 16 MB fetch, and the first
/// `Model2Vec.potion_base_8m()` on 30.9 MB; before this, both waits were
/// silent. Model libraries in Python announce a download on stderr the
/// same way, and a caller who wants quiet redirects the stream. A notice
/// that cannot be written is not worth failing the constructor over.
pub(super) fn announce(py: Python<'_>, notice: &crate::domain::ProvisionNotice) {
    let _ = write_stream(py, "stderr", crate::cli::notice_text(notice).as_bytes());
}

/// Write one rendered buffer to the named `sys` stream and flush it.
fn write_stream(py: Python<'_>, name: &str, bytes: &[u8]) -> PyResult<()> {
    if bytes.is_empty() {
        return Ok(());
    }
    let stream = py.import("sys")?.getattr(name)?;
    // The renderers only emit UTF-8; `from_utf8_lossy` is the belt to
    // that suspenders, so a surprise byte degrades one character
    // rather than losing the whole run's output.
    stream.call_method1("write", (String::from_utf8_lossy(bytes).as_ref(),))?;
    stream.call_method0("flush")?;
    Ok(())
}
