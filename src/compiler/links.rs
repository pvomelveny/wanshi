// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.

//! Local links that resolve to nothing.
//!
//! A dangling link is a warning rather than an error by design — drafting a
//! link before its target exists is a legitimate way to work. That makes the
//! set useful beyond diagnostics: it is also the list of notes the author has
//! promised and not yet written, which is what `wanshi refs sync` fills in from
//! a bibliography.
//!
//! Both callers must agree on what "resolves to" means. The rule is not
//! obvious — a target beginning with `/` is root-absolute in the slug space,
//! and anything else resolves against the *containing directory*, so
//! `#local("boolean/x")` written inside `boolean/index` means
//! `boolean/boolean/x`. Sharing this function is what keeps a second
//! implementation from disagreeing with the one that publishes the site.

use std::collections::{HashMap, HashSet};

use crate::{
    compiler::section::{HTMLContent, LazyContent, UnresolvedSection},
    path_utils, slug,
    slug::Slug,
};

/// A local link whose target does not exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DanglingLink {
    /// The section containing the link.
    pub from: Slug,
    /// The target exactly as it was written.
    pub url: String,
    /// The slug that target resolves to.
    pub target: Slug,
}

impl DanglingLink {
    /// The message both the build and `wanshi check` print for this link.
    ///
    /// One wording in one place, for the same reason the resolver is shared:
    /// the build's own shorter line dropped the URL *as written* — the part
    /// that usually shows the mistake, since this is nearly always a
    /// relative-vs-absolute slip — along with the pointer to `refs sync` that
    /// a missing cited work needs.
    pub fn message(&self) -> String {
        let mut message = format!(
            "Dangling local link in `{}`: `{}` resolves to missing section `{}`.",
            self.from, self.url, self.target
        );
        // A missing work is a different problem from a missing note: the note
        // has to be written, the work only has to be imported.
        if self
            .target
            .as_str()
            .starts_with(&crate::environment::refs_dir())
        {
            message.push_str(" Run `wanshi refs sync` to generate it from the bibliography.");
        }
        message
    }
}

/// Every local link in the forest that resolves to a missing section.
///
/// Sorted, and deduplicated on the `(from, target, url)` triple, so repeating a
/// link within one note reports once and the output is stable between runs.
pub fn dangling_local_links(shallows: &HashMap<Slug, UnresolvedSection>) -> Vec<DanglingLink> {
    let mut seen = HashSet::new();
    let mut dangling = Vec::new();

    for (&from, section) in shallows {
        let mut check_contents = |contents: &[LazyContent]| {
            for content in contents {
                let LazyContent::Local(local) = content else {
                    continue;
                };
                let target = resolve_local_target(from, &local.url);
                if shallows.contains_key(&target) {
                    continue;
                }
                if seen.insert((from, target, local.url.clone())) {
                    dangling.push(DanglingLink {
                        from,
                        url: local.url.clone(),
                        target,
                    });
                }
            }
        };

        if let HTMLContent::Lazy(contents) = &section.content {
            check_contents(contents);
        }
        // Metadata values are compiled like bodies — a link written in a title
        // still renders as a link and records a backlink — so a dangling one
        // there is just as published as one in the body. Scanning only the
        // content let it slip past `check --strict` and `refs sync` alike.
        for value in section.metadata.0.values() {
            if let HTMLContent::Lazy(contents) = value {
                check_contents(contents);
            }
        }
    }

    dangling.sort_by(|a, b| {
        (a.from.as_str(), a.target.as_str(), &a.url).cmp(&(
            b.from.as_str(),
            b.target.as_str(),
            &b.url,
        ))
    });
    dangling
}

/// Resolve a `#local()` target, written in `current_slug`, to a slug.
pub fn resolve_local_target(current_slug: Slug, url: &str) -> Slug {
    slug::to_slug(path_utils::relative_to_current(current_slug.as_str(), url))
}

/// Parse every section in the workspace, bypassing the cache.
///
/// Returns the sections and any per-file failures, rather than printing or
/// bailing, so a caller can decide: `check` turns them into diagnostics, while
/// `refs sync` refuses to write anything if the forest does not parse.
///
/// Cache-free on purpose. Both callers run rarely and need the current state of
/// every file; a stale entry would mean generating a stub for a link that has
/// since been deleted.
pub fn parse_all_sections(
    workspace: &crate::compiler::Workspace,
) -> (HashMap<Slug, UnresolvedSection>, Vec<String>) {
    let mut shallows = HashMap::new();
    let mut failures = Vec::new();

    let mut entries: Vec<(Slug, crate::slug::Ext)> = workspace
        .slug_exts
        .iter()
        .map(|(&slug, &ext)| (slug, ext))
        .collect();
    entries.sort_by_key(|(slug, _)| slug.as_str());

    for (slug, ext) in entries {
        match crate::compiler::parse_source_sections(slug, ext) {
            Ok(sections) => {
                for (section_slug, section) in sections {
                    if shallows.insert(section_slug, section).is_some() {
                        failures.push(format!(
                            "Duplicate section slug `{section_slug}` generated while parsing `{slug}.{ext}`."
                        ));
                    }
                }
            }
            Err(err) => failures.push(format!("Failed to parse `{slug}.{ext}`: {err:#}")),
        }
    }

    (shallows, failures)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::section::{HTMLContentBuilder, LocalLink};

    fn section_linking_to(targets: &[&str]) -> UnresolvedSection {
        let mut builder = HTMLContentBuilder::new();
        for target in targets {
            builder.push(LazyContent::Local(LocalLink {
                url: target.to_string(),
                text: None,
            }));
        }
        UnresolvedSection {
            metadata: crate::entry::HTMLMetaData(crate::ordered_map::OrderedMap::new()),
            content: builder.build(),
        }
    }

    #[test]
    fn test_root_absolute_target_resolves_from_the_tree_root() {
        assert_eq!(
            resolve_local_target(Slug::new("boolean/index"), "/welcome"),
            Slug::new("welcome")
        );
    }

    #[test]
    fn test_relative_target_resolves_against_the_containing_directory() {
        // The trap: `boolean/basics` written inside `boolean/index` is
        // `boolean/boolean/basics`, not `boolean/basics`.
        assert_eq!(
            resolve_local_target(Slug::new("boolean/index"), "basics"),
            Slug::new("boolean/basics")
        );
        assert_eq!(
            resolve_local_target(Slug::new("boolean/index"), "boolean/basics"),
            Slug::new("boolean/boolean/basics")
        );
    }

    #[test]
    fn test_dangling_links_reports_only_missing_targets() {
        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("notes/a"),
            section_linking_to(&["/notes/b", "/refs/kkl1988"]),
        );
        shallows.insert(Slug::new("notes/b"), section_linking_to(&[]));

        let dangling = dangling_local_links(&shallows);
        assert_eq!(dangling.len(), 1, "only the missing one: {dangling:?}");
        assert_eq!(dangling[0].target, Slug::new("refs/kkl1988"));
        assert_eq!(dangling[0].from, Slug::new("notes/a"));
    }

    // Regression test: metadata values compile like bodies — a link in a title
    // renders as a link and records a backlink — but only the body was
    // scanned, so a dangling link written in metadata was published silently:
    // `check --strict` reported zero problems and `refs sync` never stubbed it.
    #[test]
    fn test_dangling_links_include_links_written_in_metadata_values() {
        let mut builder = HTMLContentBuilder::new();
        builder.push(LazyContent::Local(LocalLink {
            url: "/refs/missing".to_string(),
            text: None,
        }));
        let mut metadata = crate::ordered_map::OrderedMap::new();
        metadata.insert("title".to_string(), builder.build());

        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("notes/a"),
            UnresolvedSection {
                metadata: crate::entry::HTMLMetaData(metadata),
                content: HTMLContent::Plain("<p>body</p>".to_string()),
            },
        );

        let dangling = dangling_local_links(&shallows);
        assert_eq!(dangling.len(), 1, "the metadata link must be seen");
        assert_eq!(dangling[0].target, Slug::new("refs/missing"));
        assert_eq!(dangling[0].from, Slug::new("notes/a"));
    }

    // The build and `wanshi check` report the same condition, so they report
    // it in the same words: the build's own shorter wording named only the
    // resolved slug, leaving out the URL as written and the `refs sync`
    // pointer that a missing cited work needs.
    #[test]
    fn test_dangling_link_message_names_the_url_and_hints_at_refs_sync() {
        let _guard = crate::environment::mock_environment();

        let note = DanglingLink {
            from: Slug::new("notes/a"),
            url: "./nowhere".to_string(),
            target: Slug::new("notes/nowhere"),
        };
        assert_eq!(
            note.message(),
            "Dangling local link in `notes/a`: `./nowhere` resolves to missing section `notes/nowhere`."
        );

        let work = DanglingLink {
            from: Slug::new("notes/a"),
            url: "/refs/kkl1988".to_string(),
            target: Slug::new("refs/kkl1988"),
        };
        assert!(
            work.message()
                .ends_with("Run `wanshi refs sync` to generate it from the bibliography."),
            "a missing cited work points at the command that writes it: {}",
            work.message()
        );
    }

    #[test]
    fn test_dangling_links_deduplicates_a_repeated_link() {
        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("notes/a"),
            section_linking_to(&["/refs/x", "/refs/x"]),
        );
        assert_eq!(dangling_local_links(&shallows).len(), 1);
    }
}
