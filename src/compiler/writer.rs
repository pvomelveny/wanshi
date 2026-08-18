// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use eyre::eyre;
use std::{collections::HashSet, ops::Not};

use crate::{
    compiler::counter::{Counter, NumberKind},
    config::build::FooterMode,
    entry::{MetaData, KEY_INTERNAL_ANON_SUBTREE},
    environment::{self, verify_update_hash},
    html_flake::{self, html_footer_section},
    slug::Slug,
};

use super::{
    callback::CallbackValue,
    section::{Section, SectionContent},
    state::CompileState,
    taxon::{display_taxon, Taxon},
};

/// The page's own section. Its title is the one `h1` on the page.
const PAGE_LEVEL: u8 = 1;

/// Footer blocks ("References", "Backlinks") are `h2`, so the entries listed
/// inside them are `h3`.
const FOOTER_ENTRY_LEVEL: u8 = 3;

/// Deepest heading HTML defines.
const MAX_HEADING_LEVEL: u8 = 6;

/// Push the Typst headings in a chunk of body HTML down by its section's depth.
///
/// Typst maps its own `= ` to `<h2>`, reserving `h1` for the note's title — an
/// assumption that holds for a note rendered as its own page and breaks the
/// moment it is embedded, where the note's title is no longer `h1`. Shifting by
/// `depth - 1` restores it: at depth 1 nothing moves, and deeper the headings
/// follow their section down. Levels clamp at `h6` rather than inventing `h7`.
///
/// Scans rather than matching a regex: `<h` must be followed by an ASCII digit,
/// so Typst's own `<head>` — which `typst_cli::html_to_body_content` leaves in
/// the content — can never match. `<h1 class="query-title">` from a resolved
/// listing is a real heading and shifts with the rest.
fn shift_heading_levels(html: &str, depth: u8) -> String {
    let shift = depth.saturating_sub(1);
    if shift == 0 || html.is_empty() {
        return html.to_string();
    }

    let bytes = html.as_bytes();
    let mut out = String::with_capacity(html.len());
    let mut cursor = 0;

    while let Some(found) = html[cursor..].find('<') {
        let open = cursor + found;
        // `<h` or `</h`, then a digit 1-6.
        let after = if bytes.get(open + 1) == Some(&b'/') {
            open + 2
        } else {
            open + 1
        };
        let is_heading = bytes.get(after) == Some(&b'h')
            && bytes
                .get(after + 1)
                .is_some_and(|d| d.is_ascii_digit() && (b'1'..=b'6').contains(d));

        if !is_heading {
            out.push_str(&html[cursor..open + 1]);
            cursor = open + 1;
            continue;
        }

        let level = (bytes[after + 1] - b'0')
            .saturating_add(shift)
            .min(MAX_HEADING_LEVEL);
        out.push_str(&html[cursor..after + 1]);
        out.push((b'0' + level) as char);
        cursor = after + 2;
    }

    out.push_str(&html[cursor..]);
    out
}

/// What a section's title carries, and where numbering has got to inside it.
///
/// `taxon` and `number` are never both set: a section with a taxon shows its
/// number in the pill, one without shows a bare number in front of the title.
struct Label {
    taxon: String,
    number: String,
    children: Counter,
}

pub struct Writer {}

impl Writer {
    pub fn write(section: &Section, state: &CompileState) -> eyre::Result<()> {
        let (html, page_title) = Writer::html_doc(section, state)?;
        let relative_path = format!("{}.html", section.slug()?);
        let filepath = crate::environment::output_path(&relative_path);

        match verify_update_hash(&relative_path, &html) {
            // The hash records what was written, not what is still there, so a
            // matching hash alone does not mean the page exists: deleting the
            // output directory and rebuilding would otherwise produce a site
            // with no pages at all, and report success.
            Ok(changed) if changed || !filepath.exists() => match std::fs::write(&filepath, html) {
                Ok(()) => {
                    if *crate::cli::build::verbose() {
                        color_print::ceprintln!("<g>[build]</> {:?} {}", page_title, filepath);
                    }
                }
                Err(err) => color_print::ceprintln!("<r>{:?}</>", err),
            },
            Ok(_) => {
                if *crate::cli::build::verbose_skip() {
                    color_print::ceprintln!("<dim>[skip]</> {} (unchanged)", relative_path);
                }
            }
            Err(err) => {
                color_print::ceprintln!(
                    "<y>Warning: failed to verify hash for `{}`: {}</>",
                    relative_path,
                    err
                );
            }
        }

        Ok(())
    }

    pub fn write_needed_slugs<I>(all_slugs: I, state: &CompileState) -> eyre::Result<()>
    where
        I: IntoIterator<Item = Slug>,
    {
        for slug in all_slugs {
            let section = state
                .compiled()
                .get(&slug)
                .ok_or_else(|| eyre!("slug `{}` not in compiled entries", slug))?;
            Writer::write(section, state)?;
        }
        Ok(())
    }

    /// Whether this page numbers its sections.
    ///
    /// Read from the page being rendered, never from an embedded note: a number
    /// is a position in one page's sequence, and the same note holds different
    /// positions on different pages. A note's own `numbering` therefore governs
    /// its own page and is ignored wherever it is embedded.
    fn page_numbering(section: &Section) -> eyre::Result<bool> {
        Ok(section
            .metadata
            .numbering()?
            .unwrap_or_else(environment::numbering))
    }

    pub fn rss_content_html(section: &Section, state: &CompileState) -> eyre::Result<String> {
        let mut counter = Counter::init();
        let numbering = Writer::page_numbering(section)?;
        let (article_inner, _catalog_item) = Writer::section_to_html(
            section,
            &mut counter,
            true,
            false,
            state,
            PAGE_LEVEL,
            numbering,
        )?;
        Ok(article_inner)
    }

    pub fn html_doc(section: &Section, state: &CompileState) -> eyre::Result<(String, String)> {
        let mut counter = Counter::init();
        let numbering = Writer::page_numbering(section)?;

        let (article_inner, items) = Writer::section_to_html(
            section,
            &mut counter,
            true,
            false,
            state,
            PAGE_LEVEL,
            numbering,
        )?;
        let catalog_html = if items.is_empty().not() {
            html_flake::html_catalog_block(&items)
        } else {
            Default::default()
        };

        let slug = section.slug()?;
        let html_header = Writer::header(state, slug);

        let callback = state.callback().0.get(&slug);
        let footer_sort_by = section
            .metadata
            .footer_sort_by()
            .unwrap_or_else(environment::footer_sort_by);
        let footer_html = Writer::footer(
            section.metadata.footer_mode()?,
            section.metadata.references_enabled()?,
            &footer_sort_by,
            state,
            &section.references,
            callback,
        )?;
        let page_title = section.metadata.page_title().map_or("", |s| s.as_str());

        let html = crate::html_flake::html_doc(
            page_title,
            &html_header,
            &article_inner,
            &footer_html,
            &catalog_html,
        );

        Ok((html, page_title.to_string()))
    }

    fn header(state: &CompileState, slug: Slug) -> String {
        // We must avoid the root section defaulting to itself as its parent.
        if slug.as_str() == crate::compiler::INDEX_SLUG {
            return String::default();
        }

        let parent = state.parent_of(slug);
        let Some(section) = state.compiled().get(&parent) else {
            color_print::ceprintln!(
                "<y>Warning: missing parent section `{}` for `{}`; header nav is skipped.</>",
                parent,
                slug
            );
            return String::default();
        };

        let href = environment::full_html_url(parent);
        let title = section.metadata.title().map_or("", |s| s);
        let page_title = section.metadata.page_title().map_or("", |s| s);
        html_flake::html_header_nav(title, page_title, &href)
    }

    fn footer(
        footer_mode: Option<FooterMode>,
        enable_references: bool,
        footer_sort_by: &str,
        state: &CompileState,
        references: &HashSet<Slug>,
        callback: Option<&CallbackValue>,
    ) -> eyre::Result<String> {
        let mut references: Vec<Slug> = references.iter().copied().collect();
        Writer::sort_footer_slugs(&mut references, state, footer_sort_by);

        let references_text = environment::get_footer_references_text();
        let references_html = if enable_references {
            let mut content = String::new();
            for slug in &references {
                let Some(section) = state.compiled().get(slug) else {
                    color_print::ceprintln!(
                        "<y>Warning: missing referenced section `{}`; skipping footer reference.</>",
                        slug
                    );
                    continue;
                };
                content.push_str(&Writer::footer_section_to_html(
                    footer_mode,
                    section,
                    FOOTER_ENTRY_LEVEL,
                )?);
            }

            if content.is_empty() {
                String::default()
            } else {
                html_footer_section("references", &references_text, &content)
            }
        } else {
            String::default()
        };

        let backlinks_text = environment::get_footer_backlinks_text();
        let backlinks_html = if let Some(s) = callback {
            let mut backlinks: Vec<Slug> = s.backlinks.iter().copied().collect();
            Writer::sort_footer_slugs(&mut backlinks, state, footer_sort_by);
            let mut content = String::new();
            for slug in backlinks {
                let Some(section) = state.compiled().get(&slug) else {
                    color_print::ceprintln!(
                        "<y>Warning: missing backlink section `{}`; skipping footer backlink.</>",
                        slug
                    );
                    continue;
                };
                content.push_str(&Writer::footer_section_to_html(
                    footer_mode,
                    section,
                    FOOTER_ENTRY_LEVEL,
                )?);
            }

            if content.is_empty() {
                String::default()
            } else {
                html_footer_section("backlinks", &backlinks_text, &content)
            }
        } else {
            String::default()
        };
        // "Found in": the notes that embed this one.
        //
        // Always rendered as links, whatever `footer-mode` says. An embedder
        // contains the note whose page this is, so rendering one in embed mode
        // would print the page inside its own footer. Forcing Link mode means
        // there is nothing to recurse into rather than a recursion to guard.
        let embedded_by_text = environment::get_footer_embedded_by_text();
        let embedded_by_html = if let Some(s) = callback {
            let mut hosts: Vec<Slug> = s.embedded_by.iter().copied().collect();
            Writer::sort_footer_slugs(&mut hosts, state, footer_sort_by);
            let mut content = String::new();
            for slug in hosts {
                let Some(section) = state.compiled().get(&slug) else {
                    color_print::ceprintln!(
                        "<y>Warning: missing embedding section `{}`; skipping footer entry.</>",
                        slug
                    );
                    continue;
                };
                content.push_str(&Writer::footer_section_to_html(
                    Some(FooterMode::Link),
                    section,
                    FOOTER_ENTRY_LEVEL,
                )?);
            }

            if content.is_empty() {
                String::default()
            } else {
                html_footer_section("embedded-by", &embedded_by_text, &content)
            }
        } else {
            String::default()
        };

        Ok(html_flake::html_footer(
            &references_html,
            &backlinks_html,
            &embedded_by_html,
        ))
    }

    fn sort_footer_slugs(slugs: &mut [Slug], state: &CompileState, footer_sort_by: &str) {
        let sort_key = footer_sort_by.trim();
        slugs.sort_by(|left, right| {
            let left_section = state.compiled().get(left);
            let right_section = state.compiled().get(right);
            let left_value = left_section.map_or("", |section| {
                Writer::footer_sort_value(sort_key, section, left)
            });
            let right_value = right_section.map_or("", |section| {
                Writer::footer_sort_value(sort_key, section, right)
            });

            crate::footer_sort::compare_values(sort_key, left_value, right_value)
                .then_with(|| left.cmp(right))
        });
    }

    fn footer_sort_value<'a>(
        footer_sort_by: &str,
        section: &'a Section,
        slug: &'a Slug,
    ) -> &'a str {
        match footer_sort_by {
            "slug" => slug.as_str(),
            "date" => section.metadata.get_str("date").map_or("", String::as_str),
            "taxon" => section.metadata.data_taxon().map_or("", String::as_str),
            "title" => section.metadata.title().map_or("", String::as_str),
            key => section.metadata.get_str(key).map_or("", String::as_str),
        }
    }

    /// `depth` is the section's heading depth, not its catalog depth. A page
    /// renders at `PAGE_LEVEL` and emits no row of its own, so its children —
    /// the outermost catalog rows — arrive here one deeper than that.
    fn catalog_item(
        section: &Section,
        taxon: &str,
        number: &str,
        child_html: &str,
        depth: u8,
    ) -> eyre::Result<String> {
        let slug = section.slug()?;
        let title = section.metadata.title().map_or("", |s| s);
        let page_title = section.metadata.page_title().map_or("", |s| s);
        let use_hash_href = Writer::is_internal_anonymous_subtree(section)?;
        Ok(html_flake::catalog_item(html_flake::CatalogItemArgs {
            slug,
            title,
            page_title,
            details_open: section.option.details_open,
            taxon,
            number,
            child_html,
            use_hash_href,
            level: depth.saturating_sub(PAGE_LEVEL),
        }))
    }

    fn footer_content_to_html(
        page_option: Option<FooterMode>,
        content: &SectionContent,
        level: u8,
    ) -> eyre::Result<String> {
        match content {
            SectionContent::Plain(s) => Ok(shift_heading_levels(s, level)),
            SectionContent::Embed(section) => {
                Writer::footer_section_to_html(page_option, section, level + 1)
            }
            SectionContent::Query(_) => Ok(String::new()),
        }
    }

    fn footer_section_to_html(
        page_option: Option<FooterMode>,
        section: &Section,
        level: u8,
    ) -> eyre::Result<String> {
        let footer_mode = page_option.unwrap_or(environment::footer_mode());

        match footer_mode {
            FooterMode::Link => {
                // No number: a footer entry points at another note, and a
                // number here would claim a place in this page's sequence that
                // the entry does not hold.
                let summary = section.metadata.to_header(None, None, "", false, level)?;
                let data_taxon = section.metadata.data_taxon().map_or("", |s| s);
                Ok(format!(
                    r#"<section class="block" data-taxon="{data_taxon}" style="margin-bottom: 0.4em;">{summary}</section>"#
                ))
            }
            FooterMode::Embed => {
                let mut contents = String::new();
                for content in &section.children {
                    contents.push_str(&Writer::footer_content_to_html(
                        page_option,
                        content,
                        level,
                    )?);
                }
                // A footer entry is a pointer to another note. Its author,
                // status and the rest belong on that note's own page, not
                // repeated unlabelled under every page that links to it.
                html_flake::html_article_inner(html_flake::ArticleInnerArgs {
                    metadata: &section.metadata,
                    contents: &contents,
                    hide_metadata: true,
                    open: false,
                    adhoc_title: None,
                    adhoc_taxon: None,
                    number: "",
                    level,
                })
            }
        }
    }

    pub fn section_to_html(
        section: &Section,
        counter: &mut Counter,
        toplevel: bool,
        hide_metadata: bool,
        state: &CompileState,
        depth: u8,
        inherited_numbering: bool,
    ) -> eyre::Result<(String, String)> {
        // A block overrides the page only by saying so. The resolved value is
        // what children inherit, not the page's, so `numbering: false` on
        // something that contains other blocks covers all of them — and an
        // explicit `true` further in turns numbering back on.
        let numbering = section.option.numbering.unwrap_or(inherited_numbering);
        // The page's own title takes no number even on a numbered page. It is
        // the only thing at its level, so a number distinguishes it from
        // nothing — and taking one would push every number on the page a level
        // deeper, turning `Definition 1.` into `Definition 1.1.`
        let numbered_here = numbering && !toplevel;
        let Label {
            taxon: adhoc_taxon,
            number: adhoc_number,
            children: mut subcounter,
        } = Writer::label(section, counter, numbered_here);
        let (mut contents, mut items) = (String::new(), String::new());

        if !section.children.is_empty() {
            let is_collection = section.metadata.is_collect()?;

            for child in &section.children {
                let (content_html, item_html) = Writer::content_to_html(
                    child,
                    &mut subcounter,
                    !is_collection,
                    state,
                    depth,
                    numbering,
                )?;
                contents.push_str(&content_html);
                items.push_str(&item_html);
            }
        };

        if !toplevel && section.metadata.is_backlinks_transparent()? {
            let slug = section.slug()?;
            let footer_sort_by = section
                .metadata
                .footer_sort_by()
                .unwrap_or_else(environment::footer_sort_by);
            let backlinks_html = Writer::footer(
                section.metadata.footer_mode()?,
                false,
                &footer_sort_by,
                state,
                &section.references,
                state.callback().0.get(&slug),
            )?;
            contents += &backlinks_html;
        }

        let child_html = if !items.is_empty() {
            format!(r#"<ul class="block">{}</ul>"#, &items)
        } else {
            String::default()
        };

        let catalog_item = if toplevel {
            child_html
        } else {
            section
                .option
                .catalog
                .then(|| {
                    Writer::catalog_item(section, &adhoc_taxon, &adhoc_number, &child_html, depth)
                })
                .transpose()?
                .unwrap_or(String::new())
        };

        let article_inner = html_flake::html_article_inner(html_flake::ArticleInnerArgs {
            metadata: &section.metadata,
            contents: &contents,
            hide_metadata,
            open: section.option.details_open,
            adhoc_title: None,
            adhoc_taxon: Some(adhoc_taxon.as_str()),
            number: &adhoc_number,
            level: depth,
        })?;

        Ok((article_inner, catalog_item))
    }

    fn content_to_html(
        content: &SectionContent,
        counter: &mut Counter,
        hide_metadata: bool,
        state: &CompileState,
        depth: u8,
        inherited_numbering: bool,
    ) -> eyre::Result<(String, String)> {
        match content {
            // Typst numbers its own headings from a note's own top level, so a
            // note embedded three deep would emit the same levels as the page
            // around it. Shifting by the containing depth slots them in below
            // their section instead of alongside it.
            SectionContent::Plain(s) => Ok((shift_heading_levels(s, depth), String::new())),
            SectionContent::Embed(section) => Writer::section_to_html(
                section,
                counter,
                false,
                hide_metadata,
                state,
                depth + 1,
                inherited_numbering,
            ),
            // Listings are substituted for plain HTML before writing begins;
            // reaching one here means the resolve pass was skipped.
            SectionContent::Query(spec) => {
                color_print::ceprintln!(
                    "<y>Warning: unresolved listing in `{}`; rendering nothing.</>",
                    spec.owner
                );
                Ok((String::new(), String::new()))
            }
        }
    }

    /// Render-time formatting of a section's taxon.
    ///
    /// The metadata holds the taxon exactly as authored; capitalisation and the
    /// trailing ". " separator are applied here, at the point of display.
    /// The two labels a section title can carry: its taxon, and its number.
    ///
    /// Only one of them ever holds the number. A taxon'd block reads
    /// `Definition 1.1.`, with the digits inside the pill, the way a statement
    /// is written on paper; a section with no taxon — a Typst heading, above all
    /// — puts a bare number in front of its title, the way a section is. Both
    /// spellings are conventional, and which one applies is decided by whether
    /// there is a word for the number to attach to.
    ///
    /// The counter steps for either, so an unnumbered taxon and a numbered
    /// heading share one sequence rather than each keeping their own.
    fn label(section: &Section, counter: &mut Counter, numbering: bool) -> Label {
        let text = section.metadata.taxon().map_or("", |s| s);
        if !numbering {
            return Label {
                taxon: display_taxon(text),
                number: String::new(),
                children: counter.passthrough(),
            };
        }

        // The taxon decides both which sequence the section counts in and where
        // its number is rendered, so the two can never disagree: a section with
        // no taxon is part of the outline and shows a leading number, one with a
        // taxon is a statement and shows its number inside the pill.
        let kind = if text.is_empty() {
            NumberKind::Outline
        } else {
            NumberKind::Statement
        };
        let (number, children) = counter.take(kind);

        if text.is_empty() {
            Label {
                taxon: String::new(),
                number,
                children,
            }
        } else {
            Label {
                taxon: Taxon::new(Some(number), text.to_string()).display(),
                number: String::new(),
                children,
            }
        }
    }

    fn is_internal_anonymous_subtree(section: &Section) -> eyre::Result<bool> {
        Ok(section
            .metadata
            .get_bool(KEY_INTERNAL_ANON_SUBTREE)?
            .unwrap_or(false))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        compiler::{
            section::{
                EmbedContent, HTMLContent, LazyContent, LocalLink, SectionOption, UnresolvedSection,
            },
            state::compile_all,
        },
        entry::{
            HTMLMetaData, KEY_EXT, KEY_INTERNAL_ANON_SUBTREE, KEY_NUMBERING, KEY_PAGE_TITLE,
            KEY_REFERENCES, KEY_SLUG, KEY_TAXON, KEY_TITLE,
        },
        ordered_map::OrderedMap,
    };

    use super::*;

    fn with_test_env(f: impl FnOnce()) {
        let root = crate::test_io::case_dir("writer");
        std::fs::create_dir_all(root.as_std_path()).unwrap();
        crate::environment::with_test_environment(
            root.clone(),
            crate::environment::BuildMode::Publish,
            f,
        );
        let _ = std::fs::remove_dir_all(root.as_std_path());
    }

    fn shallow_section(slug: &str, title: &str) -> UnresolvedSection {
        shallow_section_with_content(slug, title, HTMLContent::Plain("<p>hello</p>".to_string()))
    }

    fn shallow_section_with_content(
        slug: &str,
        title: &str,
        content: HTMLContent,
    ) -> UnresolvedSection {
        let mut metadata = OrderedMap::new();
        metadata.insert(KEY_SLUG.to_string(), HTMLContent::Plain(slug.to_string()));
        metadata.insert(KEY_EXT.to_string(), HTMLContent::Plain("typst".to_string()));
        metadata.insert(KEY_TITLE.to_string(), HTMLContent::Plain(title.to_string()));
        metadata.insert(
            KEY_PAGE_TITLE.to_string(),
            HTMLContent::Plain(title.to_string()),
        );

        UnresolvedSection {
            metadata: HTMLMetaData(metadata),
            content,
        }
    }

    fn shallow_section_with_date(slug: &str, title: &str, date: &str) -> UnresolvedSection {
        let mut section = shallow_section(slug, title);
        section
            .metadata
            .0
            .insert("date".to_string(), HTMLContent::Plain(date.to_string()));
        section
    }

    #[test]
    fn test_shift_heading_levels_is_a_noop_at_the_top_level() {
        // Typst maps `= ` to `<h2>` on the assumption that the note's title is
        // `h1` — true for a note rendered as its own page.
        let html = "<h2>A</h2><p>x</p><h3>B</h3>";
        assert_eq!(shift_heading_levels(html, 1), html);
    }

    #[test]
    fn test_shift_heading_levels_follows_the_section_down() {
        let html = "<h2>A</h2><h3>B</h3>";
        assert_eq!(shift_heading_levels(html, 2), "<h3>A</h3><h4>B</h4>");
        assert_eq!(shift_heading_levels(html, 3), "<h4>A</h4><h5>B</h5>");
    }

    #[test]
    fn test_shift_heading_levels_clamps_at_h6() {
        // Deep nesting flattens at the bottom rather than inventing `h7`.
        assert_eq!(shift_heading_levels("<h4>A</h4>", 9), "<h6>A</h6>");
    }

    #[test]
    fn test_shift_heading_levels_rewrites_closing_tags_too() {
        assert_eq!(
            shift_heading_levels("<h2 id=\"x\">A</h2>", 2),
            "<h3 id=\"x\">A</h3>"
        );
    }

    #[test]
    fn test_shift_heading_levels_leaves_typst_head_alone() {
        // `typst_cli::html_to_body_content` keeps Typst's own `<head>` inside the
        // content; matching `<h` plus a digit is what keeps it out of reach.
        let html = "<head><style>a{}</style></head><body><p>x</p></body>";
        assert_eq!(shift_heading_levels(html, 3), html);
    }

    #[test]
    fn test_shift_heading_levels_leaves_links_alone() {
        // `html_link` output lands in the same Plain chunks and has no headings.
        let html = r#"<span class="link local"><a href="/x" title="t">T</a></span>"#;
        assert_eq!(shift_heading_levels(html, 4), html);
    }

    #[test]
    fn test_write_restores_a_page_deleted_from_the_output() {
        with_test_env(|| {
            let mut shallows = HashMap::new();
            shallows.insert(Slug::new("a"), shallow_section("a", "A"));

            let state = compile_all(&shallows).unwrap();
            let section = state.compiled().get(&Slug::new("a")).unwrap();
            let filepath = crate::environment::output_path("a.html");
            std::fs::create_dir_all(filepath.parent().unwrap()).unwrap();

            Writer::write(section, &state).unwrap();
            assert!(filepath.exists(), "first write should create the page");

            // The content hash is now current, so a writer that trusted it
            // alone would leave the output missing and still report success.
            std::fs::remove_file(&filepath).unwrap();
            Writer::write(section, &state).unwrap();

            assert!(
                filepath.exists(),
                "a page removed from the output should be rewritten"
            );
        });
    }

    #[test]
    fn test_html_doc_skips_header_when_parent_section_is_missing() {
        with_test_env(|| {
            let mut shallows = HashMap::new();
            shallows.insert(Slug::new("a"), shallow_section("a", "A"));

            let state = compile_all(&shallows).unwrap();
            let section = state.compiled().get(&Slug::new("a")).unwrap();
            let (html, _title) = Writer::html_doc(section, &state).unwrap();

            assert!(!html.contains(r#"class="header""#));
        });
    }

    #[test]
    fn test_html_doc_returns_error_for_invalid_bool_metadata() {
        with_test_env(|| {
            let mut shallows = HashMap::new();
            let mut section = shallow_section("a", "A");
            section.metadata.0.insert(
                KEY_REFERENCES.to_string(),
                HTMLContent::Plain("maybe".to_string()),
            );
            shallows.insert(Slug::new("a"), section);

            let state = compile_all(&shallows).unwrap();
            let section = state.compiled().get(&Slug::new("a")).unwrap();
            let err = Writer::html_doc(section, &state).unwrap_err();

            assert!(err.to_string().contains("invalid bool metadata"));
        });
    }

    #[test]
    fn test_html_doc_toc_uses_hash_link_for_internal_anonymous_subtree() {
        with_test_env(|| {
            let mut shallows = HashMap::new();
            shallows.insert(
                Slug::new("index"),
                shallow_section_with_content(
                    "index",
                    "Root",
                    HTMLContent::Lazy(vec![LazyContent::Embed(EmbedContent {
                        url: "/anon".to_string(),
                        title: None,
                        option: SectionOption::default(),
                    })]),
                ),
            );

            let mut anonymous = shallow_section_with_content(
                "anon",
                "Anonymous",
                HTMLContent::Lazy(vec![LazyContent::Embed(EmbedContent {
                    url: "/leaf".to_string(),
                    title: None,
                    option: SectionOption::default(),
                })]),
            );
            anonymous.metadata.0.insert(
                KEY_INTERNAL_ANON_SUBTREE.to_string(),
                HTMLContent::Plain("true".to_string()),
            );
            shallows.insert(Slug::new("anon"), anonymous);
            shallows.insert(Slug::new("leaf"), shallow_section("leaf", "Leaf"));

            let state = compile_all(&shallows).unwrap();
            let root = state.compiled().get(&Slug::new("index")).unwrap();
            let (html, _) = Writer::html_doc(root, &state).unwrap();

            let leaf_href = environment::full_html_url(Slug::new("leaf"));
            let anon_href = environment::full_html_url(Slug::new("anon"));
            let anon_hash_href = format!("#{}", crate::slug::to_hash_id("anon"));
            assert!(html.contains(&format!(r#"href="{}""#, leaf_href)));
            assert!(html.contains(&format!(r#"href="{}""#, anon_hash_href)));
            assert!(!html.contains(&format!(r#"href="{}""#, anon_href)));
        });
    }

    /// Pins the level arithmetic, which is off by one and easy to get wrong: a
    /// page emits no catalog row of its own, so its children are the outermost
    /// rows and must take the *first* bullet, not the second.
    #[test]
    fn test_html_doc_toc_bullets_step_down_with_nesting() {
        with_test_env(|| {
            let mut shallows = HashMap::new();
            let nest = |slug: &str, title: &str, child: &str| {
                shallow_section_with_content(
                    slug,
                    title,
                    HTMLContent::Lazy(vec![LazyContent::Embed(EmbedContent {
                        url: format!("/{}", child),
                        title: None,
                        option: SectionOption::default(),
                    })]),
                )
            };
            shallows.insert(Slug::new("index"), nest("index", "Root", "outer"));
            shallows.insert(Slug::new("outer"), nest("outer", "Outer", "middle"));
            shallows.insert(Slug::new("middle"), nest("middle", "Middle", "inner"));
            shallows.insert(Slug::new("inner"), shallow_section("inner", "Inner"));

            let state = compile_all(&shallows).unwrap();
            let root = state.compiled().get(&Slug::new("index")).unwrap();
            let (html, _) = Writer::html_doc(root, &state).unwrap();

            let bullet_of = |slug: &str| {
                let href = environment::full_html_url(Slug::new(slug));
                let anchor = html
                    .match_indices(&format!(r#"href="{}""#, href))
                    .map(|(i, _)| &html[i..])
                    .find(|rest| rest.contains("</a>"))
                    .unwrap_or_else(|| panic!("no catalog row for `{}`", slug));
                let text = &anchor[anchor.find('>').unwrap() + 1..];
                text[..text.find("</a>").unwrap()].to_string()
            };

            assert_eq!(bullet_of("outer"), "\u{25A0}");
            assert_eq!(bullet_of("middle"), "\u{25C6}");
            assert_eq!(bullet_of("inner"), "\u{25B8}");
        });
    }

    // --- numbering ---------------------------------------------------------

    /// Build `index`, embedding each child in order. `page` and each child may
    /// carry extra metadata, and each embed may override numbering.
    /// A child of the test page: slug, extra metadata, and its embed's override.
    type NumberingChild<'a> = (&'a str, &'a [(&'a str, &'a str)], Option<bool>);

    fn numbering_doc(page_meta: &[(&str, &str)], children: &[NumberingChild<'_>]) -> String {
        let mut shallows = HashMap::new();
        let embeds: Vec<LazyContent> = children
            .iter()
            .map(|(slug, _, numbering)| {
                LazyContent::Embed(EmbedContent {
                    url: format!("/{slug}"),
                    title: None,
                    option: SectionOption::new(*numbering, true, true),
                })
            })
            .collect();

        let mut page = shallow_section_with_content("index", "Root", HTMLContent::Lazy(embeds));
        for (k, v) in page_meta {
            page.metadata
                .0
                .insert(k.to_string(), HTMLContent::Plain(v.to_string()));
        }
        shallows.insert(Slug::new("index"), page);

        for (slug, meta, _) in children {
            let mut child = shallow_section(slug, slug);
            for (k, v) in *meta {
                child
                    .metadata
                    .0
                    .insert(k.to_string(), HTMLContent::Plain(v.to_string()));
            }
            shallows.insert(Slug::new(*slug), child);
        }

        let state = compile_all(&shallows).unwrap();
        let root = state.compiled().get(&Slug::new("index")).unwrap();
        Writer::html_doc(root, &state).unwrap().0
    }

    /// Every rendered label, in order: the taxon pill or the bare number.
    fn labels(html: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut cursor = 0;
        while let Some(i) = html[cursor..].find("class=\"section-title\"") {
            let start = cursor + i;
            let end = html[start..]
                .find("</h")
                .map(|e| start + e)
                .unwrap_or(html.len());
            let title = &html[start..end];
            let mut label = String::new();
            for class in ["section-number", "taxon"] {
                let open = format!("<span class=\"{class}\">");
                if let Some(o) = title.find(&open) {
                    let text_start = o + open.len();
                    if let Some(c) = title[text_start..].find("</span>") {
                        label.push_str(&title[text_start..text_start + c]);
                    }
                }
            }
            out.push(label);
            cursor = end;
        }
        out
    }

    #[test]
    fn test_numbering_is_off_without_metadata_or_config() {
        with_test_env(|| {
            let html = numbering_doc(&[], &[("a", &[(KEY_TAXON, "definition")], None)]);
            assert_eq!(labels(&html), vec!["", "Definition."]);
        });
    }

    #[test]
    fn test_note_metadata_turns_numbering_on_for_everything_inside() {
        with_test_env(|| {
            let html = numbering_doc(
                &[(KEY_NUMBERING, "true")],
                &[
                    ("a", &[(KEY_TAXON, "definition")], None),
                    ("b", &[(KEY_TAXON, "theorem")], None),
                ],
            );
            assert_eq!(labels(&html), vec!["", "Definition 1.", "Theorem 2."]);
        });
    }

    /// The case that is impossible without this change: a section with no taxon
    /// has nowhere to put a number, so it never got one.
    #[test]
    fn test_a_section_with_no_taxon_takes_a_bare_number() {
        with_test_env(|| {
            let html = numbering_doc(&[(KEY_NUMBERING, "true")], &[("a", &[], None)]);
            assert_eq!(labels(&html), vec!["", "1."]);
        });
    }

    /// The page is the only thing at its level, so a number there says nothing —
    /// and taking one would push everything else a level deeper.
    #[test]
    fn test_the_page_title_is_never_numbered() {
        with_test_env(|| {
            let html = numbering_doc(
                &[(KEY_NUMBERING, "true"), (KEY_TAXON, "theorem")],
                &[("a", &[(KEY_TAXON, "definition")], None)],
            );
            assert_eq!(labels(&html), vec!["Theorem.", "Definition 1."]);
        });
    }

    #[test]
    fn test_an_embed_can_opt_out_of_a_numbered_page() {
        with_test_env(|| {
            let html = numbering_doc(
                &[(KEY_NUMBERING, "true")],
                &[
                    ("a", &[(KEY_TAXON, "definition")], None),
                    ("b", &[(KEY_TAXON, "proof")], Some(false)),
                    ("c", &[(KEY_TAXON, "remark")], None),
                ],
            );
            assert_eq!(
                labels(&html),
                vec!["", "Definition 1.", "Proof.", "Remark 2."],
                "an opted-out block takes no number and no place in the sequence"
            );
        });
    }

    #[test]
    fn test_an_embed_can_opt_in_on_an_unnumbered_page() {
        with_test_env(|| {
            let html = numbering_doc(
                &[],
                &[
                    ("a", &[(KEY_TAXON, "definition")], Some(true)),
                    ("b", &[(KEY_TAXON, "remark")], None),
                ],
            );
            assert_eq!(labels(&html), vec!["", "Definition 1.", "Remark."]);
        });
    }

    /// A number describes a position on a page. An embedded note holds a
    /// different position on every page that embeds it, so its own metadata
    /// cannot be what decides.
    #[test]
    fn test_an_embedded_notes_own_numbering_metadata_is_ignored() {
        with_test_env(|| {
            let html = numbering_doc(
                &[],
                &[(
                    "a",
                    &[(KEY_TAXON, "definition"), (KEY_NUMBERING, "true")],
                    None,
                )],
            );
            assert_eq!(labels(&html), vec!["", "Definition."]);
        });
    }

    /// The two sequences at render level, and the case that prompted separating
    /// them: a heading is 1 even after statements, and a statement written once
    /// the heading has closed picks the top-level sequence back up.
    #[test]
    fn test_headings_and_statements_count_separately_across_the_page() {
        with_test_env(|| {
            let mut shallows = HashMap::new();
            let embed = |url: &str| {
                LazyContent::Embed(EmbedContent {
                    url: url.to_string(),
                    title: None,
                    option: SectionOption::default(),
                })
            };

            let mut page = shallow_section_with_content(
                "index",
                "Root",
                HTMLContent::Lazy(vec![embed("/first"), embed("/section"), embed("/after")]),
            );
            page.metadata.0.insert(
                KEY_NUMBERING.to_string(),
                HTMLContent::Plain("true".to_string()),
            );
            shallows.insert(Slug::new("index"), page);

            for (slug, taxon) in [
                ("first", "definition"),
                ("inside", "remark"),
                ("after", "definition"),
            ] {
                let mut section = shallow_section(slug, slug);
                section
                    .metadata
                    .0
                    .insert(KEY_TAXON.to_string(), HTMLContent::Plain(taxon.to_string()));
                shallows.insert(Slug::new(slug), section);
            }

            // No taxon, so this counts in the outline and holds a statement.
            shallows.insert(
                Slug::new("section"),
                shallow_section_with_content(
                    "section",
                    "A section",
                    HTMLContent::Lazy(vec![embed("/inside")]),
                ),
            );

            let state = compile_all(&shallows).unwrap();
            let root = state.compiled().get(&Slug::new("index")).unwrap();
            let html = Writer::html_doc(root, &state).unwrap().0;

            assert_eq!(
                labels(&html),
                vec![
                    "",              // the page itself
                    "Definition 1.", // a statement, before any section
                    "1.",            // the first section, despite following a statement
                    "Remark 1.1.",   // numbered inside that section
                    "Definition 2.", // back at top level, continuing 1
                ]
            );
        });
    }

    #[test]
    fn test_footer_entries_are_never_numbered() {
        with_test_env(|| {
            let mut shallows = HashMap::new();
            let mut page = shallow_section_with_content(
                "index",
                "Root",
                HTMLContent::Lazy(vec![LazyContent::Local(LocalLink {
                    url: "/target".to_string(),
                    text: None,
                })]),
            );
            page.metadata.0.insert(
                KEY_NUMBERING.to_string(),
                HTMLContent::Plain("true".to_string()),
            );
            shallows.insert(Slug::new("index"), page);
            shallows.insert(Slug::new("target"), shallow_section("target", "Target"));

            let state = compile_all(&shallows).unwrap();
            let target = state.compiled().get(&Slug::new("target")).unwrap();
            let html = Writer::html_doc(target, &state).unwrap().0;
            let footer = &html[html.rfind("<footer").unwrap_or(0)..];
            assert!(
                !footer.contains(r#"<span class="section-number">1."#),
                "a backlink entry must not claim a place in this page's sequence"
            );
        });
    }

    #[test]
    fn test_sort_footer_slugs_uses_parsed_dates_for_date_key() {
        with_test_env(|| {
            let mut shallows = HashMap::new();
            shallows.insert(
                Slug::new("old"),
                shallow_section_with_date("old", "Old", "January 2, 2020"),
            );
            shallows.insert(
                Slug::new("mid"),
                shallow_section_with_date("mid", "Mid", "2021-01-01"),
            );
            shallows.insert(
                Slug::new("new"),
                shallow_section_with_date("new", "New", "August 15, 2021"),
            );

            let state = compile_all(&shallows).unwrap();
            let mut slugs = vec![Slug::new("new"), Slug::new("old"), Slug::new("mid")];
            Writer::sort_footer_slugs(&mut slugs, &state, "date");

            assert_eq!(
                slugs,
                vec![Slug::new("old"), Slug::new("mid"), Slug::new("new")]
            );
        });
    }
}
