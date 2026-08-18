// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use crate::{
    entry::{EntryMetaData, KEY_DATE},
    environment::{self, input_path},
    html_macro::html,
    slug::Slug,
};

/// The row under a title: the date, and the note's custom keys.
///
/// `show_extra` drops everything but the date. Custom keys are rendered as
/// bare values with no key name, which reads as stray words wherever the
/// section is not the subject of the page — a footer entry would otherwise
/// print another note's author and status as if they were this page's.
pub fn html_header_metadata(etc: &[(String, String)], show_extra: bool) -> String {
    // `[build].header-keys` decides what a note prints under its title, in the
    // order listed. Anything absent from it is still in `wanshi.json` and still
    // sorts and filters; it just does not appear, because a bare value with no
    // key name beside it means little to anyone but its author.
    let allowed = environment::header_keys();
    let shows = |key: &str| allowed.iter().any(|k| k == key);

    let date = etc
        .iter()
        .find(|(key, _)| key == KEY_DATE)
        .map(|(_, value)| value)
        .filter(|_| shows(KEY_DATE));

    let mut items = String::new();
    if show_extra {
        for key in &allowed {
            if key == KEY_DATE {
                continue;
            }
            if let Some((_, value)) = etc.iter().find(|(k, _)| k == key) {
                items.push_str(&html!(li class="meta-item" { (value) }));
            }
        }
    }
    let rest = html!(div class="metadata" { ul { (items) } });

    match date {
        Some(date) => html!(div class="metadata-row" {
            span class="date" { (date) }
            (rest)
        }),
        None => rest,
    }
}

pub struct HtmlHeaderArgs<'a> {
    pub title: &'a str,
    pub taxon: &'a str,
    pub slug: &'a Slug,
    pub ext: &'a str,
    pub show_slug: bool,
    pub source_slug: Option<&'a str>,
    pub source_pos: Option<&'a str>,
    pub etc: &'a [(String, String)],
    /// Whether the note's custom keys appear beneath the title.
    pub show_extra: bool,
}

pub fn html_header(args: HtmlHeaderArgs<'_>) -> String {
    let HtmlHeaderArgs {
        title,
        taxon,
        slug,
        ext,
        show_slug,
        source_slug,
        source_pos,
        etc,
        show_extra,
    } = args;
    let slug_str = slug.as_str();
    let source_slug = source_slug.unwrap_or(slug_str);
    let is_serve = environment::is_serve();
    let serve_edit = environment::editor_url();
    let deploy_edit = environment::deploy_edit_url();

    let slug_text = EntryMetaData::to_slug_text(slug_str);
    let slug_url = environment::full_html_url(*slug);
    let slug_link = if show_slug {
        html!(a class="slug" href={slug_url} { "["(slug_text)"]" })
    } else {
        String::new()
    };

    let edit_text = environment::get_edit_text();
    let hash_anchor = if !show_slug {
        let hash_id = crate::slug::to_hash_id(slug_str);
        html!(a class="hash" href={format!("#{hash_id}")} { "[#]" })
    } else {
        String::new()
    };
    let edit_url = match (is_serve, serve_edit, deploy_edit) {
        (true, Some(prefix), _) => {
            let source_path = input_path(format!("{}.{}", source_slug, ext));
            let editor_url = (|| {
                let source_path = source_path.canonicalize().ok()?;
                let source_url = url::Url::from_file_path(source_path).ok()?;
                let base = url::Url::parse(&prefix).ok()?;
                let url = base.join(source_url.path()).ok()?.to_string();
                Some(append_editor_position(url, &prefix, source_pos))
            })();

            match editor_url {
                Some(url) => html!(a class="edit" href={url} { (edit_text) }),
                None => {
                    color_print::ceprintln!(
                        "<y>Warning: failed to construct editor URL for `{}` (source `{}`).</>",
                        slug,
                        source_slug
                    );
                    String::new()
                }
            }
        }
        (false, _, Some(prefix)) => {
            let source_path = format!("{}.{}", source_slug, ext);
            let editor_url =
                append_editor_position(format!("{}{}", prefix, source_path), &prefix, source_pos);
            html!(a class="edit" href={editor_url.to_string()} { (edit_text) })
        }
        _ => String::default(),
    };

    html!(header {
        h1 {
            span class="taxon" { (taxon) }
            span class="title" { (title) } " "
            (slug_link)
            (hash_anchor)
            (edit_url)
        }
        (html_header_metadata(etc, show_extra))
    })
}

fn append_editor_position(url: String, prefix: &str, source_pos: Option<&str>) -> String {
    if !is_vscode_family_file_url(prefix) {
        return url;
    }
    let Some(pos) = source_pos else {
        return url;
    };
    if parse_source_pos(pos).is_none() {
        return url;
    }
    format!("{url}:{pos}")
}

fn is_vscode_family_file_url(prefix: &str) -> bool {
    [
        "vscode://file",
        "vscode-insiders://file",
        "vsc://file",
        "vscodium://file",
    ]
    .iter()
    .any(|candidate| prefix.starts_with(candidate))
}

fn parse_source_pos(pos: &str) -> Option<(usize, usize)> {
    let (line, col) = pos.split_once(':')?;
    let line = line.parse::<usize>().ok()?;
    let col = col.parse::<usize>().ok()?;
    if line == 0 || col == 0 {
        return None;
    }
    Some((line, col))
}

#[cfg(test)]
mod tests {
    use super::{append_editor_position, parse_source_pos};
    use crate::slug::Slug;

    #[test]
    fn test_append_editor_position_for_vscode() {
        let url = append_editor_position(
            "vscode://file/c:/repo/docs/trees/book/index.md".to_string(),
            "vscode://file/",
            Some("12:3"),
        );
        assert_eq!(url, "vscode://file/c:/repo/docs/trees/book/index.md:12:3");
    }

    #[test]
    fn test_append_editor_position_for_vscode_family() {
        let cases = [
            (
                "vscode-insiders://file/c:/repo/docs/trees/book/index.md",
                "vscode-insiders://file/",
            ),
            ("vsc://file/c:/repo/docs/trees/book/index.md", "vsc://file/"),
            (
                "vscodium://file/c:/repo/docs/trees/book/index.md",
                "vscodium://file/",
            ),
        ];

        for (url, prefix) in cases {
            let with_pos = append_editor_position(url.to_string(), prefix, Some("12:3"));
            assert_eq!(with_pos, format!("{url}:12:3"));
        }
    }

    #[test]
    fn test_append_editor_position_ignores_non_vscode_family_or_invalid_pos() {
        let web = append_editor_position(
            "https://example.com/edit/path".to_string(),
            "https://example.com/edit/",
            Some("12:3"),
        );
        assert_eq!(web, "https://example.com/edit/path");

        let invalid = append_editor_position(
            "vscode://file/c:/repo/docs/trees/book/index.md".to_string(),
            "vscode://file/",
            Some("0:3"),
        );
        assert_eq!(invalid, "vscode://file/c:/repo/docs/trees/book/index.md");
    }

    #[test]
    fn test_parse_source_pos() {
        assert_eq!(parse_source_pos("1:1"), Some((1, 1)));
        assert_eq!(parse_source_pos("12:3"), Some((12, 3)));
        assert_eq!(parse_source_pos("0:3"), None);
        assert_eq!(parse_source_pos("abc"), None);
    }

    #[test]
    fn test_html_header_can_hide_slug_link() {
        let etc = Vec::new();
        let html = super::html_header(super::HtmlHeaderArgs {
            title: "Title",
            taxon: "Taxon. ",
            slug: &Slug::new("book/child"),
            ext: "typst",
            show_slug: false,
            source_slug: None,
            source_pos: None,
            etc: &etc,
            show_extra: true,
        });
        assert!(!html.contains("class=\"slug\""));
        assert!(html.contains("href=\"#book-child\""));
        assert!(html.contains(">[#]</a>"));
    }

    #[test]
    fn test_header_metadata_prints_only_the_configured_keys() {
        crate::environment::mock_environment().unwrap();

        let etc = vec![
            ("date".to_string(), "2026-08-17".to_string()),
            ("author".to_string(), "Someone".to_string()),
            ("status".to_string(), "stable".to_string()),
        ];

        // Defaults are date and author. `status` is a perfectly good key that
        // sorts and filters; it simply is not printed unless asked for.
        let shown = super::html_header_metadata(&etc, true);
        assert!(shown.contains("2026-08-17"));
        assert!(shown.contains("Someone"));
        assert!(
            !shown.contains("stable"),
            "an unlisted key should stay out of the header"
        );

        // A footer entry renders another note's header; its custom keys would
        // read as stray words belonging to the page doing the linking.
        let hidden = super::html_header_metadata(&etc, false);
        assert!(hidden.contains("2026-08-17"), "the date still belongs");
        assert!(!hidden.contains("Someone"), "custom keys should be dropped");
    }
}
