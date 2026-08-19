// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use eyre::eyre;
use std::collections::{BTreeSet, HashMap, HashSet};

use camino::Utf8Path;

use crate::{
    entry::{
        is_plain_metadata, EntryMetaData, HTMLMetaData, MetaData, KEY_EXT,
        KEY_INTERNAL_ANON_SUBTREE, KEY_SLUG, KEY_TITLE,
    },
    environment,
    ordered_map::OrderedMap,
    path_utils,
    slug::{self, Slug, INDEX_SLUG},
};

use super::{
    callback::{AmbiguousParent, Callback, CallbackValue},
    section::{
        HTMLContent, LazyContent, Section, SectionContent, SectionContents, UnresolvedSection,
    },
    taxon::Taxon,
};

#[derive(Debug)]
pub struct CompileState {
    residued: BTreeSet<Slug>,
    compiled: HashMap<Slug, Section>,
    callback: Callback,
    visiting: HashSet<Slug>,
    compile_stack: Vec<Slug>,

    /// Sections containing a `#query(...)` listing. Their output depends on
    /// other sections, so incremental builds must rewrite them whenever
    /// anything changes.
    query_owners: HashSet<Slug>,
}

type UnresolvedSections = HashMap<Slug, UnresolvedSection>;

pub fn compile_all(shallows: &UnresolvedSections) -> eyre::Result<CompileState> {
    compile_all_with_missing_index_warning(shallows, true)
}

pub fn compile_all_without_missing_index_warning(
    shallows: &UnresolvedSections,
) -> eyre::Result<CompileState> {
    compile_all_with_missing_index_warning(shallows, false)
}

fn compile_all_with_missing_index_warning(
    shallows: &UnresolvedSections,
    emit_missing_index_warning: bool,
) -> eyre::Result<CompileState> {
    let residued: BTreeSet<Slug> = shallows.keys().copied().collect();

    let mut state = CompileState::new(residued);
    let index = Slug::new(INDEX_SLUG);
    if emit_missing_index_warning && state.compile(shallows, index)?.is_none() {
        color_print::ceprintln!(
            "<y>Warning: Missing `{}` section, please provide `{}.{}`.</>",
            INDEX_SLUG,
            INDEX_SLUG,
            crate::slug::Ext::Typ
        );
    } else if !emit_missing_index_warning {
        let _ = state.compile(shallows, index)?;
    }

    /*
     * Unlinked or unembedded pages.
     */
    while let Some(slug) = state.residued.pop_first() {
        state.compile(shallows, slug)?;
    }

    state.normalize_internal_anonymous_graph();
    // Listings ask questions about the finished graph, so they can only be
    // resolved once every section has been compiled and normalized.
    state.query_owners = super::query::resolve_all(&mut state);
    Ok(state)
}

impl CompileState {
    fn new(residued: BTreeSet<Slug>) -> CompileState {
        CompileState {
            residued,
            compiled: HashMap::new(),
            callback: Callback::new(),
            visiting: HashSet::new(),
            compile_stack: Vec::new(),
            query_owners: HashSet::new(),
        }
    }

    fn compile(
        &mut self,
        shallows: &UnresolvedSections,
        slug: Slug,
    ) -> eyre::Result<Option<&Section>> {
        self.fetch_section(shallows, slug)
    }

    fn fetch_section(
        &mut self,
        shallows: &UnresolvedSections,
        slug: Slug,
    ) -> eyre::Result<Option<&Section>> {
        if self.compiled.contains_key(&slug) {
            return Ok(self.compiled.get(&slug));
        }

        if self.visiting.contains(&slug) {
            let mut chain: Vec<String> =
                self.compile_stack.iter().map(ToString::to_string).collect();
            chain.push(slug.to_string());
            return Err(eyre!("cyclic embed detected: {}", chain.join(" -> ")));
        }

        let Some(shallow) = shallows.get(&slug) else {
            return Ok(None);
        };
        self.visiting.insert(slug);
        self.compile_stack.push(slug);
        let result = self.compile_unresolved(shallows, shallow);
        self.compile_stack.pop();
        self.visiting.remove(&slug);
        result?;
        Ok(self.compiled.get(&slug))
    }

    fn compile_unresolved(
        &mut self,
        shallows: &UnresolvedSections,
        spanned: &UnresolvedSection,
    ) -> eyre::Result<()> {
        let slug = spanned.slug()?;
        let ext = spanned.ext()?;
        let mut children: SectionContents = vec![];
        let mut references: HashSet<Slug> = HashSet::new();

        match &spanned.content {
            HTMLContent::Plain(html) => {
                children.push(SectionContent::Plain(html.to_string()));
            }
            HTMLContent::Lazy(lazy_contents) => {
                let mut callback: Callback = Callback::new();

                for lazy_content in lazy_contents {
                    match lazy_content {
                        LazyContent::Plain(html) => {
                            children.push(SectionContent::Plain(html.to_string()));
                        }
                        LazyContent::Embed(embed_content) => {
                            let child_slug = subsection_slug(slug, &embed_content.url);

                            let refered = match self.fetch_section(shallows, child_slug)? {
                                Some(refered_section) => refered_section,
                                None => {
                                    return Err(eyre!(
                                        "[{}] attempting to fetch a non-existent [{}]",
                                        slug,
                                        child_slug
                                    ));
                                }
                            };

                            if embed_content.option.details_open {
                                references.extend(refered.references.clone());
                            }
                            callback.insert_parent(child_slug, slug);

                            // The durable record that this embed happened.
                            // `insert_parent` alone is not enough: `parent`
                            // holds one slug, and a parent the child declares
                            // for itself displaces the embedder, so a note that
                            // is embedded *and* names its own parent would keep
                            // no trace of being embedded at all.
                            //
                            // `asback` is deliberately not consulted. It is
                            // documented as governing a page's *links*, and a
                            // hub that embeds notes is usually exactly the page
                            // you want listed under "Found in".
                            if child_slug != slug && backlinks_enabled(shallows, child_slug)? {
                                callback.insert_embedded_by(child_slug, slug);
                            }

                            let mut child_section = refered.clone();
                            child_section.option = embed_content.option.clone();
                            if let Some(title) = &embed_content.title {
                                child_section
                                    .metadata
                                    .update(KEY_TITLE.to_owned(), title.to_string())
                            };
                            children.push(SectionContent::Embed(child_section));
                        }
                        LazyContent::Query(spec) => {
                            children.push(SectionContent::Query(spec.clone()));
                        }
                        // `split_heading_sections` consumes these while the note
                        // is still being parsed. Reaching one here would mean a
                        // section skipped that pass; drop it rather than panic,
                        // so a cache written before it existed cannot crash a
                        // build.
                        LazyContent::Outdent => {}
                        LazyContent::Local(local_link) => {
                            let link_slug = subsection_slug(slug, &local_link.url);

                            let metadata = get_metadata(shallows, link_slug);
                            let article_title_html = metadata
                                .and_then(|s| s.title())
                                .map_or_else(String::new, html_content_to_html_string);
                            let article_title_plain = metadata
                                .and_then(|s| s.title())
                                .map_or_else(String::new, HTMLContent::remove_all_tags);
                            let page_title_plain = metadata
                                .and_then(|s| s.page_title())
                                .map(|s| strip_html_tags(s))
                                .unwrap_or_else(|| article_title_plain.clone());

                            if link_slug != slug && is_reference(shallows, link_slug)? {
                                references.insert(link_slug);
                            }

                            /*
                             * Making oneself the content of a backlink should not be expected behavior.
                             */
                            if link_slug != slug
                                && backlinks_enabled(shallows, link_slug)?
                                && is_backlink(shallows, slug)?
                            {
                                callback.insert_backlinks(link_slug, vec![slug]);
                            }

                            let text = local_link.text.clone().unwrap_or(article_title_html);

                            let html = crate::html_flake::html_link(
                                &environment::full_html_url(link_slug),
                                &format!("{} [{}]", page_title_plain, link_slug),
                                &text,
                                crate::recorder::State::LocalLink.strify(),
                            );
                            children.push(SectionContent::Plain(html.to_string()));
                        }
                    }
                }

                self.callback.merge(callback);
            }
        };

        if let Some(parent) = spanned.metadata.parent() {
            self.callback.specify_parent(slug, parent);
        }

        // compile metadata
        let mut metadata = EntryMetaData(OrderedMap::new());
        for key in spanned.metadata.keys() {
            let Some(value) = spanned.metadata.get(key) else {
                return Err(eyre!(
                    "metadata key `{}` vanished while compiling `{}`",
                    key,
                    slug
                ));
            };
            if is_plain_metadata(key) {
                if let Some(val) = value.as_string() {
                    metadata.update(key.to_string(), val.to_owned());
                } else {
                    return Err(eyre!(
                        "metadata field `{}` in `{}` is expected to be plain text",
                        key,
                        slug
                    ));
                }
            } else {
                let spanned: UnresolvedSection = Self::metadata_to_section(value, slug, ext);
                self.compile_unresolved(shallows, &spanned)?;
                let compiled = self.compiled.get(&slug).ok_or_else(|| {
                    eyre!(
                        "compiled section `{}` disappeared while compiling metadata",
                        slug
                    )
                })?;
                let html = compiled.spanned();
                metadata.update(key.to_string(), html);
            };
        }

        // remove from `self.residued` after compiled.
        self.residued.remove(&slug);

        let section = Section::new(metadata, children, references);
        self.compiled.insert(slug, section);
        Ok(())
    }

    fn metadata_to_section(
        content: &HTMLContent,
        current_slug: Slug,
        current_ext: &str,
    ) -> UnresolvedSection {
        let mut metadata = OrderedMap::new();
        metadata.insert(
            KEY_SLUG.to_string(),
            HTMLContent::Plain(current_slug.to_string()),
        );
        metadata.insert(
            KEY_EXT.to_string(),
            HTMLContent::Plain(current_ext.to_string()),
        );

        UnresolvedSection {
            metadata: HTMLMetaData(metadata),
            content: content.clone(),
        }
    }

    pub fn compiled(&self) -> &HashMap<Slug, Section> {
        &self.compiled
    }

    pub(super) fn compiled_mut(&mut self) -> &mut HashMap<Slug, Section> {
        &mut self.compiled
    }

    /// Sections whose rendered output depends on the rest of the graph.
    pub fn query_owners(&self) -> &HashSet<Slug> {
        &self.query_owners
    }

    pub fn callback(&self) -> &Callback {
        &self.callback
    }

    fn normalize_internal_anonymous_graph(&mut self) {
        let internal_slugs = self.collect_internal_anonymous_slugs();
        if internal_slugs.is_empty() {
            self.report_ambiguous_parents(&HashMap::new());
            return;
        }

        for section in self.compiled.values_mut() {
            section
                .references
                .retain(|reference| !internal_slugs.contains(reference));
        }

        let normalized_parents: HashMap<Slug, Option<Slug>> = self
            .callback
            .0
            .iter()
            .map(|(&slug, value)| {
                (
                    slug,
                    Self::resolve_visible_parent(value.parent, &self.callback.0, &internal_slugs),
                )
            })
            .collect();

        // Where each internal section's edges should be re-attributed to. An
        // edge recorded against a section that is about to be deleted has to be
        // lifted to the nearest section that survives, not dropped: the note a
        // reader sees is the one that did the linking, and to them the link was
        // written by the page. Dropping instead would mean a `#local` after a
        // heading silently produced no backlink at all.
        let visible_hosts: HashMap<Slug, Option<Slug>> = internal_slugs
            .iter()
            .map(|&slug| {
                (
                    slug,
                    Self::resolve_visible_parent(Some(slug), &self.callback.0, &internal_slugs),
                )
            })
            .collect();

        for (&slug, value) in &mut self.callback.0 {
            value.backlinks = lift_edges(&value.backlinks, slug, &visible_hosts);
            value.embedded_by = lift_edges(&value.embedded_by, slug, &visible_hosts);
            if let Some(&parent) = normalized_parents.get(&slug) {
                value.parent = parent;
            }
        }

        self.report_ambiguous_parents(&visible_hosts);

        self.callback
            .0
            .retain(|slug, _| !internal_slugs.contains(slug));
        self.compiled
            .retain(|slug, _| !internal_slugs.contains(slug));
    }

    /// Report sections embedded from two places, now that the hosts can be
    /// named in slugs the author wrote.
    ///
    /// Two conditions are checked here rather than where the conflict was
    /// noticed, and both need the normalized graph. A host that is not
    /// published resolves to the section that is, so the warning names
    /// something addressable — `"parent": "index/:options"` is not advice
    /// anyone can take. And once both hosts have resolved they are often the
    /// *same* section: a note embedded under two headings of one page has one
    /// parent and no ambiguity, and warning about it would be noise on every
    /// build.
    fn report_ambiguous_parents(&mut self, visible_hosts: &HashMap<Slug, Option<Slug>>) {
        let ambiguous = self.callback.take_ambiguous_parents();
        for (child, kept, discarded) in reportable_ambiguities(ambiguous, visible_hosts) {
            color_print::ceprintln!(
                "<y>Warning: `{}` is embedded in both `{}` and `{}`; using `{}` as its parent.\n         Set `\"parent\"` in its metadata to choose deliberately.</>",
                child,
                kept,
                discarded,
                kept
            );
        }
    }

    fn collect_internal_anonymous_slugs(&self) -> HashSet<Slug> {
        self.compiled
            .iter()
            .filter_map(|(&slug, section)| {
                section
                    .metadata
                    .get_str(KEY_INTERNAL_ANON_SUBTREE)
                    .is_some_and(|value| value == "true")
                    .then_some(slug)
            })
            .collect()
    }

    /// Walk up out of anonymous subtrees to the nearest parent that will still
    /// exist in the published graph. `None` means "no usable parent was
    /// recorded", leaving the fallback to [`CompileState::parent_of`].
    fn resolve_visible_parent(
        parent: Option<Slug>,
        callbacks: &HashMap<Slug, CallbackValue>,
        internal_slugs: &HashSet<Slug>,
    ) -> Option<Slug> {
        let mut parent = parent?;
        let mut visited = HashSet::new();
        while internal_slugs.contains(&parent) {
            if !visited.insert(parent) {
                color_print::ceprintln!(
                    "<y>Warning: cyclic internal parent chain detected at `{}`; falling back to the default parent.</>",
                    parent
                );
                return None;
            }
            parent = callbacks.get(&parent).and_then(|value| value.parent)?;
        }
        Some(parent)
    }

    /// The effective parent of a section, in precedence order: the parent it
    /// declared, the parent inferred from whatever embedded it, and otherwise
    /// the nearest enclosing directory index.
    pub fn parent_of(&self, slug: Slug) -> Slug {
        self.callback
            .0
            .get(&slug)
            .and_then(|value| value.parent)
            .unwrap_or_else(|| {
                nearest_directory_index(slug, |candidate| self.compiled.contains_key(&candidate))
            })
    }
}

/// The directory index that owns `slug`: `notes/deep/alice` prefers
/// `notes/deep/index`, then `notes/index`, then the root `index`.
///
/// A directory index never adopts itself, so `notes/index` resolves upward to
/// the root rather than becoming its own parent.
pub(super) fn nearest_directory_index(slug: Slug, exists: impl Fn(Slug) -> bool) -> Slug {
    let root = Slug::new(INDEX_SLUG);
    let mut directory = Utf8Path::new(slug.as_str()).parent();

    while let Some(current) = directory {
        let candidate = if current.as_str().is_empty() {
            root
        } else {
            Slug::new(format!("{current}/{INDEX_SLUG}"))
        };

        if candidate != slug && exists(candidate) {
            return candidate;
        }
        if current.as_str().is_empty() {
            break;
        }
        directory = current.parent();
    }

    root
}

/// Which recorded ambiguities are worth telling the author about, expressed in
/// the slugs they wrote.
///
/// Each host resolves through `visible_hosts` the way an edge does. Two that
/// resolve to the same section were never ambiguous — a note embedded under two
/// headings of one page has exactly one parent — and one that resolves to
/// nothing has no host to name.
fn reportable_ambiguities(
    ambiguous: Vec<AmbiguousParent>,
    visible_hosts: &HashMap<Slug, Option<Slug>>,
) -> Vec<(Slug, Slug, Slug)> {
    let visible = |slug: Slug| match visible_hosts.get(&slug) {
        Some(host) => *host,
        None => Some(slug),
    };

    ambiguous
        .into_iter()
        .filter_map(|entry| {
            let kept = visible(entry.kept)?;
            let discarded = visible(entry.discarded)?;
            (kept != discarded).then_some((entry.child, kept, discarded))
        })
        .collect()
}

/// Re-attribute edges away from sections that are about to be deleted.
///
/// An edge naming an internal section becomes one naming the nearest section
/// that survives publication. `visible_hosts` holds that mapping for every
/// internal slug; anything absent from it is already visible and passes through.
///
/// Two things are dropped rather than lifted: an internal section with no
/// visible host at all, and an edge that after lifting would point `owner` at
/// itself — which is what a note linking to its own page from inside one of its
/// headings would otherwise produce.
fn lift_edges(
    edges: &HashSet<Slug>,
    owner: Slug,
    visible_hosts: &HashMap<Slug, Option<Slug>>,
) -> HashSet<Slug> {
    edges
        .iter()
        .filter_map(|edge| match visible_hosts.get(edge) {
            Some(host) => *host,
            None => Some(*edge),
        })
        .filter(|edge| *edge != owner)
        .collect()
}

/// Calculate the slug of a subsection referenced by the current file, from the `url` referencing
/// it. If the url starts with `/`, the slug is considered absolute starting from the base of the
/// tree. Otherwise it's attached to the directory containing the current file.
fn subsection_slug(current_slug: Slug, url: &str) -> Slug {
    slug::to_slug(path_utils::relative_to_current(current_slug.as_str(), url))
}

fn get_metadata(shallows: &UnresolvedSections, slug: Slug) -> Option<&HTMLMetaData> {
    shallows.get(&slug).map(|s| &s.metadata)
}

fn html_content_to_html_string(content: &HTMLContent) -> String {
    content
        .as_string()
        .cloned()
        .unwrap_or_else(|| content.remove_all_tags())
}

fn strip_html_tags(text: &str) -> String {
    HTMLContent::Plain(text.to_string()).remove_all_tags()
}

fn backlinks_enabled(shallows: &UnresolvedSections, slug: Slug) -> eyre::Result<bool> {
    match shallows.get(&slug) {
        Some(section) => section.metadata.backlinks_enabled(),
        None => Ok(true),
    }
}

fn is_reference(shallows: &UnresolvedSections, slug: Slug) -> eyre::Result<bool> {
    match shallows.get(&slug) {
        Some(section) => {
            let metadata = &section.metadata;
            Ok(metadata.is_asref()?.unwrap_or(environment::asref())
                || Taxon::is_reference(metadata.data_taxon().map_or("", String::as_str)))
        }
        None => Ok(false),
    }
}

fn is_backlink(shallows: &UnresolvedSections, slug: Slug) -> eyre::Result<bool> {
    match shallows.get(&slug) {
        Some(section) => {
            let metadata = &section.metadata;
            Ok(metadata.is_asback()?.unwrap_or(true))
        }
        None => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::super::section::{EmbedContent, LocalLink, SectionOption};
    use super::*;
    use crate::{
        entry::{KEY_ASREF, KEY_INTERNAL_ANON_SUBTREE},
        ordered_map::OrderedMap,
    };

    fn shallow(slug: &str) -> UnresolvedSection {
        shallow_with_content(slug, HTMLContent::Plain(String::new()))
    }

    fn shallow_with_content(slug: &str, content: HTMLContent) -> UnresolvedSection {
        let mut metadata = OrderedMap::new();
        metadata.insert(KEY_SLUG.to_string(), HTMLContent::Plain(slug.to_string()));
        metadata.insert(KEY_EXT.to_string(), HTMLContent::Plain("typst".to_string()));

        UnresolvedSection {
            metadata: HTMLMetaData(metadata),
            content,
        }
    }

    #[test]
    fn test_subsection_slug() {
        assert_eq!(subsection_slug(Slug::new("a/b"), "c/d.typst"), "a/c/d");
        assert_eq!(subsection_slug(Slug::new("a/b"), "./c/d.typst"), "a/c/d");
        assert_eq!(subsection_slug(Slug::new("index"), "./a.b"), "a.b");

        assert_eq!(subsection_slug(Slug::new("a/b"), "/c/d.typst"), "c/d");
    }

    fn embed(url: &str) -> LazyContent {
        LazyContent::Embed(EmbedContent {
            url: url.to_string(),
            title: None,
            option: SectionOption::default(),
        })
    }

    #[test]
    fn test_embedding_records_the_host_on_the_child() {
        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("host"),
            shallow_with_content("host", HTMLContent::Lazy(vec![embed("/child.typst")])),
        );
        shallows.insert(Slug::new("child"), shallow("child"));

        let state = compile_all(&shallows).unwrap();
        let child = state.callback().0.get(&Slug::new("child")).unwrap();

        assert!(child.embedded_by.contains(&Slug::new("host")));
        // Containment is not citation; the two edges stay separate.
        assert!(child.backlinks.is_empty());
    }

    #[test]
    fn test_embedded_by_records_only_the_direct_host() {
        // `outer` embeds `middle` embeds `inner`. `inner` is contained by
        // `middle`, and only transitively by `outer`.
        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("outer"),
            shallow_with_content("outer", HTMLContent::Lazy(vec![embed("/middle.typst")])),
        );
        shallows.insert(
            Slug::new("middle"),
            shallow_with_content("middle", HTMLContent::Lazy(vec![embed("/inner.typst")])),
        );
        shallows.insert(Slug::new("inner"), shallow("inner"));

        let state = compile_all(&shallows).unwrap();
        let inner = state.callback().0.get(&Slug::new("inner")).unwrap();

        assert!(inner.embedded_by.contains(&Slug::new("middle")));
        assert!(
            !inner.embedded_by.contains(&Slug::new("outer")),
            "one level down only"
        );
    }

    #[test]
    fn test_embedded_by_records_every_host() {
        // The case `parent` cannot represent: it keeps one slug and warns.
        let mut shallows = HashMap::new();
        for host in ["alpha", "beta"] {
            shallows.insert(
                Slug::new(host),
                shallow_with_content(host, HTMLContent::Lazy(vec![embed("/shared.typst")])),
            );
        }
        shallows.insert(Slug::new("shared"), shallow("shared"));

        let state = compile_all(&shallows).unwrap();
        let shared = state.callback().0.get(&Slug::new("shared")).unwrap();

        assert_eq!(shared.embedded_by.len(), 2);
        assert!(shared.embedded_by.contains(&Slug::new("alpha")));
        assert!(shared.embedded_by.contains(&Slug::new("beta")));
    }

    #[test]
    fn test_compile_all_returns_error_for_cyclic_embed() {
        let embed_to_b = LazyContent::Embed(EmbedContent {
            url: "/b.typst".to_string(),
            title: None,
            option: SectionOption::default(),
        });
        let embed_to_a = LazyContent::Embed(EmbedContent {
            url: "/a.typst".to_string(),
            title: None,
            option: SectionOption::default(),
        });

        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("a"),
            shallow_with_content("a", HTMLContent::Lazy(vec![embed_to_b])),
        );
        shallows.insert(
            Slug::new("b"),
            shallow_with_content("b", HTMLContent::Lazy(vec![embed_to_a])),
        );

        let err = compile_all(&shallows).unwrap_err();
        assert!(err.to_string().contains("cyclic embed"));
    }

    #[test]
    fn test_local_link_title_attribute_uses_plain_text() {
        let mut shallows = HashMap::new();

        shallows.insert(
            Slug::new("index"),
            shallow_with_content(
                "index",
                HTMLContent::Lazy(vec![LazyContent::Local(LocalLink {
                    url: "/target".to_string(),
                    text: None,
                })]),
            ),
        );

        let mut target_metadata = OrderedMap::new();
        target_metadata.insert(
            KEY_SLUG.to_string(),
            HTMLContent::Plain("target".to_string()),
        );
        target_metadata.insert(KEY_EXT.to_string(), HTMLContent::Plain("typst".to_string()));
        target_metadata.insert(
            KEY_TITLE.to_string(),
            HTMLContent::Plain(r#"<span lang="zh">abc</span>"#.to_string()),
        );
        target_metadata.insert(
            crate::entry::KEY_PAGE_TITLE.to_string(),
            HTMLContent::Plain(r#"<span lang="zh">abc</span>"#.to_string()),
        );
        shallows.insert(
            Slug::new("target"),
            UnresolvedSection {
                metadata: HTMLMetaData(target_metadata),
                content: HTMLContent::Plain(String::new()),
            },
        );

        let state = compile_all_without_missing_index_warning(&shallows).unwrap();
        let html = state
            .compiled()
            .get(&Slug::new("index"))
            .and_then(|section| section.children.first())
            .and_then(|child| match child {
                SectionContent::Plain(html) => Some(html.as_str()),
                _ => None,
            })
            .expect("compiled index html");

        assert!(html.contains(r#"title="abc [target]""#));
        assert!(!html.contains("&lt;span"));
        assert!(html.contains(r#"><span lang="zh">abc</span></a>"#));
    }

    #[test]
    fn test_metadata_local_link_without_text_uses_target_title() {
        let mut shallows = HashMap::new();

        let mut index_metadata = OrderedMap::new();
        index_metadata.insert(
            KEY_SLUG.to_string(),
            HTMLContent::Plain("index".to_string()),
        );
        index_metadata.insert(KEY_EXT.to_string(), HTMLContent::Plain("typst".to_string()));
        index_metadata.insert(
            "author".to_string(),
            HTMLContent::Lazy(vec![LazyContent::Local(LocalLink {
                url: "/kokic".to_string(),
                text: None,
            })]),
        );
        shallows.insert(
            Slug::new("index"),
            UnresolvedSection {
                metadata: HTMLMetaData(index_metadata),
                content: HTMLContent::Plain(String::new()),
            },
        );

        let mut target_metadata = OrderedMap::new();
        target_metadata.insert(
            KEY_SLUG.to_string(),
            HTMLContent::Plain("kokic".to_string()),
        );
        target_metadata.insert(KEY_EXT.to_string(), HTMLContent::Plain("typst".to_string()));
        target_metadata.insert(
            KEY_TITLE.to_string(),
            HTMLContent::Plain("Kokic".to_string()),
        );
        shallows.insert(
            Slug::new("kokic"),
            UnresolvedSection {
                metadata: HTMLMetaData(target_metadata),
                content: HTMLContent::Plain(String::new()),
            },
        );

        let state = compile_all_without_missing_index_warning(&shallows).unwrap();
        let author = state
            .compiled()
            .get(&Slug::new("index"))
            .and_then(|section| section.metadata.get_str("author"))
            .expect("compiled author metadata");

        assert!(author.contains(r#"class="link local""#));
        assert!(author.contains(">Kokic</a>"));
    }

    #[test]
    fn test_compile_filters_internal_anonymous_sections_from_compiled_graph() {
        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("index"),
            shallow_with_content(
                "index",
                HTMLContent::Lazy(vec![LazyContent::Local(LocalLink {
                    url: "/anon".to_string(),
                    text: None,
                })]),
            ),
        );

        let mut anon = shallow_with_content("anon", HTMLContent::Plain("<p>anon</p>".to_string()));
        anon.metadata.0.insert(
            KEY_INTERNAL_ANON_SUBTREE.to_string(),
            HTMLContent::Plain("true".to_string()),
        );
        anon.metadata.0.insert(
            KEY_ASREF.to_string(),
            HTMLContent::Plain("true".to_string()),
        );
        shallows.insert(Slug::new("anon"), anon);

        let state = compile_all_without_missing_index_warning(&shallows).unwrap();
        let index = state.compiled().get(&Slug::new("index")).unwrap();
        assert!(!index.references.contains(&Slug::new("anon")));
        assert!(!state.compiled().contains_key(&Slug::new("anon")));
        assert!(!state.callback().0.contains_key(&Slug::new("anon")));
    }

    /// A link written inside a section that is not published still happened, and
    /// to a reader it was written by the page. It is re-attributed there rather
    /// than discarded — which is what used to happen, so a `#local` inside an
    /// anonymous subtree produced no backlink at all.
    #[test]
    fn test_compile_lifts_internal_anonymous_backlinks_to_the_visible_host() {
        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("index"),
            shallow_with_content(
                "index",
                HTMLContent::Lazy(vec![LazyContent::Embed(EmbedContent {
                    url: "/anon".to_string(),
                    title: None,
                    option: SectionOption::default(),
                })]),
            ),
        );

        let mut anon = shallow_with_content(
            "anon",
            HTMLContent::Lazy(vec![LazyContent::Local(LocalLink {
                url: "/target".to_string(),
                text: None,
            })]),
        );
        anon.metadata.0.insert(
            KEY_INTERNAL_ANON_SUBTREE.to_string(),
            HTMLContent::Plain("true".to_string()),
        );
        shallows.insert(Slug::new("anon"), anon);
        shallows.insert(
            Slug::new("target"),
            shallow_with_content("target", HTMLContent::Plain("<p>target</p>".to_string())),
        );

        let state = compile_all_without_missing_index_warning(&shallows).unwrap();
        let backlinks = &state
            .callback()
            .0
            .get(&Slug::new("target"))
            .expect("target should have a callback entry")
            .backlinks;
        assert_eq!(
            backlinks,
            &HashSet::from([Slug::new("index")]),
            "the backlink should name the page, not the deleted anonymous section"
        );
    }

    /// The same lifting for the embed edge. Without it every note embedded after
    /// a heading loses its "Found in" entry, because the section that embedded
    /// it is one the reader never sees.
    #[test]
    fn test_compile_lifts_internal_anonymous_embedded_by_to_the_visible_host() {
        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("index"),
            shallow_with_content(
                "index",
                HTMLContent::Lazy(vec![LazyContent::Embed(EmbedContent {
                    url: "/anon".to_string(),
                    title: None,
                    option: SectionOption::default(),
                })]),
            ),
        );

        let mut anon = shallow_with_content(
            "anon",
            HTMLContent::Lazy(vec![LazyContent::Embed(EmbedContent {
                url: "/target".to_string(),
                title: None,
                option: SectionOption::default(),
            })]),
        );
        anon.metadata.0.insert(
            KEY_INTERNAL_ANON_SUBTREE.to_string(),
            HTMLContent::Plain("true".to_string()),
        );
        shallows.insert(Slug::new("anon"), anon);
        shallows.insert(
            Slug::new("target"),
            shallow_with_content("target", HTMLContent::Plain("<p>target</p>".to_string())),
        );

        let state = compile_all_without_missing_index_warning(&shallows).unwrap();
        let embedded_by = &state
            .callback()
            .0
            .get(&Slug::new("target"))
            .expect("target should have a callback entry")
            .embedded_by;
        assert_eq!(embedded_by, &HashSet::from([Slug::new("index")]));
    }

    /// Lifting can point an edge at the page it started from — a note linking to
    /// itself from inside one of its own headings. A page is not its own
    /// backlink, so that one is dropped rather than lifted.
    #[test]
    fn test_compile_drops_lifted_edges_that_point_at_their_own_page() {
        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("index"),
            shallow_with_content(
                "index",
                HTMLContent::Lazy(vec![LazyContent::Embed(EmbedContent {
                    url: "/anon".to_string(),
                    title: None,
                    option: SectionOption::default(),
                })]),
            ),
        );

        let mut anon = shallow_with_content(
            "anon",
            HTMLContent::Lazy(vec![LazyContent::Local(LocalLink {
                url: "/index".to_string(),
                text: None,
            })]),
        );
        anon.metadata.0.insert(
            KEY_INTERNAL_ANON_SUBTREE.to_string(),
            HTMLContent::Plain("true".to_string()),
        );
        shallows.insert(Slug::new("anon"), anon);

        let state = compile_all_without_missing_index_warning(&shallows).unwrap();
        let backlinks = state
            .callback()
            .0
            .get(&Slug::new("index"))
            .map(|value| value.backlinks.clone())
            .unwrap_or_default();
        assert!(
            backlinks.is_empty(),
            "index should not backlink to itself, got {:?}",
            backlinks
        );
    }

    fn ambiguity(child: &str, kept: &str, discarded: &str) -> AmbiguousParent {
        AmbiguousParent {
            child: Slug::new(child),
            kept: Slug::new(kept),
            discarded: Slug::new(discarded),
        }
    }

    /// Two headings on one page are one parent. Reporting the raw hosts would
    /// warn on every build about an ambiguity the reader cannot see and the
    /// author cannot resolve — and would advise setting `parent` to a slug that
    /// is not published.
    #[test]
    fn test_two_hosts_on_the_same_page_are_not_an_ambiguity() {
        let visible_hosts = HashMap::from([
            (Slug::new("page/:one"), Some(Slug::new("page"))),
            (Slug::new("page/:two"), Some(Slug::new("page"))),
        ]);
        let reported = reportable_ambiguities(
            vec![ambiguity("shared", "page/:one", "page/:two")],
            &visible_hosts,
        );
        assert!(reported.is_empty(), "got {reported:?}");
    }

    /// A real ambiguity is still reported, in the slugs the author wrote rather
    /// than the synthesised ones the headings carry.
    #[test]
    fn test_a_real_ambiguity_is_reported_with_visible_slugs() {
        let visible_hosts = HashMap::from([
            (Slug::new("a/:background"), Some(Slug::new("a"))),
            (Slug::new("b/:background"), Some(Slug::new("b"))),
        ]);
        let reported = reportable_ambiguities(
            vec![ambiguity("shared", "a/:background", "b/:background")],
            &visible_hosts,
        );
        assert_eq!(
            reported,
            vec![(Slug::new("shared"), Slug::new("a"), Slug::new("b"))]
        );
    }

    /// A host that resolves to nothing cannot be named, so there is no advice
    /// to give.
    #[test]
    fn test_an_unresolvable_host_is_not_reported() {
        let visible_hosts = HashMap::from([(Slug::new("gone/:one"), None)]);
        let reported =
            reportable_ambiguities(vec![ambiguity("shared", "gone/:one", "b")], &visible_hosts);
        assert!(reported.is_empty(), "got {reported:?}");
    }

    /// Nothing is dropped when no heading is involved: hosts absent from the
    /// map are already visible and pass straight through.
    #[test]
    fn test_hosts_that_are_already_visible_pass_through() {
        let reported = reportable_ambiguities(vec![ambiguity("shared", "a", "b")], &HashMap::new());
        assert_eq!(
            reported,
            vec![(Slug::new("shared"), Slug::new("a"), Slug::new("b"))]
        );
    }

    #[test]
    fn test_compile_collapses_internal_parent_chain_to_visible_parent() {
        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("index"),
            shallow_with_content(
                "index",
                HTMLContent::Lazy(vec![LazyContent::Embed(EmbedContent {
                    url: "/anon".to_string(),
                    title: None,
                    option: SectionOption::default(),
                })]),
            ),
        );
        let mut anon = shallow_with_content(
            "anon",
            HTMLContent::Lazy(vec![LazyContent::Embed(EmbedContent {
                url: "/child".to_string(),
                title: None,
                option: SectionOption::default(),
            })]),
        );
        anon.metadata.0.insert(
            KEY_INTERNAL_ANON_SUBTREE.to_string(),
            HTMLContent::Plain("true".to_string()),
        );
        shallows.insert(Slug::new("anon"), anon);
        shallows.insert(
            Slug::new("child"),
            shallow_with_content("child", HTMLContent::Plain("<p>child</p>".to_string())),
        );

        let state = compile_all_without_missing_index_warning(&shallows).unwrap();
        let child_callback = state
            .callback()
            .0
            .get(&Slug::new("child"))
            .expect("child callback");
        // The chain collapses to a real recorded parent, not to the fallback:
        // `child` was genuinely embedded under `index` via the anonymous wrapper.
        assert_eq!(child_callback.parent, Some(Slug::new("index")));
        assert_eq!(state.parent_of(Slug::new("child")), Slug::new("index"));
        assert!(!state.callback().0.contains_key(&Slug::new("anon")));
    }

    fn exists_in<'a>(slugs: &'a [&'a str]) -> impl Fn(Slug) -> bool + 'a {
        move |candidate: Slug| slugs.iter().any(|s| candidate == *s)
    }

    #[test]
    fn test_nearest_directory_index_prefers_the_closest_enclosing_index() {
        let all = ["index", "notes/index", "notes/deep/index"];
        assert_eq!(
            nearest_directory_index(Slug::new("notes/deep/alice"), exists_in(&all)),
            Slug::new("notes/deep/index")
        );
    }

    #[test]
    fn test_nearest_directory_index_climbs_past_missing_levels() {
        // `notes/deep/index` does not exist, so the search continues upward.
        let existing = ["index", "notes/index"];
        assert_eq!(
            nearest_directory_index(Slug::new("notes/deep/alice"), exists_in(&existing)),
            Slug::new("notes/index")
        );
    }

    #[test]
    fn test_nearest_directory_index_falls_back_to_the_root() {
        let existing = ["index"];
        assert_eq!(
            nearest_directory_index(Slug::new("notes/deep/alice"), exists_in(&existing)),
            Slug::new("index")
        );
        assert_eq!(
            nearest_directory_index(Slug::new("alice"), exists_in(&existing)),
            Slug::new("index")
        );
    }

    #[test]
    fn test_directory_index_does_not_become_its_own_parent() {
        let all = ["index", "notes/index"];
        assert_eq!(
            nearest_directory_index(Slug::new("notes/index"), exists_in(&all)),
            Slug::new("index")
        );
    }

    #[test]
    fn test_compile_parents_unattached_sections_to_their_directory_index() {
        let mut shallows = HashMap::new();
        for slug in ["index", "notes/index", "notes/alice", "other/bob"] {
            shallows.insert(Slug::new(slug), shallow(slug));
        }

        let state = compile_all_without_missing_index_warning(&shallows).unwrap();

        // Nothing embeds these, so each falls back to its directory index.
        assert_eq!(
            state.parent_of(Slug::new("notes/alice")),
            Slug::new("notes/index")
        );
        assert_eq!(
            state.parent_of(Slug::new("notes/index")),
            Slug::new("index")
        );
        // No `other/index` exists, so this one climbs to the root.
        assert_eq!(state.parent_of(Slug::new("other/bob")), Slug::new("index"));
    }

    #[test]
    fn test_embedded_parent_wins_over_the_directory_index() {
        let mut shallows = HashMap::new();
        shallows.insert(Slug::new("index"), shallow("index"));
        shallows.insert(Slug::new("notes/index"), shallow("notes/index"));
        shallows.insert(
            Slug::new("hub"),
            shallow_with_content(
                "hub",
                HTMLContent::Lazy(vec![LazyContent::Embed(EmbedContent {
                    url: "/notes/alice".to_string(),
                    title: None,
                    option: SectionOption::default(),
                })]),
            ),
        );
        shallows.insert(Slug::new("notes/alice"), shallow("notes/alice"));

        let state = compile_all_without_missing_index_warning(&shallows).unwrap();

        assert_eq!(state.parent_of(Slug::new("notes/alice")), Slug::new("hub"));
    }

    #[test]
    fn test_declared_parent_wins_over_everything() {
        let mut shallows = HashMap::new();
        shallows.insert(Slug::new("index"), shallow("index"));
        shallows.insert(Slug::new("notes/index"), shallow("notes/index"));
        shallows.insert(Slug::new("elsewhere"), shallow("elsewhere"));

        let mut alice = shallow("notes/alice");
        alice.metadata.0.insert(
            crate::entry::KEY_PARENT.to_string(),
            HTMLContent::Plain("elsewhere".to_string()),
        );
        shallows.insert(Slug::new("notes/alice"), alice);

        let state = compile_all_without_missing_index_warning(&shallows).unwrap();

        assert_eq!(
            state.parent_of(Slug::new("notes/alice")),
            Slug::new("elsewhere")
        );
    }
}
