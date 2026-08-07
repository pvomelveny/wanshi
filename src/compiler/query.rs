// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.

//! Resolution of `#query(...)` listings.
//!
//! Listings cannot be resolved while sections are being compiled, because they
//! ask questions about the finished graph — which sections are children of this
//! one, what the most recent notes are — and that graph does not exist until
//! every section has been compiled. Resolution therefore runs as a pass over the
//! completed [`CompileState`].
//!
//! Each [`QuerySpec`] carries the slug of the section that wrote it, so the pass
//! can descend into embedded copies of a section and still resolve their
//! listings against the original author rather than the host page.

use std::collections::{HashMap, HashSet};

use crate::{
    entry::{MetaData, KEY_DATE, KEY_TAXON, KEY_TITLE},
    footer_sort,
    slug::Slug,
};

use super::{
    section::{QueryOrder, QueryScope, QuerySpec, Section, SectionContent},
    state::CompileState,
};

/// A single row of a resolved listing.
struct QueryHit {
    slug: Slug,
    title: String,
    page_title: String,
    taxon: String,
    date: Option<String>,
}

/// Replace every listing placeholder in every compiled section with rendered
/// HTML. Returns the slugs of sections that contained at least one listing, so
/// incremental builds know to rewrite them whenever anything else changes.
pub(super) fn resolve_all(state: &mut CompileState) -> HashSet<Slug> {
    let owners = collect_query_owners(state);
    if owners.is_empty() {
        return owners;
    }

    let rendered: HashMap<String, String> = collect_specs(state)
        .into_iter()
        .map(|spec| (spec_cache_key(&spec), render(state, &spec)))
        .collect();

    for section in state.compiled_mut().values_mut() {
        substitute(section, &rendered);
    }

    owners
}

/// Sections that own at least one listing, including listings that only appear
/// inside an embedded copy.
fn collect_query_owners(state: &CompileState) -> HashSet<Slug> {
    let mut owners = HashSet::new();
    for section in state.compiled().values() {
        visit_specs(section, &mut |spec| {
            owners.insert(spec.owner);
        });
    }
    owners
}

fn collect_specs(state: &CompileState) -> Vec<QuerySpec> {
    let mut seen = HashSet::new();
    let mut specs = Vec::new();
    for section in state.compiled().values() {
        visit_specs(section, &mut |spec| {
            if seen.insert(spec_cache_key(spec)) {
                specs.push(spec.clone());
            }
        });
    }
    specs
}

fn visit_specs(section: &Section, f: &mut impl FnMut(&QuerySpec)) {
    for content in &section.children {
        match content {
            SectionContent::Query(spec) => f(spec),
            SectionContent::Embed(child) => visit_specs(child, f),
            SectionContent::Plain(_) => {}
        }
    }
}

fn substitute(section: &mut Section, rendered: &HashMap<String, String>) {
    for content in &mut section.children {
        match content {
            SectionContent::Query(spec) => {
                let html = rendered.get(&spec_cache_key(spec)).cloned().unwrap_or_default();
                *content = SectionContent::Plain(html);
            }
            SectionContent::Embed(child) => substitute(child, rendered),
            SectionContent::Plain(_) => {}
        }
    }
}

/// Identical listings resolve to identical HTML, so they are rendered once.
fn spec_cache_key(spec: &QuerySpec) -> String {
    format!(
        "{}|{:?}|{:?}|{:?}|{:?}|{}|{:?}|{:?}|{:?}|{}",
        spec.owner,
        spec.scope,
        spec.taxon,
        spec.key,
        spec.value,
        spec.sort,
        spec.order,
        spec.limit,
        spec.title,
        spec.include_indexes,
    )
}

/// The sections a listing resolves to, in display order.
///
/// Separated from rendering so selection can be tested without a site
/// environment: everything interesting about a listing is decided here.
fn select(state: &CompileState, spec: &QuerySpec) -> Vec<QueryHit> {
    let mut hits: Vec<QueryHit> = candidates(state, spec)
        .into_iter()
        // A listing never includes the page it is written on.
        .filter(|&slug| slug != spec.owner)
        .filter(|&slug| matches_filters(state, slug, spec))
        .filter_map(|slug| hit(state, slug))
        .collect();

    sort_hits(&mut hits, spec);
    if let Some(limit) = spec.limit {
        hits.truncate(limit);
    }
    hits
}

fn render(state: &CompileState, spec: &QuerySpec) -> String {
    let hits = select(state, spec);
    crate::html_flake::html_query_block(spec.title.as_deref(), &hits_html(&hits))
}

fn candidates(state: &CompileState, spec: &QuerySpec) -> Vec<Slug> {
    let all = || state.compiled().keys().copied().collect::<Vec<Slug>>();

    match &spec.scope {
        QueryScope::All => all(),
        QueryScope::Children => all()
            .into_iter()
            .filter(|&slug| state.parent_of(slug) == spec.owner)
            .collect(),
        QueryScope::Descendants => all()
            .into_iter()
            .filter(|&slug| is_descendant_of(state, slug, spec.owner))
            .collect(),
        QueryScope::Siblings => {
            let parent = state.parent_of(spec.owner);
            all()
                .into_iter()
                .filter(|&slug| state.parent_of(slug) == parent)
                .collect()
        }
        QueryScope::Orphans => all()
            .into_iter()
            .filter(|&slug| is_orphan(state, slug))
            .collect(),
        QueryScope::Prefix(prefix) => {
            let prefix = prefix.trim_end_matches('/');
            all()
                .into_iter()
                .filter(|slug| {
                    let slug = slug.as_str();
                    slug.strip_prefix(prefix)
                        .is_some_and(|rest| rest.starts_with('/'))
                })
                .collect()
        }
    }
}

/// Walk up the parent chain looking for `ancestor`. The visited set guards
/// against a malformed chain looping forever.
fn is_descendant_of(state: &CompileState, slug: Slug, ancestor: Slug) -> bool {
    let mut current = slug;
    let mut visited = HashSet::new();
    while visited.insert(current) {
        let parent = state.parent_of(current);
        if parent == ancestor {
            return true;
        }
        if parent == current {
            return false;
        }
        current = parent;
    }
    false
}

/// A section nothing links to and nothing embeds: reachable only by URL or by a
/// directory listing, which in a note forest usually means it was written and
/// then lost track of.
///
/// Directory index pages are *not* exempt by default. An unlinked `notes/index`
/// is genuinely unreachable for a reader browsing from the root — being a parent
/// makes it reachable *from* its children, not *to* it — so surfacing it is the
/// point rather than noise. Only the root index is exempt here, because it is
/// the entry point and has nowhere to be linked from.
///
/// Callers that disagree pass `include-indexes: false`, which is applied as a
/// general filter in [`matches_filters`] rather than special-cased here.
fn is_orphan(state: &CompileState, slug: Slug) -> bool {
    if slug.as_str() == super::INDEX_SLUG {
        return false;
    }
    match state.callback().0.get(&slug) {
        Some(callback) => {
            let embedded = callback.parent.is_some() && !callback.is_parent_specified;
            callback.backlinks.is_empty() && !embedded
        }
        None => true,
    }
}

/// Whether `slug` names a directory index — the hub for the directory it sits
/// in, or the root index itself.
fn is_index_page(slug: Slug) -> bool {
    let slug = slug.as_str();
    slug == super::INDEX_SLUG
        || slug
            .strip_suffix(super::INDEX_SLUG)
            .is_some_and(|parent| parent.ends_with('/'))
}

fn matches_filters(state: &CompileState, slug: Slug, spec: &QuerySpec) -> bool {
    if !spec.include_indexes && is_index_page(slug) {
        return false;
    }

    let Some(section) = state.compiled().get(&slug) else {
        return false;
    };

    if let Some(taxon) = &spec.taxon {
        let actual = section
            .metadata
            .data_taxon()
            .map(String::as_str)
            .unwrap_or_default();
        if !actual.eq_ignore_ascii_case(taxon) {
            return false;
        }
    }

    if let Some(key) = &spec.key {
        match (section.metadata.get_str(key), &spec.value) {
            (None, _) => return false,
            (Some(actual), Some(expected)) if actual != expected => return false,
            _ => {}
        }
    }

    true
}

fn hit(state: &CompileState, slug: Slug) -> Option<QueryHit> {
    let section = state.compiled().get(&slug)?;
    let metadata = &section.metadata;
    Some(QueryHit {
        slug,
        title: metadata
            .get_str(KEY_TITLE)
            .cloned()
            .unwrap_or_else(|| slug.to_string()),
        page_title: metadata
            .page_title()
            .cloned()
            .unwrap_or_else(|| slug.to_string()),
        taxon: metadata.get_str(KEY_TAXON).cloned().unwrap_or_default(),
        date: metadata.get_str(KEY_DATE).cloned(),
    })
}

fn sort_hits(hits: &mut [QueryHit], spec: &QuerySpec) {
    let key = spec.sort.trim();
    hits.sort_by(|left, right| {
        let ordering = footer_sort::compare_values(key, &sort_value(left, key), &sort_value(right, key))
            // Slug breaks ties so output stays stable across builds.
            .then_with(|| left.slug.cmp(&right.slug));
        match spec.order {
            QueryOrder::Ascending => ordering,
            QueryOrder::Descending => ordering.reverse(),
        }
    });
}

fn sort_value(hit: &QueryHit, key: &str) -> String {
    match key {
        "slug" => hit.slug.to_string(),
        "title" => hit.page_title.clone(),
        "taxon" => hit.taxon.clone(),
        KEY_DATE => hit.date.clone().unwrap_or_default(),
        _ => String::new(),
    }
}

fn hits_html(hits: &[QueryHit]) -> String {
    let mut html = String::new();
    for hit in hits {
        html.push_str(&crate::html_flake::html_query_item(
            hit.slug,
            &hit.title,
            &hit.page_title,
            &hit.taxon,
            hit.date.as_deref(),
        ));
    }
    html
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        compiler::{
            section::{EmbedContent, HTMLContent, LazyContent, SectionOption, UnresolvedSection},
            state::compile_all_without_missing_index_warning,
        },
        entry::{HTMLMetaData, KEY_EXT, KEY_SLUG},
        ordered_map::OrderedMap,
    };

    fn note(slug: &str, title: &str, taxon: &str, date: &str) -> UnresolvedSection {
        let mut metadata = OrderedMap::new();
        metadata.insert(KEY_SLUG.to_string(), HTMLContent::Plain(slug.to_string()));
        metadata.insert(KEY_EXT.to_string(), HTMLContent::Plain("typ".to_string()));
        metadata.insert(KEY_TITLE.to_string(), HTMLContent::Plain(title.to_string()));
        metadata.insert(KEY_TAXON.to_string(), HTMLContent::Plain(taxon.to_string()));
        metadata.insert(
            crate::entry::KEY_DATA_TAXON.to_string(),
            HTMLContent::Plain(taxon.to_string()),
        );
        metadata.insert(KEY_DATE.to_string(), HTMLContent::Plain(date.to_string()));
        UnresolvedSection {
            metadata: HTMLMetaData(metadata),
            content: HTMLContent::Plain(String::new()),
        }
    }

    fn spec(owner: &str, scope: QueryScope) -> QuerySpec {
        QuerySpec {
            owner: Slug::new(owner),
            scope,
            taxon: None,
            key: None,
            value: None,
            sort: "slug".to_string(),
            order: QueryOrder::Ascending,
            limit: None,
            title: None,
            include_indexes: true,
        }
    }

    /// index, notes/index, notes/{alice,bob}, notes/deep/{index,carol}, stray
    fn forest() -> HashMap<Slug, UnresolvedSection> {
        let mut shallows = HashMap::new();
        for (slug, title, taxon, date) in [
            ("index", "Root", "collection", "2026-01-01"),
            ("notes/index", "Notes", "collection", "2026-01-02"),
            ("notes/alice", "Alice", "remark", "2026-03-01"),
            ("notes/bob", "Bob", "definition", "2026-05-02"),
            ("notes/deep/index", "Deep", "collection", "2026-01-03"),
            ("notes/deep/carol", "Carol", "definition", "2026-01-15"),
            ("stray", "Stray", "remark", "2026-02-20"),
        ] {
            shallows.insert(Slug::new(slug), note(slug, title, taxon, date));
        }
        shallows
    }

    fn slugs_of(state: &CompileState, spec: &QuerySpec) -> Vec<String> {
        select(state, spec)
            .into_iter()
            .map(|hit| hit.slug.to_string())
            .collect()
    }

    #[test]
    fn test_children_lists_direct_children_only_and_excludes_self() {
        let state = compile_all_without_missing_index_warning(&forest()).unwrap();
        assert_eq!(
            slugs_of(&state, &spec("notes/index", QueryScope::Children)),
            vec!["notes/alice", "notes/bob", "notes/deep/index"]
        );
    }

    #[test]
    fn test_descendants_is_transitive() {
        let state = compile_all_without_missing_index_warning(&forest()).unwrap();
        assert_eq!(
            slugs_of(&state, &spec("notes/index", QueryScope::Descendants)),
            vec![
                "notes/alice",
                "notes/bob",
                "notes/deep/carol",
                "notes/deep/index"
            ]
        );
    }

    #[test]
    fn test_siblings_share_a_parent_and_exclude_self() {
        let state = compile_all_without_missing_index_warning(&forest()).unwrap();
        assert_eq!(
            slugs_of(&state, &spec("notes/alice", QueryScope::Siblings)),
            vec!["notes/bob", "notes/deep/index"]
        );
    }

    #[test]
    fn test_prefix_scope_matches_on_path_boundaries() {
        let state = compile_all_without_missing_index_warning(&forest()).unwrap();
        assert_eq!(
            slugs_of(
                &state,
                &spec("index", QueryScope::Prefix("notes".to_string()))
            ),
            vec![
                "notes/alice",
                "notes/bob",
                "notes/deep/carol",
                "notes/deep/index",
                "notes/index"
            ]
        );
        // `note` must not match `notes/...`.
        assert!(slugs_of(
            &state,
            &spec("index", QueryScope::Prefix("note".to_string()))
        )
        .is_empty());
    }

    #[test]
    fn test_taxon_filter_is_case_insensitive() {
        let state = compile_all_without_missing_index_warning(&forest()).unwrap();
        let mut query = spec("index", QueryScope::All);
        query.taxon = Some("Definition".to_string());
        assert_eq!(
            slugs_of(&state, &query),
            vec!["notes/bob", "notes/deep/carol"]
        );
    }

    #[test]
    fn test_sort_by_date_descending_with_limit() {
        let state = compile_all_without_missing_index_warning(&forest()).unwrap();
        let mut query = spec("index", QueryScope::All);
        query.sort = "date".to_string();
        query.order = QueryOrder::Descending;
        query.limit = Some(3);
        assert_eq!(
            slugs_of(&state, &query),
            vec!["notes/bob", "notes/alice", "stray"]
        );
    }

    #[test]
    fn test_orphans_excludes_linked_embedded_and_root_sections() {
        let mut shallows = forest();
        // `index` embeds `stray`, so `stray` is reachable.
        shallows.insert(
            Slug::new("index"),
            UnresolvedSection {
                metadata: note("index", "Root", "collection", "2026-01-01").metadata,
                content: HTMLContent::Lazy(vec![LazyContent::Embed(EmbedContent {
                    url: "/stray".to_string(),
                    title: None,
                    option: SectionOption::default(),
                })]),
            },
        );
        // `notes/alice` links to `notes/bob`, so `notes/bob` is reachable.
        shallows.insert(
            Slug::new("notes/alice"),
            UnresolvedSection {
                metadata: note("notes/alice", "Alice", "remark", "2026-03-01").metadata,
                content: HTMLContent::Lazy(vec![LazyContent::Local(
                    crate::compiler::section::LocalLink {
                        url: "/notes/bob".to_string(),
                        text: None,
                    },
                )]),
            },
        );

        let state = compile_all_without_missing_index_warning(&shallows).unwrap();
        let found = slugs_of(&state, &spec("notes/deep/carol", QueryScope::Orphans));

        assert!(!found.iter().any(|s| s == "index"), "root is never orphaned");
        assert!(!found.iter().any(|s| s == "stray"), "embedded, so reachable");
        assert!(
            !found.iter().any(|s| s == "notes/bob"),
            "linked to, so reachable"
        );
        assert!(!found.iter().any(|s| s == "notes/deep/carol"), "is the owner");
        assert!(found.iter().any(|s| s == "notes/alice"));
    }

    #[test]
    fn test_is_index_page_matches_hubs_at_any_depth() {
        assert!(is_index_page(Slug::new("index")));
        assert!(is_index_page(Slug::new("notes/index")));
        assert!(is_index_page(Slug::new("notes/deep/index")));

        assert!(!is_index_page(Slug::new("notes/alice")));
        // A note that merely ends in the word must not be mistaken for a hub.
        assert!(!is_index_page(Slug::new("notes/reindex")));
        assert!(!is_index_page(Slug::new("indexing")));
    }

    #[test]
    fn test_include_indexes_defaults_to_listing_hubs() {
        let state = compile_all_without_missing_index_warning(&forest()).unwrap();
        let found = slugs_of(&state, &spec("stray", QueryScope::Orphans));

        assert!(
            found.iter().any(|s| s == "notes/index"),
            "unlinked hubs are orphans by default"
        );
    }

    #[test]
    fn test_include_indexes_false_drops_hubs_from_any_scope() {
        let state = compile_all_without_missing_index_warning(&forest()).unwrap();

        let mut orphans = spec("stray", QueryScope::Orphans);
        orphans.include_indexes = false;
        let found = slugs_of(&state, &orphans);
        assert!(!found.iter().any(|s| s.ends_with("index")), "got: {found:?}");
        assert!(found.iter().any(|s| s == "notes/alice"), "got: {found:?}");

        // The option is a general filter, not special-cased to orphans.
        let mut children = spec("notes/index", QueryScope::Children);
        children.include_indexes = false;
        assert_eq!(
            slugs_of(&state, &children),
            vec!["notes/alice", "notes/bob"],
            "notes/deep/index should be filtered out"
        );
    }

    #[test]
    fn test_query_scope_parses_named_scopes_and_prefixes() {
        assert_eq!(QueryScope::parse("children"), QueryScope::Children);
        assert_eq!(QueryScope::parse("descendants"), QueryScope::Descendants);
        assert_eq!(QueryScope::parse("siblings"), QueryScope::Siblings);
        assert_eq!(QueryScope::parse("all"), QueryScope::All);
        assert_eq!(QueryScope::parse("orphans"), QueryScope::Orphans);
        assert_eq!(
            QueryScope::parse("notes/"),
            QueryScope::Prefix("notes/".to_string())
        );
        // A leading slash is accepted for symmetry with link targets.
        assert_eq!(
            QueryScope::parse("/notes"),
            QueryScope::Prefix("notes".to_string())
        );
    }

    #[test]
    fn test_query_order_defaults_to_ascending() {
        assert_eq!(QueryOrder::parse("desc"), QueryOrder::Descending);
        assert_eq!(QueryOrder::parse("descending"), QueryOrder::Descending);
        assert_eq!(QueryOrder::parse("asc"), QueryOrder::Ascending);
        assert_eq!(QueryOrder::parse("nonsense"), QueryOrder::Ascending);
    }
}
