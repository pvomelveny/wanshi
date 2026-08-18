// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Build {
    pub typst_root: String,
    pub short_slug: bool,
    pub pretty_urls: bool,
    pub footer_mode: FooterMode,
    pub footer_sort_by: String,
    pub inline_css: bool,
    pub inline_script: bool,
    pub asref: bool,
    pub output: String,
    pub edit: Option<String>,
    pub search: bool,
    pub search_content: bool,
    pub header_keys: Vec<String>,
}

impl Default for Build {
    fn default() -> Self {
        Self {
            typst_root: "trees".to_string(),
            short_slug: false,
            pretty_urls: false,
            footer_mode: FooterMode::default(),
            footer_sort_by: "slug".to_string(),
            inline_css: false,
            inline_script: false,
            asref: false,
            output: "./publish".to_string(),
            edit: None,
            search: true,
            // Body text is indexed as a token -> sections map, never as stored
            // prose, so enabling it by default does not duplicate the notes.
            search_content: true,
            // Metadata is data first and chrome second. Custom keys are
            // rendered as bare values with no key name, which is only legible
            // to whoever wrote them, so a note shows nothing under its title
            // unless the site asks for it. These three read as themselves: a
            // date, a name, and a link the reader can follow.
            header_keys: vec![
                "date".to_string(),
                "author".to_string(),
                "see-also".to_string(),
            ],
        }
    }
}

#[derive(Debug, Copy, Clone, clap::ValueEnum, Default, Deserialize, Serialize)]
pub enum FooterMode {
    #[default]
    #[serde(rename = "link")]
    Link,

    #[serde(rename = "embed")]
    Embed,
}

#[derive(Debug)]
pub struct ParseFooterModeError;

impl FromStr for FooterMode {
    type Err = ParseFooterModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "link" => Ok(FooterMode::Link),
            "embed" => Ok(FooterMode::Embed),
            _ => Err(ParseFooterModeError),
        }
    }
}

impl std::fmt::Display for FooterMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FooterMode::Link => write!(f, "link"),
            FooterMode::Embed => write!(f, "embed"),
        }
    }
}
