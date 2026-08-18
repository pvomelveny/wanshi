// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::fs::create_dir_all;

use camino::{Utf8Path, Utf8PathBuf};

use crate::{path_utils, slug::Slug};

use super::{ENTRY_DIR_NAME, HASH_DIR_NAME};

/// URL keep posix style, so the type of return value is [`String`].
pub fn full_url<P: AsRef<Utf8Path>>(path: P) -> String {
    let base_url = super::base_url();
    let path = path_utils::pretty_path(path.as_ref());
    if let Some(stripped) = path.strip_prefix("/") {
        return format!("{base_url}{stripped}");
    } else if let Some(stripped) = path.strip_prefix("./") {
        return format!("{base_url}{stripped}");
    }
    format!("{base_url}{path}")
}

pub fn full_html_url(slug: Slug) -> String {
    // A directory index is already served by its directory: `notes/index.html`
    // answers a request for `notes/`, so link to the directory. This also makes
    // the URL agree with the slug shown beside the title, which already reads
    // `[notes]` rather than `[notes/index]`.
    //
    // Built here rather than through `full_url`, which normalizes paths and
    // would drop the trailing slash. The slash is load-bearing: without it a
    // relative reference from the page resolves against the parent directory.
    if let Some(directory) = crate::slug::directory_of_index(slug) {
        let base_url = super::base_url();
        return if directory.is_empty() {
            base_url
        } else {
            format!("{base_url}{directory}/")
        };
    }

    let pretty_urls = super::with_config(|cfg| cfg.build.pretty_urls);
    let page_suffix = super::to_page_suffix(pretty_urls);
    full_url(format!("{}{}", slug, page_suffix))
}

pub fn input_path<P: AsRef<Utf8Path>>(path: P) -> Utf8PathBuf {
    let mut filepath: Utf8PathBuf = super::trees_dir();
    filepath.push(path);
    filepath
}

pub fn create_parent_dirs<P: AsRef<Utf8Path>>(path: P) {
    let Some(parent_dir) = path.as_ref().parent() else {
        return;
    };
    if !parent_dir.exists() {
        if let Err(err) = create_dir_all(parent_dir) {
            color_print::ceprintln!(
                "<y>Warning: failed to create parent directory `{}`: {}</>",
                parent_dir,
                err
            );
        }
    }
}

/// Path to a generated file inside the output directory, creating its parent
/// directories.
///
/// [`super::output_dir`] is already rooted at the project, so this only joins
/// and creates. It used to route through a helper that started from
/// `root_dir()` and pushed the *already-rooted* output directory onto it,
/// applying the root twice: with root `notes` and output `../public/notes` the
/// pages went to `notes/notes/../public/notes`, i.e. `notes/public/notes`,
/// while the JSON artifacts — which use `output_dir()` directly — went to the
/// right place. Running from inside the project hid it, because the root is
/// then `.` and doubling it changes nothing.
pub fn output_path<P: AsRef<Utf8Path>>(path: P) -> Utf8PathBuf {
    let filepath = super::output_dir().join(path);
    create_parent_dirs(&filepath);
    filepath
}

pub fn hash_dir() -> Utf8PathBuf {
    super::get_cache_dir().join(HASH_DIR_NAME)
}

/// Return the hash file path `<hash_dir>/<path>.hash` for the given file or directory.
/// e.g. `/path/to/index.md` will return `<hash_dir>/path/to/index.md.hash`.
///
/// If the directory does not exist, it will be created.
pub fn hash_file_path<P: AsRef<Utf8Path>>(path: P) -> Utf8PathBuf {
    let mut hash_path = hash_dir();
    hash_path.push(path);
    let ext = hash_path
        .extension()
        .map(|ext| format!("{ext}.hash"))
        .unwrap_or_else(|| "hash".to_string());
    hash_path.set_extension(ext);
    create_parent_dirs(&hash_path);
    hash_path
}

pub fn entry_dir() -> Utf8PathBuf {
    super::get_cache_dir().join(ENTRY_DIR_NAME)
}

/// Return the hash file path `<hash_dir>/<path>.hash` for the given file or directory.
/// e.g. `/path/to/index.md` will return `<entry_dir>/path/to/index.md.entry`.
///
/// If the directory does not exist, it will be created.
pub fn entry_file_path<P: AsRef<Utf8Path>>(path: P) -> Utf8PathBuf {
    let mut entry_path = entry_dir();
    entry_path.push(path);
    let ext = entry_path
        .extension()
        .map(|ext| format!("{ext}.entry"))
        .unwrap_or_else(|| "entry".to_string());
    entry_path.set_extension(ext);
    create_parent_dirs(&entry_path);
    entry_path
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn test_create_parent_dirs_creates_missing_directories() {
        let root = crate::test_io::case_dir("env-paths-parent");
        let target = root.join("a/b/c/file.txt");
        create_parent_dirs(target.as_path());
        assert!(target.parent().is_some_and(|parent| parent.exists()));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_full_url_normalizes_leading_prefixes() {
        let root = crate::test_io::case_dir("env-paths-full-url");
        fs::create_dir_all(root.as_std_path()).unwrap();

        super::super::with_test_environment(root.clone(), super::super::BuildMode::Publish, || {
            let base = super::super::base_url();
            assert_eq!(full_url("/notes/a"), format!("{base}notes/a"));
            assert_eq!(full_url("./notes/a"), format!("{base}notes/a"));
            assert_eq!(full_url("notes/a"), format!("{base}notes/a"));
        });

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_full_html_url_links_directory_indexes_to_their_directory() {
        let root = crate::test_io::case_dir("env-paths-dir-index");
        fs::create_dir_all(root.as_std_path()).unwrap();

        super::super::with_test_environment(root.clone(), super::super::BuildMode::Publish, || {
            let base = super::super::base_url();

            // The trailing slash matters: without it a relative reference from
            // the page would resolve against the parent directory.
            assert_eq!(
                full_html_url(Slug::new("notes/index")),
                format!("{base}notes/")
            );
            assert_eq!(
                full_html_url(Slug::new("notes/deep/index")),
                format!("{base}notes/deep/")
            );
            // The root index is the site root.
            assert_eq!(full_html_url(Slug::new("index")), base);

            // Ordinary pages are untouched.
            assert_eq!(
                full_html_url(Slug::new("notes/alice")),
                format!("{base}notes/alice.html")
            );
            // A name that merely ends in the word is not a directory index.
            assert_eq!(
                full_html_url(Slug::new("notes/reindex")),
                format!("{base}notes/reindex.html")
            );
        });

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_hash_and_entry_paths_preserve_original_extension_suffix() {
        let root = crate::test_io::case_dir("env-paths-hash-entry");
        fs::create_dir_all(root.as_std_path()).unwrap();

        super::super::with_test_environment(root.clone(), super::super::BuildMode::Publish, || {
            let hash = hash_file_path("nested/a.b.md");
            let entry = entry_file_path("nested/a.b.md");

            assert!(hash.as_str().contains("a.b.md.hash"));
            assert!(entry.as_str().contains("a.b.md.entry"));
            assert!(hash.parent().is_some_and(|parent| parent.exists()));
            assert!(entry.parent().is_some_and(|parent| parent.exists()));
        });

        let _ = fs::remove_dir_all(root);
    }

    /// Regression: `output_path` applied the project root twice.
    ///
    /// `output_dir()` is already rooted, so pushing it onto `root_dir()` again
    /// sent pages to `<root>/<root>/<output>` while the JSON artifacts, which
    /// use `output_dir()` directly, went to `<root>/<output>`. A build split
    /// its own output across two directories. Invisible from inside a project,
    /// where the root is `.`.
    #[test]
    fn test_output_path_does_not_apply_the_root_twice() {
        let root = crate::test_io::case_dir("env-paths-output-root-once");
        fs::create_dir_all(root.as_std_path()).unwrap();

        super::super::with_test_environment(root.clone(), super::super::BuildMode::Publish, || {
            let expected = super::super::output_dir().join("index.html");
            assert_eq!(output_path("index.html"), expected);
        });

        let _ = fs::remove_dir_all(root);
    }

    /// A page and an artifact written by the same build must land under the
    /// same directory, whatever the root is.
    #[test]
    fn test_output_path_agrees_with_output_dir() {
        let root = crate::test_io::case_dir("env-paths-output-agrees");
        fs::create_dir_all(root.as_std_path()).unwrap();

        super::super::with_test_environment(root.clone(), super::super::BuildMode::Publish, || {
            let dir = super::super::output_dir();
            let page = output_path("notes/a.html");
            assert!(
                page.starts_with(&dir),
                "page {page} should sit under the output dir {dir}"
            );
        });

        let _ = fs::remove_dir_all(root);
    }

    /// Regression: `typst-root` was the one configured path taken as written
    /// rather than resolved against the project root, so it silently meant
    /// "relative to the current working directory". `wanshi build --config
    /// path/to/Wanshi.toml` from anywhere else could not find the sources.
    #[test]
    fn test_typst_root_dir_is_resolved_against_the_project_root() {
        let root = crate::test_io::case_dir("env-paths-typst-root");
        fs::create_dir_all(root.as_std_path()).unwrap();

        super::super::with_test_environment(root.clone(), super::super::BuildMode::Publish, || {
            let typst_root = super::super::typst_root_dir();
            assert!(
                typst_root.starts_with(&root),
                "typst root {typst_root} should sit under the project root {root}"
            );
            // And it agrees with where sections are actually read from.
            assert_eq!(typst_root, super::super::trees_dir());
        });

        let _ = fs::remove_dir_all(root);
    }
}
