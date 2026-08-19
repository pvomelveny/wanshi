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
    entry::{MetaData, KEY_DATE, KEY_INTERNAL_ANON_SUBTREE, KEY_TAXON, KEY_TITLE},
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
            taxon,
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

/// A section's own prose, excluding notes embedded into it: embedded content
/// belongs to the note that wrote it, and indexing it twice would make a hub
/// page match everything it displays.
///
/// Internal-anonymous sections are the exception, and must be descended into. A
/// heading is one of these, so nearly all of a page's prose now lives inside
/// them — and they have no page of their own to be found by, since they are
/// stripped before the index is built. Their text is the page's own text, and
/// skipping them left every note searchable only down to its first heading.
fn own_text(section: &Section) -> String {
    let mut text = String::new();
    collect_own_text(section, &mut text);
    text
}

fn collect_own_text(section: &Section, text: &mut String) {
    for content in &section.children {
        match content {
            SectionContent::Plain(html) => {
                text.push_str(&strip_html(html));
                text.push(' ');
            }
            SectionContent::Embed(child) if is_internal_anonymous(child) => {
                // The heading's own words, which were body prose before it
                // became a section and are still prose to a reader. Nothing
                // else carries them: the section is stripped, so it never
                // becomes a result with a title of its own.
                if let Some(title) = child.metadata.get_str(KEY_TITLE) {
                    text.push_str(&strip_html(title));
                    text.push(' ');
                }
                collect_own_text(child, text);
            }
            SectionContent::Embed(_) | SectionContent::Query(_) => {}
        }
    }
}

fn is_internal_anonymous(section: &Section) -> bool {
    section
        .metadata
        .get_str(KEY_INTERNAL_ANON_SUBTREE)
        .is_some_and(|value| value == "true")
}

fn strip_html(html: &str) -> String {
    crate::html_text::to_plain_text(html, &crate::html_text::NON_TEXT_ELEMENTS)
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
    use crate::{
        entry::{EntryMetaData, KEY_SLUG},
        ordered_map::OrderedMap,
    };
    use std::collections::HashSet;

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

    fn section(slug: &str, internal_anonymous: bool, children: Vec<SectionContent>) -> Section {
        let mut metadata = OrderedMap::new();
        metadata.insert(KEY_SLUG.to_string(), slug.to_string());
        metadata.insert(KEY_TITLE.to_string(), format!("Titled {slug}"));
        if internal_anonymous {
            metadata.insert(KEY_INTERNAL_ANON_SUBTREE.to_string(), "true".to_string());
        }
        Section::new(EntryMetaData(metadata), children, HashSet::new())
    }

    fn plain(html: &str) -> SectionContent {
        SectionContent::Plain(html.to_string())
    }

    /// A heading's section is internal-anonymous and is stripped before the
    /// index is built, so its prose has no page of its own to be found by. It
    /// belongs to the page, and skipping it made every note searchable only
    /// down to its first heading.
    #[test]
    fn test_own_text_descends_into_headings() {
        let page = section(
            "page",
            false,
            vec![
                plain("<p>intro</p>"),
                SectionContent::Embed(section(
                    "page/:background",
                    true,
                    vec![
                        plain("<p>underheading</p>"),
                        SectionContent::Embed(section(
                            "page/:background/:deeper",
                            true,
                            vec![plain("<p>nested</p>")],
                        )),
                    ],
                )),
            ],
        );

        let tokens = tokenize(&own_text(&page));
        for expected in ["intro", "underheading", "nested"] {
            assert!(tokens.contains(expected), "missing `{expected}`");
        }
    }

    /// A heading's words were body prose before it became a section, and are
    /// still prose to a reader. Nothing else carries them — the section is
    /// stripped, so it never becomes a result with a title of its own.
    #[test]
    fn test_own_text_indexes_the_heading_text_itself() {
        let page = section(
            "page",
            false,
            vec![SectionContent::Embed(section(
                "page/:background",
                true,
                vec![plain("<p>prose</p>")],
            ))],
        );

        let tokens = tokenize(&own_text(&page));
        assert!(tokens.contains("background"), "heading text is not indexed");
    }

    /// A real embedded note's title belongs to that note's own result, and the
    /// page must not match on it.
    #[test]
    fn test_own_text_does_not_take_an_embedded_notes_title() {
        let page = section(
            "page",
            false,
            vec![SectionContent::Embed(section("other", false, vec![]))],
        );

        assert!(!tokenize(&own_text(&page)).contains("other"));
    }

    /// The rule the descent must not undo: a note embedded in a page keeps its
    /// own index entry, so counting its words for the page too would make a hub
    /// match everything it displays.
    #[test]
    fn test_own_text_stops_at_a_real_embedded_note() {
        let page = section(
            "page",
            false,
            vec![
                plain("<p>intro</p>"),
                SectionContent::Embed(section("other", false, vec![plain("<p>elsewhere</p>")])),
            ],
        );

        let tokens = tokenize(&own_text(&page));
        assert!(tokens.contains("intro"));
        assert!(!tokens.contains("elsewhere"));
    }

    /// An embedded note written under a heading is still a note. Descending
    /// through the heading must not carry on into it.
    #[test]
    fn test_own_text_stops_at_a_note_embedded_under_a_heading() {
        let page = section(
            "page",
            false,
            vec![SectionContent::Embed(section(
                "page/:background",
                true,
                vec![
                    plain("<p>underheading</p>"),
                    SectionContent::Embed(section("other", false, vec![plain("<p>elsewhere</p>")])),
                ],
            ))],
        );

        let tokens = tokenize(&own_text(&page));
        assert!(tokens.contains("underheading"));
        assert!(!tokens.contains("elsewhere"));
    }
}
