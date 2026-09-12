// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

pub mod build;
pub mod publish;
pub mod refs;
pub mod serve;
pub mod text;
pub mod toc;
pub mod wanshi;

use build::Build;
use camino::Utf8PathBuf;
use publish::Publish;
use refs::Refs;
use serde::{Deserialize, Serialize};
use serve::Serve;
use text::Text;
use toc::Toc;
use wanshi::Wanshi;

pub const DEFAULT_CONFIG_PATH: &str = "./Wanshi.toml";

#[derive(Deserialize, Debug, Default, Serialize)]
pub struct Config {
    #[serde(default)]
    pub wanshi: Wanshi,

    #[serde(default)]
    pub toc: Toc,

    #[serde(default)]
    pub text: Text,

    #[serde(default)]
    pub build: Build,

    #[serde(default)]
    pub serve: Serve,

    #[serde(default)]
    pub publish: Publish,

    #[serde(default)]
    pub refs: Refs,
}

/// Try to find toml file in the current directory or the parent directory.
pub fn find_config(mut toml_file: Utf8PathBuf) -> eyre::Result<Utf8PathBuf> {
    if !toml_file.exists() {
        let parent = toml_file
            .parent()
            .ok_or_else(|| eyre::eyre!("cannot resolve parent directory of `{}`", toml_file))?
            .canonicalize_utf8()?;
        let parent = parent.parent().ok_or_else(|| {
            eyre::eyre!(
                "cannot find configuration file from root directory while searching from `{}`",
                toml_file
            )
        })?;

        toml_file = parent.join(DEFAULT_CONFIG_PATH);
        if !toml_file.exists() {
            return Err(eyre::eyre!("cannot find configuration file: {}", toml_file));
        }
    }
    Ok(toml_file)
}

pub fn parse_config(config: &str) -> eyre::Result<Config> {
    let mut config: Config =
        toml::from_str(config).map_err(|e| eyre::eyre!("failed to parse config file: {}", e))?;
    // Every URL builder concatenates a page path directly onto `base-url`, so
    // without the trailing slash every link, feed item, and search index URL
    // came out as `https://example.comnotes/a.html`. Normalised rather than
    // warned about: the slash-less spelling has exactly one plausible meaning.
    if !config.wanshi.base_url.ends_with('/') {
        config.wanshi.base_url.push('/');
    }
    Ok(config)
}

#[cfg(test)]
mod test {

    #[test]
    fn test_empty_toml() {
        let serve = crate::config::Serve::default();
        let config = crate::config::parse_config("").unwrap();

        assert_eq!(config.wanshi.trees, "trees");
        assert_eq!(config.wanshi.assets, "assets");
        assert_eq!(config.wanshi.base_url, "/");
        assert!(config.wanshi.theme_lock);
        assert!(!config.build.short_slug);
        assert!(!config.build.pretty_urls);
        assert!(!config.build.inline_css);
        assert!(!config.build.inline_script);
        assert_eq!(config.build.footer_sort_by, "slug");
        assert_eq!(config.serve.edit, serve.edit);
        assert_eq!(config.serve.output, serve.output);
        assert!(!config.publish.rss);
    }

    #[test]
    fn test_simple_toml() {
        let serve = crate::config::Serve::default();
        let config = crate::config::parse_config(
            r#"
            [wanshi]
            trees = "source"
            assets = "assets"
            base-url = "https://example.com/"
            theme-lock = true

            [build]
            short-slug = true
            inline-css = true
            inline-script = true
            footer-sort-by = "title"

            [publish]
            rss = true
            "#,
        )
        .unwrap();

        assert_eq!(config.wanshi.trees, "source");
        assert_eq!(config.wanshi.assets, "assets");
        assert_eq!(config.wanshi.base_url, "https://example.com/");
        assert!(config.wanshi.theme_lock);
        assert!(config.build.short_slug);
        assert!(config.build.inline_css);
        assert!(config.build.inline_script);
        assert_eq!(config.build.footer_sort_by, "title");
        assert_eq!(config.serve.edit, serve.edit);
        assert_eq!(config.serve.output, serve.output);
        assert!(config.publish.rss);
    }

    // Regression test: URL builders concatenate paths directly onto
    // `base-url`, so a slash-less value corrupted every generated URL.
    #[test]
    fn test_base_url_gains_a_trailing_slash() {
        let config = crate::config::parse_config(
            r#"
            [wanshi]
            base-url = "https://example.com"
            "#,
        )
        .unwrap();
        assert_eq!(config.wanshi.base_url, "https://example.com/");

        // An already-correct value is left alone.
        let config = crate::config::parse_config("").unwrap();
        assert_eq!(config.wanshi.base_url, "/");
    }

    // Regression test: `Serve` was the one config struct without
    // `#[serde(default)]` on the container, so a `[serve]` table overriding a
    // single key failed every command with `missing field`.
    #[test]
    fn test_partial_serve_table_fills_in_defaults() {
        let serve = crate::config::Serve::default();
        let config = crate::config::parse_config(
            r#"
            [serve]
            edit = "zed://file/"
            "#,
        )
        .unwrap();

        assert_eq!(config.serve.edit.as_deref(), Some("zed://file/"));
        assert_eq!(config.serve.output, serve.output);
        assert_eq!(config.serve.command, serve.command);
    }
}
