// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.

//! Recovering readable prose from generated HTML.
//!
//! Used wherever a section's text has to leave the page as text rather than
//! markup: RSS summaries and the search index. Both need the same three things,
//! and getting any of them wrong is visible to readers.
//!
//! 1. **Drop elements whose contents are not prose.** Typst renders inline math
//!    and diagrams as `<svg>`, so their coordinates and namespace URLs would
//!    otherwise read as words.
//! 2. **Remove the remaining tags.** The display-side stripper in
//!    [`crate::compiler::section::HTMLContent::remove_all_tags`] is regex-based
//!    and cannot match attribute names containing a colon, which is exactly what
//!    Typst's SVG output emits. This scans instead.
//! 3. **Unescape entities.** Generated HTML contains `&amp;`; text that is about
//!    to be escaped again for XML must be unescaped first, or a reader shows a
//!    literal `&amp;`.

/// Elements whose contents are markup rather than prose, in every context.
pub(crate) const NON_TEXT_ELEMENTS: [&str; 3] = ["svg", "style", "script"];

/// Extract readable text, discarding `drop_elements` and their contents.
pub(crate) fn to_plain_text(html: &str, drop_elements: &[&str]) -> String {
    let mut text = html.to_string();
    for name in drop_elements {
        text = remove_element(&text, name);
    }
    let text = strip_tags(&text);
    htmlize::unescape(&text).into_owned()
}

/// Collapse all runs of whitespace to single spaces and trim.
pub(crate) fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Drop `<name ...> … </name>` regions, contents included.
///
/// A scan rather than a regex: the general tag pattern cannot match attribute
/// names containing a colon (`xmlns:xlink`), which Typst emits on every SVG.
fn remove_element(html: &str, name: &str) -> String {
    let open = format!("<{name}");
    let close = format!("</{name}>");
    let mut out = String::with_capacity(html.len());
    let mut rest = html;

    while let Some(start) = rest.find(&open) {
        // Require a tag boundary so `<svgfoo` is not treated as `<svg`.
        let after = rest[start + open.len()..].chars().next();
        if !matches!(after, Some(c) if c.is_whitespace() || c == '>' || c == '/') {
            let cut = start + open.len();
            out.push_str(&rest[..cut]);
            rest = &rest[cut..];
            continue;
        }

        out.push_str(&rest[..start]);
        // Leave a separator so neighbouring words do not fuse into one token.
        out.push(' ');
        rest = match rest[start..].find(&close) {
            Some(end) => &rest[start + end + close.len()..],
            // Unclosed: drop the remainder rather than emit markup.
            None => "",
        };
    }

    out.push_str(rest);
    out
}

/// Remove every remaining tag. A literal `<` in text is always escaped in the
/// generated HTML, so an unescaped `<` reliably starts a tag.
fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut depth = 0usize;
    for ch in html.chars() {
        match ch {
            '<' => {
                depth += 1;
                out.push(' ');
            }
            '>' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(html: &str) -> String {
        collapse_whitespace(&to_plain_text(html, &NON_TEXT_ELEMENTS))
    }

    #[test]
    fn test_strips_tags_including_names_with_digits() {
        assert_eq!(plain("<h1>Heading</h1><p>Body</p>"), "Heading Body");
    }

    #[test]
    fn test_drops_inline_svg_math_entirely() {
        let html = concat!(
            "<p>A monoid is a set ",
            r#"<svg style="width: 1.072em" viewBox="0 0 11.792 7.513" "#,
            r#"xmlns:xlink="http://www.w3.org/1999/xlink"><g><path d="M0 0"/></g></svg>"#,
            " with an operation</p>"
        );
        let text = plain(html);
        assert_eq!(text, "A monoid is a set with an operation");
    }

    #[test]
    fn test_unescapes_entities_so_text_is_not_double_escaped() {
        // Generated HTML holds `&amp;`; leaving it would be escaped again on the
        // way into XML and shown to readers as a literal "&amp;".
        assert_eq!(
            plain("<p>Salt &amp; pepper, a &lt; b, Cayley&#x27;s</p>"),
            "Salt & pepper, a < b, Cayley's"
        );
    }

    #[test]
    fn test_separates_words_across_removed_markup() {
        assert_eq!(plain("free<svg><g/></svg>monoid"), "free monoid");
    }

    #[test]
    fn test_requires_a_tag_boundary_before_dropping() {
        assert!(plain("<svgfoo>keepme</svgfoo>").contains("keepme"));
    }

    #[test]
    fn test_drops_style_and_script_content() {
        let text = plain("<style>.a{color:red}</style><script>var x=1</script><p>prose</p>");
        assert_eq!(text, "prose");
    }

    #[test]
    fn test_caller_supplied_elements_are_dropped_too() {
        // RSS drops `<header>` so a summary starts at the prose, not the taxon,
        // title, slug and date that make up the article chrome.
        let html = "<header><h1>Remark. Alice [notes/alice]</h1></header><p>The body.</p>";
        let dropped: Vec<&str> = NON_TEXT_ELEMENTS
            .iter()
            .copied()
            .chain(std::iter::once("header"))
            .collect();
        assert_eq!(
            collapse_whitespace(&to_plain_text(html, &dropped)),
            "The body."
        );
    }
}
