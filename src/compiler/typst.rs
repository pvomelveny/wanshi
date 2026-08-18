// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Alias Qli (@AliasQli), Spore (@s-cerevisiae), Kokic (@kokic)

use super::anonymous_slug::AnonymousSlugState;
use super::heading_sections::split_heading_sections;
use super::subtree_slug::{ensure_unique_section_slugs, resolve_subtree_slug};
use camino::Utf8Path;
use eyre::{eyre, WrapErr};

use super::custom_tag::{HTMLParser, HTMLTagKind};
use super::section::{EmbedContent, LocalLink, QueryOrder, QueryScope, QuerySpec, SectionOption};
use super::section::{HTMLContent, HTMLContentBuilder, LazyContent};
use super::UnresolvedSection;
use crate::{
    entry::{
        HTMLMetaData, KEY_EXT, KEY_INTERNAL_ANON_SUBTREE, KEY_SLUG, KEY_SOURCE_SLUG, KEY_TAXON,
        KEY_TITLE,
    },
    ordered_map::OrderedMap,
    slug::{Ext, Slug},
    typst_cli,
};

use std::{borrow::Cow, collections::HashSet, str};

fn parse_bool(m: Option<&Cow<'_, str>>, def: bool) -> bool {
    match m.map(|s| s.as_ref()) {
        None | Some("auto") => def,
        Some("false") | Some("0") | Some("none") => false,
        _ => true,
    }
}

/// State shared across one source file's parse: the identity of the file being
/// parsed plus the accumulators that every nested subtree contributes to.
struct ParseContext<'a> {
    source_slug: Slug,
    /// Extension of the source file, recorded on every section it produces so
    /// edit links can reconstruct the original path.
    ext: Ext,
    subtree_sections: &'a mut Vec<(Slug, UnresolvedSection)>,
    used_slugs: &'a mut HashSet<Slug>,
    anonymous_slugs: &'a mut AnonymousSlugState,
}

impl ParseContext<'_> {
    fn ext_content(&self) -> HTMLContent {
        HTMLContent::Plain(self.ext.to_string())
    }
}

fn parse_typst_html(
    ctx: &mut ParseContext<'_>,
    html_str: &str,
    current_slug: Slug,
    metadata: &mut OrderedMap<String, HTMLContent>,
    allow_subtree: bool,
) -> eyre::Result<HTMLContent> {
    let source_slug = ctx.source_slug;
    let mut builder = HTMLContentBuilder::new();
    let mut cursor: usize = 0;

    for span in HTMLParser::new(html_str) {
        let span = span.wrap_err("failed to parse wanshi tag from typst html")?;

        builder.push_str(&html_str[cursor..span.start]);
        cursor = span.end;

        let attr = |attr_name: &str| {
            span.attrs
                .get(attr_name)
                .ok_or_else(|| eyre!("missing attribute `{attr_name}` in wanshi tag"))
        };

        let value = || {
            let value = span
                .attrs
                .get("value")
                .map_or_else(|| span.body.to_string(), |s| s.to_string());
            if value.is_empty() {
                None
            } else {
                Some(value)
            }
        };
        match span.kind {
            HTMLTagKind::Meta => {
                let key = attr("key")?.as_ref();
                // The taxon is stored exactly as authored. Capitalisation and the
                // trailing ". " are display concerns and are applied when a taxon
                // is rendered, not baked into the metadata — see `display_taxon`.
                let val = if let Some(value) = span.attrs.get("value") {
                    HTMLContent::Plain(value.to_string())
                } else {
                    parse_typst_html(ctx, span.body, current_slug, &mut OrderedMap::new(), false)?
                };
                metadata.insert(key.to_string(), val);
            }
            HTMLTagKind::Embed => {
                let def = SectionOption::default();

                let url = attr("url")?.to_string();
                let title = value();
                let numbering = parse_bool(span.attrs.get("numbering"), def.numbering);
                let details_open = parse_bool(span.attrs.get("open"), def.details_open);
                let catalog = parse_bool(span.attrs.get("catalog"), def.catalog);
                builder.push(LazyContent::Embed(EmbedContent {
                    url,
                    title,
                    option: SectionOption::new(numbering, details_open, catalog),
                }))
            }
            HTMLTagKind::Local { span: _ } => {
                let url = attr(KEY_SLUG)?.to_string();
                let text = value();
                builder.push(LazyContent::Local(LocalLink { url, text }))
            }
            HTMLTagKind::Outdent => {
                if !allow_subtree {
                    return Err(eyre!(
                        "typst outdent tag is not allowed in metadata value while parsing `{}`",
                        source_slug
                    ));
                }
                builder.push(LazyContent::Outdent)
            }
            HTMLTagKind::Query => {
                if !allow_subtree {
                    return Err(eyre!(
                        "typst query tag is not allowed in metadata value while parsing `{}`",
                        source_slug
                    ));
                }

                let optional = |name: &str| {
                    span.attrs
                        .get(name)
                        .map(|s| s.to_string())
                        .filter(|s| !s.is_empty())
                };

                let limit = match span.attrs.get("limit") {
                    Some(raw) => Some(raw.parse::<usize>().map_err(|_| {
                        eyre!(
                            "invalid `limit` in query within `{}`: `{}` (expected a whole number)",
                            source_slug,
                            raw
                        )
                    })?),
                    None => None,
                };

                builder.push(LazyContent::Query(QuerySpec {
                    // Bound to the section that wrote the query, so an embedded
                    // copy still lists that section's own children.
                    owner: current_slug,
                    scope: QueryScope::parse(attr("from")?.as_ref()),
                    taxon: optional("taxon"),
                    key: optional("key"),
                    value: optional("value"),
                    sort: optional("sort").unwrap_or_else(|| "date".to_string()),
                    order: QueryOrder::parse(
                        span.attrs.get("order").map(|s| s.as_ref()).unwrap_or("asc"),
                    ),
                    limit,
                    title: optional("title"),
                    include_indexes: parse_bool(
                        span.attrs.get("include-indexes"),
                        crate::compiler::section::include_indexes_default(),
                    ),
                }))
            }
            HTMLTagKind::Subtree => {
                if !allow_subtree {
                    return Err(eyre!(
                        "typst subtree tag is not allowed in metadata value while parsing `{}`",
                        source_slug
                    ));
                }

                let (subtree_slug, anonymous) = if let Some(raw_slug) = span.attrs.get("slug") {
                    let raw_slug = raw_slug.as_ref();
                    let subtree_slug =
                        resolve_subtree_slug(current_slug, raw_slug).wrap_err_with(|| {
                            eyre!(
                                "invalid typst subtree slug `{}` in `{}`",
                                raw_slug,
                                source_slug
                            )
                        })?;
                    if !ctx.used_slugs.insert(subtree_slug) {
                        return Err(eyre!(
                            "duplicate typst subtree slug `{}` generated from `{}`",
                            subtree_slug,
                            source_slug
                        ));
                    }
                    (subtree_slug, false)
                } else {
                    (
                        ctx.anonymous_slugs
                            .allocate_with_used(source_slug, ctx.used_slugs),
                        true,
                    )
                };

                let def = SectionOption::default();
                let numbering = parse_bool(span.attrs.get("numbering"), def.numbering);
                let details_open = parse_bool(span.attrs.get("open"), def.details_open);
                let catalog = parse_bool(span.attrs.get("catalog"), def.catalog);
                let option = SectionOption::new(numbering, details_open, catalog);

                let title = span
                    .attrs
                    .get("title")
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty());
                let taxon = span
                    .attrs
                    .get("taxon")
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty());

                builder.push(LazyContent::Embed(EmbedContent {
                    url: format!("/{subtree_slug}"),
                    title: title.clone(),
                    option,
                }));

                let mut subtree_metadata = OrderedMap::new();
                subtree_metadata.insert(
                    KEY_SLUG.to_string(),
                    HTMLContent::Plain(subtree_slug.to_string()),
                );
                subtree_metadata.insert(KEY_EXT.to_string(), ctx.ext_content());
                subtree_metadata.insert(
                    KEY_SOURCE_SLUG.to_string(),
                    HTMLContent::Plain(source_slug.to_string()),
                );
                if anonymous {
                    subtree_metadata.insert(
                        KEY_INTERNAL_ANON_SUBTREE.to_string(),
                        HTMLContent::Plain("true".to_string()),
                    );
                }
                let nested_current_slug = if anonymous {
                    current_slug
                } else {
                    subtree_slug
                };
                let subtree_content = parse_typst_html(
                    ctx,
                    span.body,
                    nested_current_slug,
                    &mut subtree_metadata,
                    true,
                )
                .wrap_err_with(|| {
                    eyre!(
                        "failed to parse typst subtree section `{}` in `{}`",
                        subtree_slug,
                        source_slug
                    )
                })?;
                apply_subtree_defaults(&mut subtree_metadata, title.as_deref(), taxon.as_deref());
                ctx.subtree_sections.push((
                    subtree_slug,
                    UnresolvedSection {
                        metadata: HTMLMetaData(subtree_metadata),
                        content: subtree_content,
                    },
                ));
            }
        }
    }

    builder.push_str(&html_str[cursor..]);

    Ok(builder.build())
}

fn apply_subtree_defaults(
    metadata: &mut OrderedMap<String, HTMLContent>,
    title: Option<&str>,
    taxon: Option<&str>,
) {
    if !metadata.contains_key(KEY_TITLE) {
        if let Some(title) = title {
            metadata.insert(KEY_TITLE.to_string(), HTMLContent::Plain(title.to_string()));
        }
    }
    if !metadata.contains_key(KEY_TAXON) {
        if let Some(taxon) = taxon {
            // Stored as authored; formatted at render time.
            metadata.insert(KEY_TAXON.to_string(), HTMLContent::Plain(taxon.to_string()));
        }
    }
}

fn parse_typst_sections_from_html(
    source_slug: Slug,
    ext: Ext,
    html_str: &str,
) -> eyre::Result<Vec<(Slug, UnresolvedSection)>> {
    let mut metadata: OrderedMap<String, HTMLContent> = OrderedMap::new();
    metadata.insert(
        KEY_SLUG.to_string(),
        HTMLContent::Plain(source_slug.to_string()),
    );
    metadata.insert(KEY_EXT.to_string(), HTMLContent::Plain(ext.to_string()));
    metadata.insert(
        KEY_SOURCE_SLUG.to_string(),
        HTMLContent::Plain(source_slug.to_string()),
    );

    let mut used_slugs = HashSet::from([source_slug]);
    let mut anonymous_slugs = AnonymousSlugState::default();
    let mut subtree_sections = Vec::new();
    let mut ctx = ParseContext {
        source_slug,
        ext,
        subtree_sections: &mut subtree_sections,
        used_slugs: &mut used_slugs,
        anonymous_slugs: &mut anonymous_slugs,
    };
    let content = parse_typst_html(&mut ctx, html_str, source_slug, &mut metadata, true)?;

    // Turn each content stream's headings into sections. Every stream is split
    // on its own, which is what keeps `#outdent()` from escaping the subtree it
    // was written in: it can only close frames this pass opened.
    let mut sections = Vec::with_capacity(subtree_sections.len() + 1);
    let mut heading_sections = Vec::new();

    let split = split_heading_sections(
        content,
        source_slug,
        source_slug,
        ext,
        &mut used_slugs,
        &mut anonymous_slugs,
    );
    heading_sections.extend(split.sections);
    sections.push((
        source_slug,
        UnresolvedSection {
            metadata: HTMLMetaData(metadata),
            content: split.content,
        },
    ));

    for (slug, section) in subtree_sections {
        let split = split_heading_sections(
            section.content,
            slug,
            source_slug,
            ext,
            &mut used_slugs,
            &mut anonymous_slugs,
        );
        heading_sections.extend(split.sections);
        sections.push((
            slug,
            UnresolvedSection {
                content: split.content,
                metadata: section.metadata,
            },
        ));
    }

    sections.extend(heading_sections);
    ensure_unique_section_slugs(&sections, source_slug, "typst subtree")?;
    Ok(sections)
}

pub fn parse_typst_sections<P: AsRef<Utf8Path>>(
    slug: Slug,
    ext: Ext,
    root_dir: P,
) -> eyre::Result<Vec<(Slug, UnresolvedSection)>> {
    let typst_root_dir = root_dir.as_ref();
    let relative_path = super::incremental::source_relative_path(slug, ext);
    let html_str = typst_cli::file_to_html(relative_path.as_str(), typst_root_dir.as_ref())
        .wrap_err_with(|| eyre!("failed to compile typst file `{relative_path}` to html"))?;

    parse_typst_sections_from_html(slug, ext, &html_str)
        .wrap_err_with(|| eyre!("failed to parse typst html structure in `{relative_path}`"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        compiler::{
            anonymous_slug::{anonymous_slug_for, ANON_SUBTREE_ORDINAL_INITIAL},
            section::LazyContent,
        },
        entry::{MetaData, KEY_INTERNAL_ANON_SUBTREE},
    };

    fn find_section(sections: &[(Slug, UnresolvedSection)], slug: Slug) -> &UnresolvedSection {
        sections
            .iter()
            .find_map(|(s, section)| (*s == slug).then_some(section))
            .expect("expected section")
    }

    #[test]
    fn test_parse_typst_sections_extracts_named_subtree() {
        let html = r#"
<p>root</p>
<wanshi-subtree slug="child" title="Child" numbering="true"><p>child</p></wanshi-subtree>
"#;
        let sections =
            parse_typst_sections_from_html(Slug::new("book/index"), Ext::Typst, html).unwrap();
        assert_eq!(sections.len(), 2);

        let root = find_section(&sections, Slug::new("book/index"));
        let root_contents = match &root.content {
            HTMLContent::Lazy(contents) => contents,
            _ => panic!("expected lazy root content"),
        };
        let embed = root_contents
            .iter()
            .find_map(|content| match content {
                LazyContent::Embed(embed) => Some(embed),
                _ => None,
            })
            .expect("expected subtree embed");
        assert_eq!(embed.url, "/book/child");
        assert_eq!(embed.title.as_deref(), Some("Child"));
        assert!(embed.option.numbering);

        let child = find_section(&sections, Slug::new("book/child"));
        assert_eq!(
            child
                .metadata
                .title()
                .and_then(HTMLContent::as_string)
                .map(String::as_str),
            Some("Child")
        );
        assert_eq!(child.metadata.ext().map(String::as_str), Some("typst"));
        assert_eq!(
            child.metadata.get_str(KEY_SOURCE_SLUG).map(String::as_str),
            Some("book/index")
        );
    }

    #[test]
    fn test_parse_typst_sections_records_the_source_extension_on_every_section() {
        // The extension must come from the parsed source, not a literal: it is
        // what edit links use to rebuild the original file path, so a section
        // claiming the wrong extension links to a file that does not exist.
        let html = r#"
<p>root</p>
<wanshi-subtree slug="child" title="Child"><p>child</p></wanshi-subtree>
"#;
        let sections =
            parse_typst_sections_from_html(Slug::new("book/index"), Ext::Typst, html).unwrap();

        let expected = Ext::Typst.to_string();
        for (slug, section) in &sections {
            assert_eq!(
                section.metadata.ext().map(String::as_str),
                Some(expected.as_str()),
                "section `{slug}` should inherit the source extension"
            );
        }
    }

    #[test]
    fn test_parse_typst_sections_subtree_body_metadata_overrides_attr_defaults() {
        let html = r#"
<wanshi-subtree slug="child" title="Outer">
<wanshi-meta key="title" value="Inner"></wanshi-meta>
<p>child</p>
</wanshi-subtree>
"#;
        let sections =
            parse_typst_sections_from_html(Slug::new("book/index"), Ext::Typst, html).unwrap();
        let root = find_section(&sections, Slug::new("book/index"));
        let root_contents = match &root.content {
            HTMLContent::Lazy(contents) => contents,
            _ => panic!("expected lazy root content"),
        };
        let embed = root_contents
            .iter()
            .find_map(|content| match content {
                LazyContent::Embed(embed) => Some(embed),
                _ => None,
            })
            .expect("expected subtree embed");
        assert_eq!(embed.title.as_deref(), Some("Outer"));

        let child = find_section(&sections, Slug::new("book/child"));
        assert_eq!(
            child
                .metadata
                .title()
                .and_then(HTMLContent::as_string)
                .map(String::as_str),
            Some("Inner")
        );
    }

    #[test]
    fn test_parse_typst_sections_extracts_anonymous_subtree() {
        let html = r#"
<p>root</p>
<wanshi-subtree title="Anonymous"><p>child</p></wanshi-subtree>
"#;
        let sections =
            parse_typst_sections_from_html(Slug::new("book/index"), Ext::Typst, html).unwrap();
        assert_eq!(sections.len(), 2);

        let root = find_section(&sections, Slug::new("book/index"));
        let root_contents = match &root.content {
            HTMLContent::Lazy(contents) => contents,
            _ => panic!("expected lazy root content"),
        };
        let embed = root_contents
            .iter()
            .find_map(|content| match content {
                LazyContent::Embed(embed) => Some(embed),
                _ => None,
            })
            .expect("expected subtree embed");
        let anonymous_slug =
            anonymous_slug_for(Slug::new("book/index"), ANON_SUBTREE_ORDINAL_INITIAL);
        assert_eq!(embed.url, format!("/{anonymous_slug}"));
        assert_eq!(embed.title.as_deref(), Some("Anonymous"));

        let anonymous = find_section(&sections, anonymous_slug);
        assert_eq!(
            anonymous
                .metadata
                .get_str(KEY_INTERNAL_ANON_SUBTREE)
                .map(String::as_str),
            Some("true")
        );
    }

    #[test]
    fn test_parse_typst_sections_nested_named_subtree_under_anonymous_wrapper_uses_visible_prefix()
    {
        let html = r#"
<wanshi-subtree>
  <wanshi-subtree slug="child"><p>nested</p></wanshi-subtree>
</wanshi-subtree>
"#;
        let sections =
            parse_typst_sections_from_html(Slug::new("book/index"), Ext::Typst, html).unwrap();
        assert!(sections
            .iter()
            .any(|(slug, _)| *slug == Slug::new("book/child")));

        let anonymous = sections
            .iter()
            .find_map(|(_, section)| {
                section
                    .metadata
                    .get_str(KEY_INTERNAL_ANON_SUBTREE)
                    .is_some_and(|value| value == "true")
                    .then_some(section)
            })
            .expect("expected anonymous wrapper section");
        let HTMLContent::Lazy(contents) = &anonymous.content else {
            panic!("expected lazy anonymous content");
        };
        let nested_embed_urls: Vec<_> = contents
            .iter()
            .filter_map(|content| match content {
                LazyContent::Embed(embed) => Some(embed.url.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(nested_embed_urls, vec!["/book/child".to_string()]);
    }
}
