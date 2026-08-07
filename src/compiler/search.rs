// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.

//! Builds the client-side search index.
//!
//! The index is *inverted*: it maps each token to the sections containing it,
//! rather than storing the sections' text. That choice is deliberate. Shipping
//! the prose would mean publishing a second copy of every note — larger, and
//! readable straight out of the artifact. An inverted index cannot reconstruct
//! a note: word order, punctuation, and repetition are all discarded, and a
//! token appearing in a hundred notes is stored once.
//!
//! Titles, taxons, and slugs are shipped as text because they are needed to
//! render results anyway, and they are what the client matches against for
//! ranking. Body text only ever reaches the client as token keys.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::{
    entry::{MetaData, KEY_DATE, KEY_TAXON, KEY_TITLE},
    environment,
    slug::Slug,
};

use super::{
    section::{Section, SectionContent},
    state::CompileState,
};

/// Tokens shorter than this carry little signal and inflate the index.
const MIN_TOKEN_LEN: usize = 2;

#[derive(Debug, Serialize)]
pub(super) struct SearchIndex {
    docs: Vec<SearchDoc>,
    /// token -> indices into `docs`, sorted for stable output.
    tokens: BTreeMap<String, Vec<usize>>,
}

#[derive(Debug, Serialize)]
struct SearchDoc {
    slug: String,
    url: String,
    title: String,
    taxon: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    date: Option<String>,
}

pub(super) fn build(state: &CompileState) -> SearchIndex {
    let mut slugs: Vec<Slug> = state.compiled().keys().copied().collect();
    slugs.sort();

    let include_content = environment::search_content_enabled();
    let mut docs = Vec::with_capacity(slugs.len());
    let mut postings: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();

    for slug in slugs {
        let Some(section) = state.compiled().get(&slug) else {
            continue;
        };
        let metadata = &section.metadata;

        let title = metadata
            .page_title()
            .or_else(|| metadata.get_str(KEY_TITLE))
            .cloned()
            .unwrap_or_else(|| slug.to_string());
        let taxon = metadata.get_str(KEY_TAXON).cloned().unwrap_or_default();
        let date = metadata.get_str(KEY_DATE).cloned();

        let id = docs.len();
        // Titles, taxons and slugs are matched client-side against the shipped
        // text, so only body tokens need to enter the inverted index.
        if include_content {
            for token in tokenize(&own_text(section)) {
                postings.entry(token).or_default().insert(id);
            }
        }

        docs.push(SearchDoc {
            slug: slug.to_string(),
            url: environment::full_html_url(slug),
            title,
            taxon: strip_taxon_suffix(&taxon),
            date,
        });
    }

    SearchIndex {
        docs,
        tokens: postings
            .into_iter()
            .map(|(token, ids)| (token, ids.into_iter().collect()))
            .collect(),
    }
}

/// A section's own prose, excluding sections embedded into it: embedded content
/// belongs to the note that wrote it, and indexing it twice would make a hub
/// page match everything it displays.
fn own_text(section: &Section) -> String {
    let mut text = String::new();
    for content in &section.children {
        if let SectionContent::Plain(html) = content {
            text.push_str(&strip_html(html));
            text.push(' ');
        }
    }
    text
}

/// Elements whose *contents* are markup rather than prose. Typst renders inline
/// math and diagrams as inline `<svg>`, so without this the index fills up with
/// coordinates, colours and namespace URLs — noise that inflates the artifact
/// and makes unrelated notes match a search for "align".
const NON_TEXT_ELEMENTS: [&str; 3] = ["svg", "style", "script"];

fn strip_html(html: &str) -> String {
    let mut text = html.to_string();
    for name in NON_TEXT_ELEMENTS {
        text = remove_element(&text, name);
    }
    let text = strip_tags(&text);
    htmlize::unescape(&text).into_owned()
}

/// Drop `<name ...> … </name>` regions, contents included.
///
/// Written as a scan rather than a regex because the general tag regex used for
/// display cannot match attribute names containing a colon (`xmlns:xlink`),
/// which is exactly what Typst's SVG output emits.
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
            // Unclosed: drop the remainder rather than index markup.
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

/// Taxons are stored for display as `"Definition. "`; results show them bare.
fn strip_taxon_suffix(taxon: &str) -> String {
    taxon.trim().trim_end_matches('.').trim().to_string()
}

/// Lowercased alphanumeric runs. Deliberately simple: no stemming, so a search
/// for a word finds exactly that word's prefix, which is predictable in a way
/// that a stemmer is not.
fn tokenize(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|token| token.chars().count() >= MIN_TOKEN_LEN)
        .map(|token| token.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_lowercases_splits_and_drops_short_tokens() {
        let tokens = tokenize("A Monoid is a set, with an operation!");
        assert!(tokens.contains("monoid"));
        assert!(tokens.contains("operation"));
        assert!(tokens.contains("set"));
        // Single characters carry no signal.
        assert!(!tokens.contains("a"));
    }

    #[test]
    fn test_tokenize_deduplicates_repeated_words() {
        // The dedupe is what keeps an inverted index far smaller than the prose
        // it was built from.
        let tokens = tokenize("monoid monoid monoid");
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_tokenize_splits_on_punctuation_and_markup_boundaries() {
        let tokens = tokenize("free-monoid, semigroup; ring/field");
        for expected in ["free", "monoid", "semigroup", "ring", "field"] {
            assert!(tokens.contains(expected), "missing `{expected}`");
        }
    }

    #[test]
    fn test_strip_taxon_suffix_removes_display_punctuation() {
        assert_eq!(strip_taxon_suffix("Definition. "), "Definition");
        assert_eq!(strip_taxon_suffix("Remark"), "Remark");
        assert_eq!(strip_taxon_suffix(""), "");
    }

    #[test]
    fn test_strip_html_keeps_text_and_drops_tags() {
        let text = strip_html("<p>A <em>free</em> monoid</p>");
        assert!(text.contains("free"));
        assert!(!text.contains("<em>"));
    }

    #[test]
    fn test_strip_html_drops_inline_svg_math_entirely() {
        // Typst renders inline math as SVG whose attributes would otherwise be
        // indexed as words like "1.072em" and "http".
        let html = concat!(
            "<p>A monoid is a set ",
            r#"<svg style="width: 1.072em" viewBox="0 0 11.792 7.513" "#,
            r#"xmlns:xlink="http://www.w3.org/1999/xlink"><g><path d="M0 0"/></g></svg>"#,
            " with an operation</p>"
        );
        let tokens = tokenize(&strip_html(html));

        assert!(tokens.contains("monoid"));
        assert!(tokens.contains("operation"));
        for noise in ["svg", "072em", "11", "1999", "xlink", "http", "viewbox"] {
            assert!(!tokens.contains(noise), "indexed markup noise: `{noise}`");
        }
    }

    #[test]
    fn test_strip_html_separates_words_across_removed_markup() {
        // Without a separator the surrounding words would fuse into "freemonoid".
        let tokens = tokenize(&strip_html("free<svg><g/></svg>monoid"));
        assert!(tokens.contains("free"));
        assert!(tokens.contains("monoid"));
        assert!(!tokens.contains("freemonoid"));
    }

    #[test]
    fn test_remove_element_requires_a_tag_boundary() {
        // `<svgfoo>` is a different element and must not trigger removal.
        let kept = strip_html("<svgfoo>keepme</svgfoo>");
        assert!(kept.contains("keepme"));
    }

    #[test]
    fn test_strip_html_unescapes_entities() {
        let text = strip_html("<p>Cayley&#x27;s theorem &amp; friends</p>");
        assert!(text.contains("Cayley's"), "got: {text}");
        assert!(text.contains("&"), "got: {text}");
    }

    #[test]
    fn test_strip_html_drops_style_and_script_content() {
        let tokens = tokenize(&strip_html(
            "<style>.entry{color:red}</style><script>var x=1</script><p>prose</p>",
        ));
        assert!(tokens.contains("prose"));
        assert!(!tokens.contains("color"));
        assert!(!tokens.contains("var"));
    }
}
