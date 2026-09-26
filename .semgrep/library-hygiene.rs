//! Fixture for .semgrep/library-hygiene.yml. A doc comment may say
//! `Result<T, String>` or `x.unwrap()` when it explains why the library does
//! not use them.

fn unwraps(v: Option<u8>) -> u8 {
    // ok: library-no-unwrap
    let a = v.unwrap_or(0);
    // ok: library-no-unwrap
    let b = v.unwrap_or_default();
    // ok: library-no-unwrap
    let c = v.ok_or(Error::EmptyInput)?;
    // ruleid: library-no-unwrap
    let d = v.unwrap();
    // ruleid: library-no-unwrap
    let e = v.expect("present");
    // A method chain split across lines is read on the line of the call.
    let f = map
        .get(&k)
        // ruleid: library-no-unwrap
        .expect("key");
    d
}

// ok: library-no-string-errors
pub fn parse(text: &str) -> Result<Vec<Sentence>> {
    todo!()
}
// ok: library-no-string-errors
pub fn pair() -> Result<(u8, String)> {
    todo!()
}
// ok: library-no-string-errors
pub fn nested() -> Result<Vec<String>, Error> {
    todo!()
}
// ok: library-no-string-errors
pub fn map() -> std::result::Result<HashMap<String, String>, domain::Error> {
    todo!()
}

// ruleid: library-no-string-errors
pub fn load(path: &Path) -> Result<Model, String> {
    todo!()
}
// ruleid: library-no-string-errors
fn private() -> std::result::Result<(), String> {
    todo!()
}
// ruleid: library-no-string-errors
pub type Fallible<T> = Result<T, std::string::String>;
// ruleid: library-no-string-errors
fn tuple() -> Result<(u8, String), String> {
    todo!()
}
// ruleid: library-no-string-errors
fn generic() -> Result<HashMap<String, Vec<u8>>, String> {
    todo!()
}
// ruleid: library-no-string-errors
fn static_str() -> Result<u8, &'static str> {
    todo!()
}
// ruleid: library-no-string-errors
fn borrowed<'a>() -> Result<u8, &'a str> {
    todo!()
}
fn body() {
    // ruleid: library-no-string-errors
    let r: Result<u8, String> = Ok(1);
}

#[cfg(test)]
mod tests {
    // ok: library-no-string-errors
    fn helper() -> Result<(), String> {
        // ok: library-no-unwrap
        let x = Some(1).unwrap();
        Ok(())
    }
}
