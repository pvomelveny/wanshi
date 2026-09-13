// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use camino::Utf8Path;
use eyre::{eyre, Context};

fn content_hash(content: &str) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    std::hash::Hash::hash(&content, &mut hasher);
    std::hash::Hasher::finish(&hasher)
}

/// Return is file modified i.e. is hash updated.
pub fn is_hash_updated<P: AsRef<Utf8Path>>(content: &str, hash_path: P) -> (bool, u64) {
    let current_hash = content_hash(content);

    let history_hash = std::fs::read_to_string(hash_path.as_ref())
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0); // no file / invalid hash: 0

    (current_hash != history_hash, current_hash)
}

/// Whether the source file at `relative_path` (under the trees directory)
/// differs from its last recorded state, along with its hash for
/// [`record_hash`].
///
/// Split from recording for the same reason as [`verify_hash`]: the check runs
/// before the source is parsed, the record only once the parse has landed in
/// the entry cache. One function that did both persisted the new hash *first*,
/// so a parse that then failed looked current to the next build — the stale
/// entry cache was served, and a source that does not compile built
/// "successfully" with its pre-edit content until it was edited again.
pub fn verify_source_hash<P: AsRef<Utf8Path>>(relative_path: P) -> eyre::Result<(bool, u64)> {
    if *crate::cli::build::no_cache_enabled() {
        // The hash is never recorded in this mode, so its value is unused.
        return Ok((true, 0));
    }

    let full_path = super::trees_dir().join(&relative_path);
    let hash_path = super::hash_file_path(&relative_path);

    let content = std::fs::read_to_string(&full_path)
        .wrap_err_with(|| eyre!("failed to read file `{}`", full_path))?;
    Ok(is_hash_updated(&content, &hash_path))
}

/// Whether `content` differs from the last recorded state of `path`'s output,
/// along with its hash for [`record_hash`].
///
/// Split from recording on purpose: the check runs before the output is
/// written, the record only after the write has landed. One function that did
/// both persisted the new hash *first*, so a write that then failed looked
/// current forever — the stale file on disk was never rewritten until its
/// source happened to change.
pub fn verify_hash<P: AsRef<Utf8Path>>(path: P, content: &str) -> (bool, u64) {
    if *crate::cli::build::no_cache_enabled() {
        return (true, content_hash(content));
    }

    let hash_path = super::hash_file_path(path.as_ref());
    is_hash_updated(content, &hash_path)
}

/// Record `hash` as the last known-good state of `path` — for an output, once
/// the page is on disk; for a source, once its parse is in the entry cache.
///
/// Call this only once that has actually happened — see [`verify_hash`] and
/// [`verify_source_hash`].
pub fn record_hash<P: AsRef<Utf8Path>>(path: P, hash: u64) -> eyre::Result<()> {
    if *crate::cli::build::no_cache_enabled() {
        return Ok(());
    }

    let hash_path = super::hash_file_path(path.as_ref());
    std::fs::write(&hash_path, hash.to_string())
        .wrap_err_with(|| eyre!("failed to write file `{}`", hash_path))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn test_is_hash_updated_returns_modified_when_hash_file_missing() {
        let root = crate::test_io::case_dir("env-hash-missing");
        let hash_path = root.join("missing.hash");
        let (is_modified, _) = is_hash_updated("content", hash_path.as_path());
        assert!(is_modified);
    }

    #[test]
    fn test_is_hash_updated_returns_modified_for_invalid_hash_history() {
        let root = crate::test_io::case_dir("env-hash-invalid");
        fs::create_dir_all(root.as_std_path()).unwrap();
        let hash_path = root.join("history.hash");
        fs::write(hash_path.as_std_path(), "not-a-number").unwrap();

        let (is_modified, _) = is_hash_updated("content", hash_path.as_path());
        assert!(is_modified);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_is_hash_updated_returns_unmodified_for_matching_hash() {
        let root = crate::test_io::case_dir("env-hash-matching");
        fs::create_dir_all(root.as_std_path()).unwrap();
        let hash_path = root.join("matching.hash");
        let (_, current_hash) = is_hash_updated("content", hash_path.as_path());
        fs::write(hash_path.as_std_path(), current_hash.to_string()).unwrap();

        let (is_modified, _) = is_hash_updated("content", hash_path.as_path());
        assert!(!is_modified);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_verify_then_record_roundtrip_detects_changes() {
        let root = crate::test_io::case_dir("env-hash-roundtrip");
        fs::create_dir_all(root.as_std_path()).unwrap();

        super::super::with_test_environment(root.clone(), super::super::BuildMode::Publish, || {
            let relative = "hash-tests/a.md";
            let (changed, hash) = verify_hash(relative, "v1");
            assert!(changed);
            record_hash(relative, hash).unwrap();

            assert!(!verify_hash(relative, "v1").0);
            assert!(verify_hash(relative, "v2").0);

            let hash_path = super::super::hash_file_path(relative);
            assert!(hash_path.exists());
        });

        let _ = fs::remove_dir_all(root);
    }

    /// The source-side check must not record anything either: recording is the
    /// parser's reward for a successful parse, not a side effect of asking. A
    /// hash persisted before the parse made a failing source look unmodified
    /// on the next build, which then served the stale entry cache and exited 0.
    #[test]
    fn test_verify_source_hash_alone_records_nothing() {
        let root = crate::test_io::case_dir("env-hash-source-verify-only");

        super::super::with_test_environment(root.clone(), super::super::BuildMode::Publish, || {
            let trees = super::super::trees_dir();
            fs::create_dir_all(trees.as_std_path()).unwrap();
            let relative = "c.typst";
            fs::write(trees.join(relative).as_std_path(), "#lorem(1)").unwrap();

            let (changed, hash) = verify_source_hash(relative).unwrap();
            assert!(changed);
            // Asked twice without recording, it must answer "changed" twice.
            assert!(verify_source_hash(relative).unwrap().0);
            assert!(!super::super::hash_file_path(relative).exists());

            // Recording is what settles it.
            record_hash(relative, hash).unwrap();
            assert!(!verify_source_hash(relative).unwrap().0);
        });

        let _ = fs::remove_dir_all(root);
    }

    /// The check must not record anything: recording is the writer's reward for
    /// a successful write, not a side effect of asking.
    #[test]
    fn test_verify_hash_alone_records_nothing() {
        let root = crate::test_io::case_dir("env-hash-verify-only");
        fs::create_dir_all(root.as_std_path()).unwrap();

        super::super::with_test_environment(root.clone(), super::super::BuildMode::Publish, || {
            let relative = "hash-tests/b.md";
            let (changed, _) = verify_hash(relative, "v1");
            assert!(changed);
            // Asked twice without recording, it must answer "changed" twice.
            assert!(verify_hash(relative, "v1").0);

            let hash_path = super::super::hash_file_path(relative);
            assert!(!hash_path.exists());
        });

        let _ = fs::remove_dir_all(root);
    }
}
