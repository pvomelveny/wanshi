// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Alias Qli (@AliasQli), Spore (@s-cerevisiae)

use std::collections::{HashMap, HashSet, VecDeque};

use camino::{Utf8Path, Utf8PathBuf};

use crate::{
    environment::verify_and_file_hash,
    slug::{Ext, Slug},
};

use super::{state, DirtySet, Workspace};

pub(super) fn source_relative_path(slug: Slug, ext: Ext) -> Utf8PathBuf {
    Utf8PathBuf::from(format!("{}.{}", slug, ext))
}

pub(super) fn is_source_modified(
    relative_path: &Utf8Path,
    dirty_paths: Option<&DirtySet>,
) -> eyre::Result<bool> {
    if *crate::cli::build::no_cache_enabled() {
        return Ok(true);
    }

    if let Some(dirty_paths) = dirty_paths {
        if dirty_paths.contains(relative_path) {
            // Keep hash baseline updated for subsequent cold builds.
            let _ = verify_and_file_hash(relative_path)?;
            return Ok(true);
        }
        return Ok(false);
    }

    verify_and_file_hash(relative_path)
}

pub fn expand_dirty_paths(workspace: &Workspace, dirty_paths: &DirtySet) -> DirtySet {
    let mut expanded = dirty_paths.clone();
    let source_paths: HashSet<Utf8PathBuf> = workspace
        .slug_exts
        .iter()
        .map(|(&slug, &ext)| source_relative_path(slug, ext))
        .collect();

    let dirty_all_sources = dirty_paths.iter().any(|path| !source_paths.contains(path));

    if dirty_all_sources {
        // A non-source dependency changed (e.g. a shared `.typ` asset, an imported
        // `.typst` library, or an include file). wanshi does not maintain a
        // dependency graph for arbitrary include relationships, so conservatively
        // reparse every known source.
        workspace.slug_exts.iter().for_each(|(&slug, &ext)| {
            expanded.insert(source_relative_path(slug, ext));
        });
    }

    expanded
}

pub(super) fn dirty_source_slugs(workspace: &Workspace, dirty_paths: &DirtySet) -> HashSet<Slug> {
    workspace
        .slug_exts
        .iter()
        .filter_map(|(&slug, &ext)| {
            let relative = source_relative_path(slug, ext);
            dirty_paths.contains(relative.as_path()).then_some(slug)
        })
        .collect()
}

pub(super) fn affected_slugs_from_dirty(
    state: &state::CompileState,
    dirty_source_slugs: &HashSet<Slug>,
) -> HashSet<Slug> {
    let mut affected = dirty_source_slugs.clone();

    // A listing renders other sections, so any change anywhere can alter it.
    // These pages are still hash-guarded on write, so rewriting them costs
    // nothing when the rendered result is unchanged.
    if !dirty_source_slugs.is_empty() {
        affected.extend(state.query_owners().iter().copied());
    }

    // Resolve every section's effective parent once. This has to cover all
    // compiled sections rather than only those with recorded callbacks: a
    // section that nothing embeds still inherits a directory index as its
    // parent, and its header navigation goes stale when that index changes.
    let effective_parents: HashMap<Slug, Slug> = state
        .compiled()
        .keys()
        .map(|&slug| (slug, state.parent_of(slug)))
        .collect();

    // Descendant sections (embedded children) share source ownership with their parent
    // in subtree mode, so source-dirty must include the whole descendant chain.
    let mut changed = true;
    while changed {
        changed = false;
        for (&slug, &parent) in &effective_parents {
            if parent != slug && affected.contains(&parent) && affected.insert(slug) {
                changed = true;
            }
        }
    }

    let mut queue: VecDeque<Slug> = affected.iter().copied().collect();

    // If a linker page changes, the target's backlink list changes too.
    for (&target_slug, callback) in &state.callback().0 {
        if callback
            .backlinks
            .iter()
            .any(|backlink_slug| dirty_source_slugs.contains(backlink_slug))
            && affected.insert(target_slug)
        {
            queue.push_back(target_slug);
        }
    }

    while let Some(slug) = queue.pop_front() {
        if let Some(&parent) = effective_parents.get(&slug) {
            if parent != slug && affected.insert(parent) {
                queue.push_back(parent);
            }
        }

        let Some(callback) = state.callback().0.get(&slug) else {
            continue;
        };

        for &backlink_slug in &callback.backlinks {
            if affected.insert(backlink_slug) {
                queue.push_back(backlink_slug);
            }
        }
    }

    affected
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;
    use crate::{
        compiler::section::{
            EmbedContent, HTMLContent, LazyContent, LocalLink, SectionOption, UnresolvedSection,
        },
        entry::{HTMLMetaData, KEY_EXT, KEY_PAGE_TITLE, KEY_SLUG, KEY_TITLE},
        ordered_map::OrderedMap,
    };

    fn shallow(slug: &str, content: HTMLContent) -> UnresolvedSection {
        let mut metadata = OrderedMap::new();
        metadata.insert(KEY_SLUG.to_string(), HTMLContent::Plain(slug.to_string()));
        metadata.insert(KEY_EXT.to_string(), HTMLContent::Plain("typst".to_string()));
        metadata.insert(KEY_TITLE.to_string(), HTMLContent::Plain(slug.to_string()));
        metadata.insert(
            KEY_PAGE_TITLE.to_string(),
            HTMLContent::Plain(slug.to_string()),
        );
        UnresolvedSection {
            metadata: HTMLMetaData(metadata),
            content,
        }
    }

    #[test]
    fn test_expand_dirty_paths_non_source_dependency_marks_all_sources() {
        let mut slug_exts = HashMap::new();
        slug_exts.insert(Slug::new("a"), Ext::Typst);
        slug_exts.insert(Slug::new("b"), Ext::Typst);
        slug_exts.insert(Slug::new("c"), Ext::Typst);
        let workspace = Workspace { slug_exts };

        let mut dirty = DirtySet::new();
        dirty.insert(Utf8PathBuf::from("shared.typ"));

        let expanded = expand_dirty_paths(&workspace, &dirty);
        assert!(expanded.contains(&Utf8PathBuf::from("shared.typ")));
        assert!(expanded.contains(&Utf8PathBuf::from("a.typst")));
        assert!(expanded.contains(&Utf8PathBuf::from("b.typst")));
        assert!(expanded.contains(&Utf8PathBuf::from("c.typst")));
    }

    #[test]
    fn test_expand_dirty_paths_source_change_keeps_scope_local() {
        let mut slug_exts = HashMap::new();
        slug_exts.insert(Slug::new("a"), Ext::Typst);
        slug_exts.insert(Slug::new("b"), Ext::Typst);
        slug_exts.insert(Slug::new("c"), Ext::Typst);
        let workspace = Workspace { slug_exts };

        let mut dirty = DirtySet::new();
        dirty.insert(Utf8PathBuf::from("b.typst"));

        let expanded = expand_dirty_paths(&workspace, &dirty);
        assert!(expanded.contains(&Utf8PathBuf::from("b.typst")));
        assert!(!expanded.contains(&Utf8PathBuf::from("a.typst")));
        assert!(!expanded.contains(&Utf8PathBuf::from("c.typst")));
    }

    #[test]
    fn test_expand_dirty_paths_unknown_tree_file_marks_all_sources() {
        let mut slug_exts = HashMap::new();
        slug_exts.insert(Slug::new("a"), Ext::Typst);
        slug_exts.insert(Slug::new("b"), Ext::Typst);
        let workspace = Workspace { slug_exts };

        let mut dirty = DirtySet::new();
        dirty.insert(Utf8PathBuf::from("includes/snippet.txt"));

        let expanded = expand_dirty_paths(&workspace, &dirty);
        assert!(expanded.contains(&Utf8PathBuf::from("a.typst")));
        assert!(expanded.contains(&Utf8PathBuf::from("b.typst")));
    }

    #[test]
    fn test_dirty_source_slugs_maps_relative_paths_to_slug_ids() {
        let mut slug_exts = HashMap::new();
        slug_exts.insert(Slug::new("a"), Ext::Typst);
        slug_exts.insert(Slug::new("b"), Ext::Typst);
        let workspace = Workspace { slug_exts };

        let mut dirty = DirtySet::new();
        dirty.insert(Utf8PathBuf::from("a.typst"));
        dirty.insert(Utf8PathBuf::from("unknown.txt"));

        let dirty_slugs = dirty_source_slugs(&workspace, &dirty);
        assert!(dirty_slugs.contains(&Slug::new("a")));
        assert!(!dirty_slugs.contains(&Slug::new("b")));
    }

    #[test]
    fn test_affected_slugs_include_link_targets_when_linker_changes() {
        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("a"),
            shallow(
                "a",
                HTMLContent::Lazy(vec![LazyContent::Local(LocalLink {
                    url: "/b.typst".to_string(),
                    text: None,
                })]),
            ),
        );
        shallows.insert(
            Slug::new("b"),
            shallow("b", HTMLContent::Plain("<p>b</p>".to_string())),
        );

        let state = state::compile_all(&shallows).unwrap();
        let dirty_slugs = HashSet::from([Slug::new("a")]);
        let affected = affected_slugs_from_dirty(&state, &dirty_slugs);

        assert!(affected.contains(&Slug::new("a")));
        assert!(affected.contains(&Slug::new("b")));
    }

    #[test]
    fn test_affected_slugs_include_embedded_descendants_when_parent_source_changes() {
        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("root"),
            shallow(
                "root",
                HTMLContent::Lazy(vec![LazyContent::Embed(EmbedContent {
                    url: "/child.typst".to_string(),
                    title: None,
                    option: SectionOption::default(),
                })]),
            ),
        );
        shallows.insert(
            Slug::new("child"),
            shallow(
                "child",
                HTMLContent::Lazy(vec![LazyContent::Embed(EmbedContent {
                    url: "/leaf.typst".to_string(),
                    title: None,
                    option: SectionOption::default(),
                })]),
            ),
        );
        shallows.insert(
            Slug::new("leaf"),
            shallow("leaf", HTMLContent::Plain("<p>leaf</p>".to_string())),
        );

        let state = state::compile_all(&shallows).unwrap();
        let dirty_slugs = HashSet::from([Slug::new("root")]);
        let affected = affected_slugs_from_dirty(&state, &dirty_slugs);

        assert!(affected.contains(&Slug::new("root")));
        assert!(affected.contains(&Slug::new("child")));
        assert!(affected.contains(&Slug::new("leaf")));
    }
}
