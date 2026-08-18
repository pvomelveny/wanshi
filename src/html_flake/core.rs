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

const CATALOG_BULLET_SYMBOL: &str = "\u{25A0}"; // ■
const HEADER_LOGO_PREFIX: &str = "\u{00AB} "; // «

pub fn html_article_inner(
    metadata: &EntryMetaData,
    contents: &String,
    hide_metadata: bool,
    open: bool,
    adhoc_title: Option<&str>,
    adhoc_taxon: Option<&str>,
) -> eyre::Result<String> {
    let summary = metadata.to_header(adhoc_title, adhoc_taxon, !hide_metadata)?;

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

pub fn html_footer_section(id: &str, summary: &str, content: &String) -> String {
    let summary = format!("<header><h1>{}</h1></header>", summary);
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
        a class="bullet" href={href} title={title_text} { (CATALOG_BULLET_SYMBOL) }
        span class="link local" onclick={onclick} {
            span class="taxon" { (taxon) }
            span class="title" { (title) }
        }
        (child_html)
    })
}

pub fn html_catalog_block(items: &str) -> String {
    let toc_text = environment::get_toc_text();
    html!(div class="block" {
        details open="" { summary { h1 { (toc_text) } } (items) }
    })
}

/// One row of a resolved `#query(...)` listing.
///
/// Deliberately reuses the `entry` classes that style the table of contents, so
/// listings inherit the same date column and bullet treatment. Unlike a catalog
/// item, every link here points at another page, so there is no hash anchor.
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
        a class="bullet" href={href.clone()} title={title_text.clone()} { (CATALOG_BULLET_SYMBOL) }
        a class="link local" href={href} title={title_text} {
            span class="taxon" { (taxon) }
            span class="title" { (title) }
        }
    })
}

/// Wrapper around a listing's rows, with an optional heading.
pub fn html_query_block(title: Option<&str>, items_html: &str) -> String {
    let heading = title.map_or(String::new(), |t| html!(h1 class="query-title" { (t) }));
    html!(div class="block query" {
        (heading)
        ul class="block" { (items_html) }
    })
}

pub fn html_footer(references_html: &str, backlinks_html: &str) -> String {
    html!(footer { (references_html) (backlinks_html) })
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
    use super::html_link;

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
