// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use camino::{Utf8Path, Utf8PathBuf};

use crate::config::{build::FooterMode, toc, wanshi};

use super::{with_config, with_environment, BuildMode, CACHE_DIR_NAME};

pub fn is_short_slug() -> bool {
    with_config(|cfg| cfg.build.short_slug)
}

/// Root passed to Typst compilation, resolved against the project root.
///
/// Every other configured path is interpreted relative to the directory holding
/// `Wanshi.toml` — see [`trees_dir`], [`output_dir`], [`assets_dir`]. This one
/// used to be taken as written, so it silently meant "relative to the current
/// working directory" instead. Running from inside the project hid that, since
/// the two coincide there; `wanshi build --config path/to/Wanshi.toml` from
/// anywhere else looked for the sources under the wrong directory and failed
/// with a "typst source not found" error naming a path that did not exist.
pub fn typst_root_dir() -> Utf8PathBuf {
    super::root_dir().join(with_config(|cfg| cfg.build.typst_root.clone()))
}

pub fn trees_dir_without_root() -> String {
    with_config(|cfg| cfg.wanshi.trees.clone())
}

fn assets_dir_without_root() -> String {
    with_config(|cfg| cfg.wanshi.assets.clone())
}

pub fn trees_dir() -> Utf8PathBuf {
    super::root_dir().join(trees_dir_without_root())
}

/// Slug prefix under the source tree where generated reference stubs live.
///
/// Returned with a trailing `/` so a `starts_with` test cannot match a sibling
/// whose name merely begins with it — `refs` must not claim `refsheet/x`.
pub fn refs_dir() -> String {
    let dir = with_config(|cfg| cfg.refs.dir.clone());
    let trimmed = dir.trim_matches('/');
    format!("{trimmed}/")
}

/// Path to the bibliography, resolved against the project root.
///
/// Root-relative, not cwd-relative: every other configured path works that way,
/// and the one that did not (`typst-root`) was a bug — `wanshi build --config
/// path/to/Wanshi.toml` from elsewhere looked for sources under the wrong
/// directory.
pub fn refs_bibliography() -> Utf8PathBuf {
    super::root_dir().join(with_config(|cfg| cfg.refs.bibliography.clone()))
}

pub fn theme_paths() -> Vec<Utf8PathBuf> {
    let root = super::root_dir();
    with_config(|cfg| {
        cfg.wanshi
            .themes
            .iter()
            .map(|theme| root.join(theme))
            .collect()
    })
}

pub fn theme_lock() -> bool {
    with_config(|cfg| cfg.wanshi.theme_lock)
}

pub fn output_dir() -> Utf8PathBuf {
    with_environment(|env| {
        let output = match env.build_mode {
            BuildMode::Publish | BuildMode::Check => env.config.build.output.clone(),
            BuildMode::Serve => env.config.serve.output.clone(),
        };
        env.root.join(output)
    })
}

pub fn indexes_path(output_dir: &Utf8Path) -> Utf8PathBuf {
    output_dir.join("wanshi.json")
}

pub fn graph_path(output_dir: &Utf8Path) -> Utf8PathBuf {
    output_dir.join("wanshi.graph.json")
}

pub const FEED_NAME: &str = "feed.xml";

pub fn feed_path(output_dir: &Utf8Path) -> Utf8PathBuf {
    output_dir.join(FEED_NAME)
}

pub const SEARCH_INDEX_NAME: &str = "wanshi.search.json";

pub fn search_index_path(output_dir: &Utf8Path) -> Utf8PathBuf {
    output_dir.join(SEARCH_INDEX_NAME)
}

pub fn search_enabled() -> bool {
    with_config(|cfg| cfg.build.search)
}

pub fn search_content_enabled() -> bool {
    with_config(|cfg| cfg.build.search && cfg.build.search_content)
}

pub fn reload_marker_path(output_dir: &Utf8Path) -> Utf8PathBuf {
    output_dir.join("wanshi.reload")
}

pub fn base_url_raw() -> String {
    with_config(|cfg| cfg.wanshi.base_url.clone())
}

pub fn base_url() -> String {
    with_environment(|env| match env.build_mode {
        BuildMode::Publish | BuildMode::Check => env.config.wanshi.base_url.clone(),
        BuildMode::Serve => wanshi::DEFAULT_BASE_URL.to_string(),
    })
}

pub fn is_toc_left() -> bool {
    match with_config(|cfg| cfg.toc.placement) {
        toc::TocPlacement::Left => true,
        toc::TocPlacement::Right => false,
    }
}

pub fn is_toc_sticky() -> bool {
    with_config(|cfg| cfg.toc.sticky)
}

pub fn is_toc_mobile_sticky() -> bool {
    with_config(|cfg| cfg.toc.mobile_sticky)
}

pub fn toc_max_width() -> String {
    with_config(|cfg| cfg.toc.max_width.clone())
}

pub fn get_edit_text() -> String {
    with_config(|cfg| cfg.text.edit.clone())
}

pub fn get_toc_text() -> String {
    with_config(|cfg| cfg.text.toc.clone())
}

pub fn get_search_text() -> String {
    with_config(|cfg| cfg.text.search.clone())
}

pub fn get_footer_references_text() -> String {
    with_config(|cfg| cfg.text.references.clone())
}

pub fn get_footer_backlinks_text() -> String {
    with_config(|cfg| cfg.text.backlinks.clone())
}

pub fn get_footer_embedded_by_text() -> String {
    with_config(|cfg| cfg.text.embedded_by.clone())
}

pub fn footer_mode() -> FooterMode {
    with_config(|cfg| cfg.build.footer_mode)
}

pub fn footer_sort_by() -> String {
    with_config(|cfg| cfg.build.footer_sort_by.clone())
}

/// Metadata keys allowed to appear beneath a title, in the order given.
///
/// Everything else stays in `wanshi.json` and remains usable as a sort key or
/// query filter — it is simply not printed. Display is opt-in because custom
/// keys render as bare values with no key name.
pub fn header_keys() -> Vec<String> {
    with_config(|cfg| cfg.build.header_keys.clone())
}

pub fn publish_rss() -> bool {
    with_config(|cfg| cfg.publish.rss)
}

pub fn inline_css() -> bool {
    with_config(|cfg| cfg.build.inline_css)
}

pub fn inline_script() -> bool {
    with_config(|cfg| cfg.build.inline_script)
}

pub fn asref() -> bool {
    with_config(|cfg| cfg.build.asref)
}

pub fn numbering() -> bool {
    with_config(|cfg| cfg.build.numbering)
}

pub fn deploy_edit_url() -> Option<String> {
    with_config(|cfg| cfg.build.edit.clone())
}

pub fn editor_url() -> Option<String> {
    with_config(|cfg| cfg.serve.edit.clone())
}

pub fn serve_command() -> Vec<String> {
    with_config(|cfg| cfg.serve.command.clone())
}

pub fn get_cache_dir() -> Utf8PathBuf {
    super::root_dir().join(CACHE_DIR_NAME)
}

pub fn assets_dir() -> Utf8PathBuf {
    super::root_dir().join(assets_dir_without_root())
}

/// The single path segment the assets directory is published under.
///
/// The build copies `assets_dir()` to `<output>/<this>`, so every generated URL
/// pointing into assets has to agree with it. Deriving both from one function
/// keeps a renamed `[wanshi].assets` from moving the files out from under the
/// links — which is how the favicon came to point at a directory that a host
/// site, not wanshi, owns.
pub fn assets_dir_name() -> Option<String> {
    assets_dir().file_name().map(str::to_owned)
}
