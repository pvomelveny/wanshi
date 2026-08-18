// Copyright (c) 2026 wanshi contributors.
// Released under the GPL-3.0 license as described in the file LICENSE.

//! Turning Typst's headings into sections.
//!
//! Typst emits `= ` as a flat `<h2>` inside a chunk of body HTML, which makes it
//! a label rather than a container: nothing can be *inside* it. Wanshi's own
//! nesting comes from sections, so before this pass a heading and an embed that
//! followed it were siblings, and the outline in the sidebar disagreed with what
//! the article looked like.
//!
//! This pass reads the outline back out of the heading stream and rebuilds it as
//! sections, so everything after a heading belongs to it until a heading at the
//! same or a shallower level, an [`LazyContent::Outdent`], or the end of the
//! content.
//!
//! The sections it synthesises are marked internal-anonymous, which is what
//! keeps this a change to *presentation* only: `resolve_visible_parent` walks up
//! out of them, and they are stripped from the published graph, so an embed that
//! lands under a heading still reports the page as its parent and still appears
//! in listings under the page.

use std::collections::HashSet;

use crate::{
    entry::{
        HTMLMetaData, KEY_EXT, KEY_INTERNAL_ANON_SUBTREE, KEY_SLUG, KEY_SOURCE_SLUG, KEY_TITLE,
    },
    ordered_map::OrderedMap,
    slug::{self, Ext, Slug},
};

use super::anonymous_slug::{AnonymousSlugState, ANON_SUBTREE_SLUG_PREFIX};
use super::section::{
    EmbedContent, HTMLContent, HTMLContentBuilder, LazyContent, LazyContents, SectionOption,
};
use super::UnresolvedSection;

/// Deepest heading HTML defines. Typst's `======` is emitted as a
/// `<div role="heading">` rather than an element, so it never reaches this pass.
const MAX_HEADING_LEVEL: u8 = 6;

pub(super) struct HeadingSplit {
    pub content: HTMLContent,
    pub sections: Vec<(Slug, UnresolvedSection)>,
}

/// One open heading, and the content accumulated inside it so far.
struct Frame {
    level: u8,
    slug: Slug,
    title: String,
    builder: HTMLContentBuilder,
}

/// Rebuild `content` so that headings contain what follows them.
///
/// `host_slug` names the section this content belongs to and prefixes the slugs
/// of everything synthesised here. A single pass with a stack handles every
/// depth: a nested heading is just a frame opened while another is still open.
pub(super) fn split_heading_sections(
    content: HTMLContent,
    host_slug: Slug,
    source_slug: Slug,
    ext: Ext,
    used_slugs: &mut HashSet<Slug>,
    anonymous_slugs: &mut AnonymousSlugState,
) -> HeadingSplit {
    let contents: LazyContents = match content {
        HTMLContent::Plain(s) => vec![LazyContent::Plain(s)],
        HTMLContent::Lazy(contents) => contents,
    };

    let mut ctx = SplitContext {
        host_slug,
        source_slug,
        ext,
        used_slugs,
        anonymous_slugs,
        sections: Vec::new(),
    };
    let mut root = HTMLContentBuilder::new();
    let mut stack: Vec<Frame> = Vec::new();

    for item in contents {
        match item {
            LazyContent::Plain(html) => split_plain(&html, &mut ctx, &mut root, &mut stack),
            // `#outdent()` leaves nothing behind; closing the frame is the whole
            // of its effect. With nothing open it is a no-op rather than an
            // error, so moving a heading never turns a working note into a
            // failing build.
            LazyContent::Outdent => close_frame(&mut ctx, &mut root, &mut stack),
            other => top(&mut root, &mut stack).push(other),
        }
    }

    while !stack.is_empty() {
        close_frame(&mut ctx, &mut root, &mut stack);
    }

    HeadingSplit {
        content: root.build(),
        sections: ctx.sections,
    }
}

struct SplitContext<'a> {
    host_slug: Slug,
    source_slug: Slug,
    ext: Ext,
    used_slugs: &'a mut HashSet<Slug>,
    anonymous_slugs: &'a mut AnonymousSlugState,
    sections: Vec<(Slug, UnresolvedSection)>,
}

fn top<'a>(root: &'a mut HTMLContentBuilder, stack: &'a mut [Frame]) -> &'a mut HTMLContentBuilder {
    match stack.last_mut() {
        Some(frame) => &mut frame.builder,
        None => root,
    }
}

/// Pop the innermost frame, registering it as a section and leaving an embed of
/// it in its parent.
///
/// The embed lands where the heading was, because nothing is written to the
/// parent while a child frame is open.
fn close_frame(ctx: &mut SplitContext<'_>, root: &mut HTMLContentBuilder, stack: &mut Vec<Frame>) {
    let Some(frame) = stack.pop() else {
        return;
    };

    let mut metadata = OrderedMap::new();
    metadata.insert(
        KEY_SLUG.to_string(),
        HTMLContent::Plain(frame.slug.to_string()),
    );
    metadata.insert(KEY_EXT.to_string(), HTMLContent::Plain(ctx.ext.to_string()));
    metadata.insert(
        KEY_SOURCE_SLUG.to_string(),
        HTMLContent::Plain(ctx.source_slug.to_string()),
    );
    metadata.insert(
        KEY_INTERNAL_ANON_SUBTREE.to_string(),
        HTMLContent::Plain("true".to_string()),
    );
    metadata.insert(KEY_TITLE.to_string(), HTMLContent::Plain(frame.title));

    ctx.sections.push((
        frame.slug,
        UnresolvedSection {
            metadata: HTMLMetaData(metadata),
            content: frame.builder.build(),
        },
    ));

    top(root, stack).push(LazyContent::Embed(EmbedContent {
        url: format!("/{}", frame.slug),
        title: None,
        option: SectionOption::default(),
    }));
}

/// Scan one blob of body HTML, splitting it at every heading element.
fn split_plain(
    html: &str,
    ctx: &mut SplitContext<'_>,
    root: &mut HTMLContentBuilder,
    stack: &mut Vec<Frame>,
) {
    let mut cursor = 0;

    while let Some(heading) = find_heading(html, cursor) {
        top(root, stack).push_str(&html[cursor..heading.start]);
        cursor = heading.end;

        // A heading closes every open heading at least as deep as itself: that
        // is what makes two `= `s siblings rather than one nesting in the other.
        while stack.last().is_some_and(|f| f.level >= heading.level) {
            close_frame(ctx, root, stack);
        }

        let slug = allocate_slug(ctx, &heading.title);
        stack.push(Frame {
            level: heading.level,
            slug,
            title: heading.title,
            builder: HTMLContentBuilder::new(),
        });
    }

    top(root, stack).push_str(&html[cursor..]);
}

struct Heading {
    level: u8,
    /// Byte range of the whole element, opening tag through closing tag.
    start: usize,
    end: usize,
    /// Inner HTML, which becomes the section's title and so may carry markup.
    title: String,
}

/// Find the next heading element at or after `from`.
///
/// Requires a digit straight after `<h`, which is why Typst's own `<head>` —
/// left in the body by `typst_cli::html_to_body_content` — can never match. An
/// unclosed heading is skipped rather than treated as running to end of input,
/// so malformed HTML cannot swallow the rest of a note.
fn find_heading(html: &str, from: usize) -> Option<Heading> {
    let bytes = html.as_bytes();
    let mut cursor = from;

    while let Some(found) = html[cursor..].find("<h") {
        let start = cursor + found;
        let digit = bytes.get(start + 2).copied();
        let level = match digit {
            Some(d) if d.is_ascii_digit() && (b'1'..=b'6').contains(&d) => d - b'0',
            _ => {
                cursor = start + 2;
                continue;
            }
        };

        // An opening tag that never closes means there is no further heading to
        // find, so this ends the scan rather than skipping ahead.
        let open_end = html[start..].find('>').map(|i| start + i + 1)?;
        let close = format!("</h{}>", level);
        let Some(close_start) = html[open_end..].find(&close).map(|i| open_end + i) else {
            cursor = open_end;
            continue;
        };

        return Some(Heading {
            level: level.min(MAX_HEADING_LEVEL),
            start,
            end: close_start + close.len(),
            title: html[open_end..close_start].to_string(),
        });
    }

    None
}

/// Name a heading's section after its text, falling back to a number.
///
/// Ordinals alone would do, but a heading's slug is also its `[#]` permalink,
/// and an ordinal renumbers every anchor below any heading you insert — quietly
/// breaking links people have already shared. Text is stable under that edit.
fn allocate_slug(ctx: &mut SplitContext<'_>, title: &str) -> Slug {
    let text = slugify(title);
    if !text.is_empty() {
        let candidate = slug::to_slug(format!(
            "{}/{}{}",
            ctx.host_slug, ANON_SUBTREE_SLUG_PREFIX, text
        ));
        if ctx.used_slugs.insert(candidate) {
            return candidate;
        }
    }
    // Empty after stripping markup, or two headings with the same words.
    ctx.anonymous_slugs
        .allocate_with_used(ctx.host_slug, ctx.used_slugs)
}

/// Reduce heading text to a slug component: tags dropped, runs of anything that
/// is not alphanumeric collapsed to a single `-`.
fn slugify(title: &str) -> String {
    let mut text = String::with_capacity(title.len());
    let mut in_tag = false;
    for ch in title.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if in_tag => {}
            _ => text.push(ch),
        }
    }

    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            out.extend(ch.to_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(contents: LazyContents) -> HeadingSplit {
        let mut used = HashSet::from([Slug::new("page")]);
        let mut anon = AnonymousSlugState::default();
        split_heading_sections(
            HTMLContent::Lazy(contents),
            Slug::new("page"),
            Slug::new("page"),
            Ext::Typ,
            &mut used,
            &mut anon,
        )
    }

    fn plain(s: &str) -> LazyContent {
        LazyContent::Plain(s.to_string())
    }

    fn embed(url: &str) -> LazyContent {
        LazyContent::Embed(EmbedContent {
            url: url.to_string(),
            title: None,
            option: SectionOption::default(),
        })
    }

    /// The embeds a section holds, by url, in order.
    fn embeds(content: &HTMLContent) -> Vec<String> {
        match content {
            HTMLContent::Plain(_) => vec![],
            HTMLContent::Lazy(items) => items
                .iter()
                .filter_map(|item| match item {
                    LazyContent::Embed(e) => Some(e.url.clone()),
                    _ => None,
                })
                .collect(),
        }
    }

    fn section<'a>(split: &'a HeadingSplit, slug: &str) -> &'a UnresolvedSection {
        &split
            .sections
            .iter()
            .find(|(s, _)| s.as_str() == slug)
            .unwrap_or_else(|| panic!("no section `{slug}` in {:?}", slugs(split)))
            .1
    }

    fn slugs(split: &HeadingSplit) -> Vec<String> {
        split.sections.iter().map(|(s, _)| s.to_string()).collect()
    }

    #[test]
    fn test_embed_after_a_heading_becomes_its_child() {
        let split = split(vec![plain("<h2>Options</h2><p>x</p>"), embed("/note")]);
        assert_eq!(embeds(&split.content), vec!["/page/:options"]);
        assert_eq!(embeds(&section(&split, "page/:options").content), ["/note"]);
    }

    #[test]
    fn test_content_before_the_first_heading_stays_on_the_page() {
        let split = split(vec![embed("/intro"), plain("<h2>Later</h2>")]);
        assert_eq!(embeds(&split.content), vec!["/intro", "/page/:later"]);
        assert!(embeds(&section(&split, "page/:later").content).is_empty());
    }

    #[test]
    fn test_headings_at_one_level_are_siblings() {
        let split = split(vec![
            plain("<h2>One</h2>"),
            embed("/a"),
            plain("<h2>Two</h2>"),
            embed("/b"),
        ]);
        assert_eq!(embeds(&split.content), vec!["/page/:one", "/page/:two"]);
        assert_eq!(embeds(&section(&split, "page/:one").content), ["/a"]);
        assert_eq!(embeds(&section(&split, "page/:two").content), ["/b"]);
    }

    #[test]
    fn test_a_deeper_heading_nests_in_the_one_above_it() {
        let split = split(vec![plain("<h2>Outer</h2><h3>Inner</h3>"), embed("/a")]);
        assert_eq!(embeds(&split.content), vec!["/page/:outer"]);
        assert_eq!(
            embeds(&section(&split, "page/:outer").content),
            ["/page/:inner"]
        );
        assert_eq!(embeds(&section(&split, "page/:inner").content), ["/a"]);
    }

    /// A skipped level nests one deep rather than inventing the level between,
    /// so the rendered heading follows the tree and never skips a number.
    #[test]
    fn test_a_skipped_level_nests_exactly_one_deep() {
        let split = split(vec![plain("<h2>Outer</h2><h4>Jumped</h4>"), embed("/a")]);
        assert_eq!(
            embeds(&section(&split, "page/:outer").content),
            ["/page/:jumped"]
        );
        assert_eq!(embeds(&section(&split, "page/:jumped").content), ["/a"]);
    }

    #[test]
    fn test_outdent_closes_one_level() {
        let split = split(vec![
            plain("<h2>Outer</h2><h3>Inner</h3>"),
            embed("/a"),
            LazyContent::Outdent,
            embed("/b"),
        ]);
        assert_eq!(embeds(&section(&split, "page/:inner").content), ["/a"]);
        assert_eq!(
            embeds(&section(&split, "page/:outer").content),
            ["/page/:inner", "/b"]
        );
    }

    #[test]
    fn test_two_outdents_reach_the_page() {
        let split = split(vec![
            plain("<h2>Outer</h2><h3>Inner</h3>"),
            LazyContent::Outdent,
            LazyContent::Outdent,
            embed("/b"),
        ]);
        assert_eq!(embeds(&split.content), vec!["/page/:outer", "/b"]);
    }

    #[test]
    fn test_outdent_with_nothing_open_is_a_no_op() {
        let split = split(vec![LazyContent::Outdent, embed("/a")]);
        assert!(split.sections.is_empty());
        assert_eq!(embeds(&split.content), vec!["/a"]);
    }

    #[test]
    fn test_typst_head_is_not_a_heading() {
        let split = split(vec![plain("<head><title>t</title></head><p>body</p>")]);
        assert!(split.sections.is_empty(), "got {:?}", slugs(&split));
    }

    /// Typst emits `======` as a `<div role="heading">`, which is not an element
    /// this pass recognises and so is left in the prose.
    #[test]
    fn test_div_role_heading_is_not_split() {
        let split = split(vec![plain(r#"<div role="heading">Six</div>"#)]);
        assert!(split.sections.is_empty());
    }

    #[test]
    fn test_an_unclosed_heading_is_skipped_rather_than_swallowing_the_rest() {
        let split = split(vec![plain("<h2>Broken<p>rest</p>")]);
        assert!(split.sections.is_empty());
        assert_eq!(embeds(&split.content), Vec::<String>::new());
    }

    #[test]
    fn test_titles_keep_their_markup() {
        let split = split(vec![plain("<h2>The <em>hard</em> part</h2>")]);
        let title = section(&split, "page/:the-hard-part")
            .metadata
            .0
            .get(KEY_TITLE)
            .and_then(|c| c.as_string())
            .cloned()
            .unwrap();
        assert_eq!(title, "The <em>hard</em> part");
    }

    #[test]
    fn test_repeated_heading_text_falls_back_to_an_ordinal() {
        let split = split(vec![plain("<h2>Same</h2><h2>Same</h2>")]);
        let names = slugs(&split);
        assert!(names.contains(&"page/:same".to_string()), "{names:?}");
        assert_eq!(names.len(), 2);
        assert_eq!(
            names.iter().collect::<HashSet<_>>().len(),
            2,
            "slugs must stay unique: {names:?}"
        );
    }

    #[test]
    fn test_a_heading_of_only_markup_falls_back_to_an_ordinal() {
        let split = split(vec![plain("<h2><em></em></h2>")]);
        assert_eq!(slugs(&split).len(), 1);
        assert!(slugs(&split)[0].starts_with("page/:"));
    }

    #[test]
    fn test_synthesised_sections_are_marked_internal_anonymous() {
        let split = split(vec![plain("<h2>Options</h2>")]);
        let marked = section(&split, "page/:options")
            .metadata
            .0
            .get(KEY_INTERNAL_ANON_SUBTREE)
            .and_then(|c| c.as_string())
            .cloned();
        assert_eq!(marked.as_deref(), Some("true"));
    }
}
