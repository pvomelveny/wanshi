// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use crate::{
    compiler::section::HTMLContent,
    entry::{EntryMetaData, MetaData},
    environment,
    html_macro::html,
    slug::Slug,
};

/// Table-of-contents bullets, one per level of indentation.
///
/// Shape rather than fill weight, because shape is what a reader can tell apart
/// without comparing two rows side by side.
const CATALOG_BULLET_SYMBOLS: [&str; 3] = ["\u{25A0}", "\u{25C6}", "\u{25B8}"]; // ■ ◆ ▸

const HEADER_LOGO_PREFIX: &str = "\u{00AB} "; // «

/// The bullet for a catalog row at `level`, counting the outermost row as 1.
///
/// Levels past the palette reuse its last glyph, the way [`super::heading_tag`]
/// clamps at `h6`. Catalog nesting has no depth limit at all, so there are
/// always levels with no glyph of their own; indentation is what separates
/// those, and it keeps working however deep the tree goes.
fn catalog_bullet(level: u8) -> &'static str {
    // Saturating rather than `level - 1`: a level of 0 means a caller lost
    // count, and rendering the top-level bullet is a better answer than a panic
    // over a decoration.
    let index = usize::from(level.saturating_sub(1));
    CATALOG_BULLET_SYMBOLS[index.min(CATALOG_BULLET_SYMBOLS.len() - 1)]
}

pub fn html_article_inner(
    metadata: &EntryMetaData,
    contents: &String,
    hide_metadata: bool,
    open: bool,
    adhoc_title: Option<&str>,
    adhoc_taxon: Option<&str>,
    level: u8,
) -> eyre::Result<String> {
    let summary = metadata.to_header(adhoc_title, adhoc_taxon, !hide_metadata, level)?;

    let article_id = metadata.id()?;
    Ok(html_section(
        &summary,
        contents,
        hide_metadata,
        open,
        article_id,
        metadata.data_taxon(),
    ))
}

/// A footer block — "References", "Backlinks".
///
/// `h2`, not `h1`: these sit below the article, so one level under the page's
/// own title. It also revives `footer h2` in the stylesheet, which matched
/// nothing while this emitted `h1`.
pub fn html_footer_section(id: &str, summary: &str, content: &String) -> String {
    let summary = format!("<header><h2>{}</h2></header>", summary);
    let inner_html = format!("{}{}", (html!(summary { (summary) })), content);
    let html_details = format!("<details open>{}</details>", inner_html);
    html!(section class="block link-list" id={id} { (html_details) })
}

pub fn html_section(
    summary: &String,
    content: &String,
    hide_metadata: bool,
    open: bool,
    id: String,
    data_taxon: Option<&String>,
) -> String {
    let mut class_name: Vec<&str> = vec!["block"];
    if hide_metadata {
        class_name.push("hide-metadata");
    }
    let data_taxon = data_taxon.map_or("", |s| s);
    let open = if open { "open" } else { "" };
    let inner_html = format!("{}{}", (html!(summary id={id} { (summary) })), content);
    let html_details = format!("<details {}>{}</details>", open, inner_html);
    html!(section class={class_name.join(" ")} data_taxon={data_taxon} { (html_details) })
}

/// One row of the table of contents.
///
/// Grouped into a struct rather than a positional list because the row grew to
/// eight fields, several of them adjacent `&str`s and `bool`s that a caller can
/// silently transpose. Mirrors [`super::HtmlHeaderArgs`].
pub struct CatalogItemArgs<'a> {
    pub slug: Slug,
    pub title: &'a str,
    pub page_title: &'a str,
    pub details_open: bool,
    pub taxon: &'a str,
    pub child_html: &'a str,
    /// Anonymous subtrees have no page of their own, so their row links to an
    /// anchor on the current page instead.
    pub use_hash_href: bool,
    /// Depth of this row in the catalog, outermost being 1. Selects the bullet.
    pub level: u8,
}

pub fn catalog_item(args: CatalogItemArgs<'_>) -> String {
    let CatalogItemArgs {
        slug,
        title,
        page_title,
        details_open,
        taxon,
        child_html,
        use_hash_href,
        level,
    } = args;

    let hash_href = format!("#{}", crate::slug::to_hash_id(slug.as_str()));
    let href = if use_hash_href {
        hash_href.clone()
    } else {
        environment::full_html_url(slug)
    };
    let title_text = format!("{} [{}]", page_title, slug);
    let onclick = format!("window.location.href='{}'", hash_href);

    let mut class_name: Vec<String> = vec!["entry".to_string()];
    if !details_open {
        class_name.push("item-summary".to_string());
    }

    // No date column here, unlike a listing row. A catalog lists the sections of
    // the page you are already reading, where the dates are near-identical and
    // uninformative, and the column cost the title enough width to wrap early.
    html!(li class={class_name.join(" ")} {
        a class="bullet" href={href} title={title_text} { (catalog_bullet(level)) }
        span class="link local" onclick={onclick} {
            span class="taxon" { (taxon) }
            span class="title" { (title) }
        }
        (child_html)
    })
}

/// The table of contents in the sidebar.
///
/// `h2` with its own class: it sits in a `<nav>` beside the article, not above
/// it, so it is not the page's `h1`. The class is needed because it used to be
/// sized by `details h1`, which now matches section titles only.
pub fn html_catalog_block(items: &str) -> String {
    let toc_text = environment::get_toc_text();
    html!(div class="block" {
        details open="" { summary { h2 class="toc-title" { (toc_text) } } (items) }
    })
}

/// One row of a resolved `#query(...)` listing.
///
/// Deliberately reuses the `entry` classes that style the table of contents, so
/// listings inherit the same date column and bullet treatment. Unlike a catalog
/// item, every link here points at another page, so there is no hash anchor.
///
/// A listing is flat however deeply nested the notes it names are, so the
/// per-level bullets of [`catalog_bullet`] do not apply: every row takes the
/// top-level glyph.
pub fn html_query_item(
    slug: Slug,
    title: &str,
    page_title: &str,
    taxon: &str,
    date: Option<&str>,
) -> String {
    let href = environment::full_html_url(slug);
    let title_text = format!("{} [{}]", page_title, slug);
    let date_html = date.map_or(String::new(), |d| html!(span class="date" { (d) }));

    html!(li class="entry" {
        (date_html)
        a class="bullet" href={href.clone()} title={title_text.clone()} { (catalog_bullet(1)) }
        a class="link local" href={href} title={title_text} {
            span class="taxon" { (taxon) }
            span class="title" { (title) }
        }
    })
}

/// Wrapper around a listing's rows, with an optional heading.
///
/// The title is `h2` for the same reason Typst emits `h2` for `= `: it sits
/// inside a section whose own title is `h1`, so it is one level down. Listings
/// are substituted into the section's body before rendering, so the same
/// depth shift that moves Typst's headings moves this one too.
pub fn html_query_block(title: Option<&str>, items_html: &str) -> String {
    let heading = title.map_or(String::new(), |t| html!(h2 class="query-title" { (t) }));
    html!(div class="block query" {
        (heading)
        ul class="block" { (items_html) }
    })
}

/// The three reverse views of a note, in order: what it cites, what cites it,
/// what contains it. Each is omitted when empty.
pub fn html_footer(references_html: &str, backlinks_html: &str, embedded_by_html: &str) -> String {
    html!(footer { (references_html) (backlinks_html) (embedded_by_html) })
}

pub fn html_link(href: &str, title: &str, text: &str, class_name: &str) -> String {
    let plain_title = HTMLContent::Plain(title.to_string()).remove_all_tags();
    let escaped_href = htmlize::escape_attribute(href);
    let escaped_title = htmlize::escape_attribute(&plain_title);
    let escaped_class = htmlize::escape_attribute(class_name);
    format!(
        r#"<span class="link {}"><a href="{}" title="{}">{}</a></span>"#,
        escaped_class, escaped_href, escaped_title, text
    )
}

pub fn html_header_nav(title: &str, page_title: &str, href: &str) -> String {
    let onclick = format!("window.location.href='{}'", href);
    html!(header class="header" {
        nav class="nav" {
            div class="logo" {
                span class="cursor-pointer" onclick={onclick} title={page_title} {
                    (HEADER_LOGO_PREFIX) (title)
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{catalog_bullet, html_link, CATALOG_BULLET_SYMBOLS};

    #[test]
    fn test_catalog_bullet_differs_for_each_level_of_the_palette() {
        assert_eq!(catalog_bullet(1), "\u{25A0}");
        assert_eq!(catalog_bullet(2), "\u{25C6}");
        assert_eq!(catalog_bullet(3), "\u{25B8}");
    }

    #[test]
    fn test_catalog_bullet_clamps_past_the_palette() {
        // Catalog nesting has no depth limit, so this is reachable, not
        // defensive: past the palette every level keeps the deepest glyph and
        // indentation carries the structure.
        let deepest = CATALOG_BULLET_SYMBOLS[CATALOG_BULLET_SYMBOLS.len() - 1];
        assert_eq!(catalog_bullet(4), deepest);
        assert_eq!(catalog_bullet(6), deepest);
        assert_eq!(catalog_bullet(u8::MAX), deepest);
    }

    #[test]
    fn test_catalog_bullet_treats_level_zero_as_the_top_level() {
        assert_eq!(catalog_bullet(0), catalog_bullet(1));
    }

    #[test]
    fn test_html_link_escapes_title_attribute() {
        let html = html_link(
            "/AC2C",
            r#"<span lang="zh">abc</span> [AC2C]""#,
            r#"<span lang="zh">abc</span>"#,
            "local",
        );
        assert!(html.contains(r#"href="/AC2C""#));
        assert!(html.contains(r#"title="abc [AC2C]&quot;""#));
        assert!(!html.contains("&lt;span"));
        assert!(!html.contains("&lt;/span&gt;"));
        assert!(!html.contains(r#"title="<span lang="zh">"#));
        assert!(html.contains(r#"><span lang="zh">abc</span></a>"#));
    }
}
