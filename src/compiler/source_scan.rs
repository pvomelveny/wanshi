// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Alias Qli (@AliasQli), Spore (@s-cerevisiae)

use std::collections::HashMap;

use camino::{Utf8Path, Utf8PathBuf};
use eyre::{bail, eyre, WrapErr};
use walkdir::WalkDir;

use crate::{
    path_utils,
    slug::{Ext, Slug},
};

#[derive(Debug)]
pub struct Workspace {
    pub slug_exts: HashMap<Slug, Ext>,
}

/// Files whose name starts with `_` or `.` are helpers, not notes: shared Typst
/// modules, macro definitions, and editor scratch files live alongside the notes
/// that import them without becoming pages themselves.
///
/// This mirrors [`should_ignore_dir`], so the same leading underscore means the
/// same thing whether it is on a file or a directory.
pub fn should_ignore_file(path: &Utf8Path) -> bool {
    path.file_name()
        .is_some_and(|name| name.starts_with(['.', '_']))
}

pub fn should_ignore_dir(path: &Utf8Path) -> bool {
    path.file_name()
        .is_some_and(|name| name.starts_with(['.', '_']))
}

/// Whether a source-tree-relative path would be discovered as a section source.
///
/// A file can carry a source extension and still not be a note — a shared Typst
/// module under `_lib/`, for instance — so callers that need to tell "a note
/// changed" from "something a note imports changed" must ask this rather than
/// inspect the extension alone.
pub fn is_section_source_path(relative: &Utf8Path) -> bool {
    Ext::is_source_extension(relative.extension())
        && !should_ignore_file(relative)
        && relative
            .ancestors()
            .skip(1)
            .all(|dir| !should_ignore_dir(dir))
}

fn to_slug_ext(source_dir: &Utf8Path, p: &Utf8Path) -> Option<(Slug, Ext)> {
    let p = p.strip_prefix(source_dir).unwrap_or(p);
    let ext = p.extension()?.parse().ok()?;
    let slug = Slug::new(path_utils::pretty_path(&p.with_extension("")));
    Some((slug, ext))
}

/// Collect all source file paths in `<trees>` dir.
pub fn all_trees_source(trees_dir: &Utf8Path) -> eyre::Result<Workspace> {
    all_trees_source_inner(trees_dir)
}

fn all_trees_source_inner(trees_dir: &Utf8Path) -> eyre::Result<Workspace> {
    let mut slug_exts = HashMap::new();

    let failed_to_read_dir = |dir: &Utf8Path| eyre!("failed to read directory `{}`", dir);
    let file_collide = |p: &Utf8Path, e: Ext| {
        eyre!(
            "`{}` collides with `{}`",
            p,
            p.with_extension(e.to_string()),
        )
    };

    if !trees_dir.exists() {
        color_print::ceprintln!(
            "<y>Warning: Source directory `{}` does not exist, skipping.</>",
            trees_dir
        );
        return Ok(Workspace { slug_exts });
    }

    // One walk for the whole tree: scanning the top level separately from its
    // subdirectories previously let the two paths apply different ignore rules.
    let walker = WalkDir::new(trees_dir)
        .follow_links(true)
        .into_iter()
        .filter_entry(|entry| {
            // Never reject the root, even when the configured source directory
            // itself begins with `_` or `.`.
            if entry.depth() == 0 || !entry.file_type().is_dir() {
                return true;
            }
            Utf8Path::from_path(entry.path()).is_none_or(|path| !should_ignore_dir(path))
        });

    for entry in walker {
        let entry = entry.wrap_err_with(|| failed_to_read_dir(trees_dir))?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = match Utf8PathBuf::from_path_buf(entry.into_path()) {
            Ok(path) => path,
            Err(non_utf8) => {
                color_print::ceprintln!(
                    "<y>Warning: skipping non-UTF-8 path `{}`.</>",
                    non_utf8.display()
                );
                continue;
            }
        };

        if should_ignore_file(path.as_path()) {
            continue;
        }

        let Some((slug, ext)) = to_slug_ext(trees_dir, &path) else {
            continue;
        };

        if let Some(previous) = slug_exts.insert(slug, ext) {
            bail!(file_collide(&path, previous));
        }
    }

    Ok(Workspace { slug_exts })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_ignore_helpers_handle_missing_file_name() {
        let empty = Utf8Path::new("");
        assert!(!should_ignore_file(empty));
        assert!(!should_ignore_dir(empty));
    }

    #[test]
    fn test_should_ignore_helpers_match_expected_names() {
        assert!(should_ignore_file(Utf8Path::new("_macros.typ")));
        assert!(should_ignore_file(Utf8Path::new(".hidden.typ")));
        assert!(!should_ignore_file(Utf8Path::new("alice.typ")));

        assert!(should_ignore_dir(Utf8Path::new(".git")));
        assert!(should_ignore_dir(Utf8Path::new("_tmp")));
        assert!(!should_ignore_dir(Utf8Path::new("trees")));
    }

    #[test]
    fn test_all_trees_source_returns_empty_workspace_when_trees_missing() {
        let missing = crate::test_io::case_dir("missing-trees");

        let workspace = all_trees_source(missing.as_path()).expect("scan should succeed");
        assert!(workspace.slug_exts.is_empty());
    }

    fn write(root: &Utf8Path, relative: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap().as_std_path()).unwrap();
        std::fs::write(path.as_std_path(), "x").unwrap();
    }

    fn scan(name: &str, files: &[&str]) -> Vec<String> {
        let root = crate::test_io::case_dir(name);
        std::fs::create_dir_all(root.as_std_path()).unwrap();
        for relative in files {
            write(root.as_path(), relative);
        }
        let workspace = all_trees_source(root.as_path()).expect("scan should succeed");
        let _ = std::fs::remove_dir_all(root.as_std_path());

        let mut slugs: Vec<String> = workspace
            .slug_exts
            .keys()
            .map(|slug| slug.to_string())
            .collect();
        slugs.sort();
        slugs
    }

    #[test]
    fn test_is_section_source_path_separates_notes_from_helpers() {
        assert!(is_section_source_path(Utf8Path::new("notes/alice.typ")));
        assert!(is_section_source_path(Utf8Path::new("legacy.typst")));

        // Helpers: right extension, still not notes.
        assert!(!is_section_source_path(Utf8Path::new("_macros.typ")));
        assert!(!is_section_source_path(Utf8Path::new("notes/_shared.typ")));
        assert!(!is_section_source_path(Utf8Path::new("_lib/wanshi.typ")));
        assert!(!is_section_source_path(Utf8Path::new(
            "notes/_private/deep.typ"
        )));

        // Not a source at all.
        assert!(!is_section_source_path(Utf8Path::new("notes/data.json")));
    }

    #[test]
    fn test_scan_accepts_both_source_extensions() {
        assert_eq!(
            scan("scan-exts", &["modern.typ", "legacy.typst"]),
            vec!["legacy".to_string(), "modern".to_string()]
        );
    }

    #[test]
    fn test_scan_skips_underscore_files_at_every_depth() {
        // The nested case is the one that regressed historically: the top level
        // and the subdirectories used to be scanned by separate code paths with
        // different ignore rules.
        assert_eq!(
            scan(
                "scan-underscore-files",
                &[
                    "_macros.typ",
                    "notes/_helpers.typ",
                    "notes/deep/_shared.typ",
                    "notes/alice.typ",
                ],
            ),
            vec!["notes/alice".to_string()]
        );
    }

    #[test]
    fn test_scan_skips_underscore_and_dot_directories() {
        assert_eq!(
            scan(
                "scan-ignored-dirs",
                &[
                    "_lib/wanshi.typ",
                    "_figures/shapes.typ",
                    ".cache/stale.typ",
                    "notes/deep/_private/secret.typ",
                    "notes/bob.typ",
                ],
            ),
            vec!["notes/bob".to_string()]
        );
    }

    #[test]
    fn test_scan_ignores_files_that_are_not_source_extensions() {
        assert_eq!(
            scan(
                "scan-non-sources",
                &["README.md", "notes/data.json", "notes/alice.typ"],
            ),
            vec!["notes/alice".to_string()]
        );
    }

    #[test]
    fn test_scan_rejects_two_sources_that_claim_the_same_slug() {
        let root = crate::test_io::case_dir("scan-collision");
        std::fs::create_dir_all(root.as_std_path()).unwrap();
        write(root.as_path(), "notes/alice.typ");
        write(root.as_path(), "notes/alice.typst");

        let err = all_trees_source(root.as_path()).unwrap_err();
        let _ = std::fs::remove_dir_all(root.as_std_path());

        assert!(format!("{err}").contains("collides with"));
    }

    #[test]
    fn test_scan_does_not_reject_a_source_root_that_starts_with_underscore() {
        let root = crate::test_io::case_dir("scan-underscore-root");
        let trees = root.join("_content");
        std::fs::create_dir_all(trees.as_std_path()).unwrap();
        write(trees.as_path(), "alice.typ");

        let workspace = all_trees_source(trees.as_path()).expect("scan should succeed");
        let _ = std::fs::remove_dir_all(root.as_std_path());

        assert_eq!(workspace.slug_exts.len(), 1);
    }
}
