//! Machine-readable metadata for release packaging.
//!
//! This binary is intentionally tiny: the release workflow supplies the
//! package version and target label, while Rust computes the executable
//! digest through the repository's independently checked SHA-256 surface.

use std::path::PathBuf;

fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                use std::fmt::Write;
                write!(&mut out, "\\u{:04x}", c as u32).expect("write to String");
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn parse_args() -> Result<(PathBuf, String, String), String> {
    let mut binary = None;
    let mut target = None;
    let mut version = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("{arg} requires a value"))?;
        match arg.as_str() {
            "--binary" => binary = Some(PathBuf::from(value)),
            "--target" => target = Some(value),
            "--version" => version = Some(value),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok((
        binary.ok_or("--binary is required")?,
        target.ok_or("--target is required")?,
        version.ok_or("--version is required")?,
    ))
}

fn run() -> Result<(), String> {
    let (binary, target, version) = parse_args()?;
    let bytes = std::fs::read(&binary)
        .map_err(|e| format!("cannot read release binary {}: {e}", binary.display()))?;
    let digest = vh_digest::sha256_hex(&bytes);
    println!(
        "{{\"schema\":\"vh-release-metadata-v1\",\"package\":\"vh\",\"version\":{},\"target\":{},\"binary\":{},\"sha256\":\"{}\",\"bytes\":{}}}",
        json_string(&version),
        json_string(&target),
        json_string(
            binary
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or("binary file name is not valid UTF-8")?
        ),
        digest,
        bytes.len()
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("release-metadata: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::json_string;

    #[test]
    fn json_string_escapes_boundary_text() {
        assert_eq!(json_string("a\"b\\c\n"), "\"a\\\"b\\\\c\\n\"");
    }
}
