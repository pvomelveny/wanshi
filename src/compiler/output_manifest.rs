// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.

//! Tracks which pages a build produced, so that pages whose sources have since
//! disappeared can be removed from the output directory.
//!
//! Output pruning is deliberately manifest-driven rather than derived from
//! scanning the output directory: wanshi only ever deletes a file it recorded
//! writing itself. A hand-written `404.html` or a `CNAME` dropped into the
//! publish directory is therefore never at risk, and a missing manifest simply
//! disables pruning for one build instead of guessing.

use std::collections::BTreeSet;

use camino::{Utf8Path, Utf8PathBuf};
use eyre::{eyre, WrapErr};

use crate::{environment, slug::Slug};

use super::stale::remove_file_if_exists;

const MANIFEST_DIR_NAME: &str = "outputs";

/// The manifest is keyed by build mode because `wanshi build` and
/// `wanshi serve` share one cache directory but write to different output
/// directories.
fn manifest_file_name() -> &'static str {
    if environment::is_serve() {
        "serve.json"
    } else {
        "publish.json"
    }
}

fn manifest_path() -> Utf8PathBuf {
    let path = environment::get_cache_dir()
        .join(MANIFEST_DIR_NAME)
        .join(manifest_file_name());
    environment::create_parent_dirs(&path);
    path
}

/// Relative output path of the page generated for `slug`.
pub(super) fn page_relative_path(slug: Slug) -> Utf8PathBuf {
    Utf8PathBuf::from(format!("{slug}.html"))
}

pub(super) fn page_paths<I: IntoIterator<Item = Slug>>(slugs: I) -> BTreeSet<Utf8PathBuf> {
    slugs.into_iter().map(page_relative_path).collect()
}

/// Pages recorded by the previous build, or an empty set when no usable
/// manifest exists. A missing or corrupt manifest disables pruning rather than
/// failing the build.
pub(super) fn load_previous() -> BTreeSet<Utf8PathBuf> {
    let path = manifest_path();
    let Ok(contents) = std::fs::read_to_string(path.as_std_path()) else {
        return BTreeSet::new();
    };
    match serde_json::from_str::<BTreeSet<String>>(&contents) {
        Ok(pages) => pages.into_iter().map(Utf8PathBuf::from).collect(),
        Err(err) => {
            color_print::ceprintln!(
                "<y>Warning: ignoring unreadable output manifest `{}`: {}</>",
                path,
                err
            );
            BTreeSet::new()
        }
    }
}

pub(super) fn save(pages: &BTreeSet<Utf8PathBuf>) -> eyre::Result<()> {
    let path = manifest_path();
    let serializable: BTreeSet<&str> = pages.iter().map(|page| page.as_str()).collect();
    let payload = serde_json::to_string(&serializable)
        .wrap_err_with(|| eyre!("failed to serialize output manifest `{}`", path))?;
    crate::atomic_text::write_text_atomically(path.as_path(), &payload, "output manifest")
}

/// Remove pages that the previous build produced and this one did not, along
/// with their output hash records.
///
/// Dropping the hash record matters: the writer skips any page whose content
/// hash is unchanged, so a page that is deleted here and later recreated with
/// identical content would otherwise never be written back.
pub(super) fn prune_removed(
    previous: &BTreeSet<Utf8PathBuf>,
    current: &BTreeSet<Utf8PathBuf>,
) -> eyre::Result<Vec<Utf8PathBuf>> {
    let output_dir = environment::output_dir();
    let hash_dir = environment::hash_dir();
    let mut removed = Vec::new();

    for relative in previous.difference(current) {
        if !is_safe_relative_output_path(relative) {
            color_print::ceprintln!(
                "<y>Warning: skipping suspicious output manifest entry `{}`.</>",
                relative
            );
            continue;
        }

        let page_path = output_dir.join(relative);
        let did_remove = remove_file_if_exists(page_path.as_path())
            .wrap_err_with(|| eyre!("failed to remove stale page `{}`", page_path))?;

        let hash_path = super::stale::hash_cache_path_no_create(hash_dir.as_path(), relative);
        let _ = remove_file_if_exists(hash_path.as_path())?;

        if did_remove {
            removed.push(relative.clone());
            if *crate::cli::build::verbose() {
                color_print::ceprintln!("<y>[clean]</> removed stale page {}", page_path);
            }
        }
    }

    remove_empty_output_dirs(output_dir.as_path(), &removed);
    Ok(removed)
}

/// Reject anything that is not a plain relative path, so a corrupted manifest
/// can never reach outside the output directory.
fn is_safe_relative_output_path(relative: &Utf8Path) -> bool {
    !relative.as_str().is_empty()
        && relative
            .components()
            .all(|component| matches!(component, camino::Utf8Component::Normal(_)))
}

/// Drop directories left empty by pruning. Walks upward from each removed page
/// and stops at the first non-empty directory or at the output root.
fn remove_empty_output_dirs(output_dir: &Utf8Path, removed: &[Utf8PathBuf]) {
    let mut candidates: BTreeSet<Utf8PathBuf> = BTreeSet::new();
    for relative in removed {
        let mut parent = relative.parent();
        while let Some(dir) = parent {
            if dir.as_str().is_empty() {
                break;
            }
            candidates.insert(dir.to_owned());
            parent = dir.parent();
        }
    }

    // Deepest first, so a parent is only considered once its children are gone.
    for relative in candidates.iter().rev() {
        let dir = output_dir.join(relative);
        let is_empty = dir
            .read_dir_utf8()
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false);
        if is_empty {
            let _ = std::fs::remove_dir(dir.as_std_path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn with_test_env(name: &str, f: impl FnOnce(Utf8PathBuf)) {
        let root = crate::test_io::case_dir(name);
        fs::create_dir_all(root.as_std_path()).unwrap();
        let root_for_call = root.clone();
        crate::environment::with_test_environment(
            root.clone(),
            crate::environment::BuildMode::Publish,
            || f(root_for_call.clone()),
        );
        let _ = fs::remove_dir_all(root.as_std_path());
    }

    fn touch(path: &Utf8Path) {
        environment::create_parent_dirs(path);
        fs::write(path.as_std_path(), "x").unwrap();
    }

    #[test]
    fn test_page_relative_path_appends_html() {
        assert_eq!(
            page_relative_path(Slug::new("notes/alice")),
            Utf8PathBuf::from("notes/alice.html")
        );
    }

    #[test]
    fn test_prune_removes_pages_absent_from_current_build() {
        with_test_env("manifest-prune", |_root| {
            let output_dir = environment::output_dir();
            let kept = output_dir.join("notes/alice.html");
            let stale = output_dir.join("notes/bob.html");
            touch(kept.as_path());
            touch(stale.as_path());

            let previous = page_paths([Slug::new("notes/alice"), Slug::new("notes/bob")]);
            let current = page_paths([Slug::new("notes/alice")]);

            let removed = prune_removed(&previous, &current).unwrap();

            assert_eq!(removed, vec![Utf8PathBuf::from("notes/bob.html")]);
            assert!(kept.exists());
            assert!(!stale.exists());
        });
    }

    #[test]
    fn test_prune_also_drops_the_output_hash_record() {
        with_test_env("manifest-prune-hash", |_root| {
            let output_dir = environment::output_dir();
            let stale = output_dir.join("gone.html");
            touch(stale.as_path());

            let hash_path = environment::hash_file_path("gone.html");
            fs::write(hash_path.as_std_path(), "123").unwrap();

            prune_removed(&page_paths([Slug::new("gone")]), &BTreeSet::new()).unwrap();

            assert!(!stale.exists());
            assert!(
                !hash_path.exists(),
                "stale output hash must go too, or a recreated page would be skipped"
            );
        });
    }

    #[test]
    fn test_prune_removes_directories_left_empty() {
        with_test_env("manifest-prune-dirs", |_root| {
            let output_dir = environment::output_dir();
            let stale = output_dir.join("deep/nested/page.html");
            touch(stale.as_path());

            prune_removed(
                &page_paths([Slug::new("deep/nested/page")]),
                &BTreeSet::new(),
            )
            .unwrap();

            assert!(!output_dir.join("deep/nested").exists());
            assert!(!output_dir.join("deep").exists());
        });
    }

    #[test]
    fn test_prune_keeps_directories_that_still_hold_files() {
        with_test_env("manifest-prune-dirs-kept", |_root| {
            let output_dir = environment::output_dir();
            let stale = output_dir.join("notes/gone.html");
            let sibling = output_dir.join("notes/stays.html");
            touch(stale.as_path());
            touch(sibling.as_path());

            prune_removed(&page_paths([Slug::new("notes/gone")]), &BTreeSet::new()).unwrap();

            assert!(sibling.exists());
            assert!(output_dir.join("notes").exists());
        });
    }

    #[test]
    fn test_prune_ignores_entries_that_escape_the_output_dir() {
        with_test_env("manifest-prune-escape", |root| {
            let outsider = root.join("keep-me.txt");
            fs::write(outsider.as_std_path(), "important").unwrap();

            let previous: BTreeSet<Utf8PathBuf> =
                [Utf8PathBuf::from("../keep-me.txt")].into_iter().collect();
            let removed = prune_removed(&previous, &BTreeSet::new()).unwrap();

            assert!(removed.is_empty());
            assert!(outsider.exists(), "pruning must never escape the output dir");
        });
    }

    #[test]
    fn test_manifest_roundtrip() {
        with_test_env("manifest-roundtrip", |_root| {
            assert!(load_previous().is_empty());

            let pages = page_paths([Slug::new("index"), Slug::new("notes/alice")]);
            save(&pages).unwrap();

            assert_eq!(load_previous(), pages);
        });
    }

    #[test]
    fn test_load_previous_tolerates_corrupt_manifest() {
        with_test_env("manifest-corrupt", |_root| {
            let path = manifest_path();
            fs::write(path.as_std_path(), "{not json").unwrap();

            assert!(load_previous().is_empty());
        });
    }
}
