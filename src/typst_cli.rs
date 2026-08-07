// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::process::Command;

use camino::Utf8Path;
use eyre::{eyre, WrapErr};

pub fn html_to_body_content(html: &str) -> eyre::Result<String> {
    let start_pos = html
        .find("<html")
        .and_then(|pos| html[pos..].find('>').map(|offset| pos + offset + 1))
        .ok_or_else(|| eyre!("missing `<html>` tag in typst html output"))?;
    let end_pos = html
        .rfind("</html>")
        .ok_or_else(|| eyre!("missing `</html>` tag in typst html output"))?;
    if end_pos < start_pos {
        return Err(eyre!("malformed html range in typst html output"));
    }
    let content = &html[start_pos..end_pos];
    Ok(content.to_string())
}

pub fn file_to_html(rel_path: &str, root_dir: &str) -> eyre::Result<String> {
    to_html_string(rel_path, root_dir)
        .and_then(|s| html_to_body_content(&s))
        .wrap_err_with(|| eyre!("failed to extract html body from `{}`", rel_path))
}

fn to_html_string<P: AsRef<Utf8Path>>(rel_path: P, root_dir: P) -> eyre::Result<String> {
    let root_dir = root_dir.as_ref();
    let rel_path = rel_path.as_ref();
    let full_path = root_dir.join(rel_path);
    if !full_path.exists() {
        return Err(eyre!(
            "typst source `{}` not found at `{}` (typst root: `{}`). Check `[build].typst-root` in \"Wanshi.toml\"",
            rel_path,
            full_path,
            root_dir
        ));
    }

    let output = Command::new("typst")
        .arg("c")
        .arg("-f=html")
        .arg("--features=html")
        .arg(format!("--root={}", root_dir))
        .args(["--input", &format!("path={}", rel_path)])
        .args(["--input", &format!("random={}", fastrand::i64(0..))])
        .arg(full_path)
        .arg("-")
        .stdout(std::process::Stdio::piped())
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        failed_in_file(concat!(file!(), '#', line!()), rel_path.as_str(), stderr);
        Err(eyre!(
            "typst html compilation failed for `{}`",
            rel_path.as_str()
        ))
    }
}

fn failed_in_file(src_pos: &'static str, file_path: &str, stderr: std::borrow::Cow<'_, str>) {
    color_print::ceprintln!(
        "<r>Command failed in {}: \n  In file {}, {}</>",
        src_pos,
        file_path,
        stderr
    );
}

#[cfg(test)]
mod tests {
    use super::{html_to_body_content, to_html_string};
    use camino::Utf8Path;
    use std::fs;

    #[test]
    fn test_html_to_body_content_ok() {
        let html = "<html><p>Hello</p></html>";
        let body = html_to_body_content(html).unwrap();
        assert_eq!(body, "<p>Hello</p>");
    }

    #[test]
    fn test_html_to_body_content_with_html_attributes() {
        let html = r#"<html lang="en"><p>Hello</p></html>"#;
        let body = html_to_body_content(html).unwrap();
        assert_eq!(body, "<p>Hello</p>");
    }

    #[test]
    fn test_html_to_body_content_missing_tags() {
        let err = html_to_body_content("<p>Hello</p>");
        assert!(err.is_err());
    }

    #[test]
    fn test_to_html_string_reports_typst_root_hint_for_missing_source() {
        let base = crate::test_io::case_dir("typst-root-hint");
        fs::create_dir_all(&base).unwrap();

        let err = to_html_string(
            Utf8Path::new("references/feature-typst.typst"),
            base.as_path(),
        )
        .unwrap_err();
        let message = format!("{err:#}");
        assert!(message.contains("Check `[build].typst-root`"));
        assert!(message.contains("references/feature-typst.typst"));

        let _ = fs::remove_dir_all(base.as_std_path());
    }
}
