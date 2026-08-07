// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use eyre::eyre;
use regex_lite::Regex;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, mem, sync::LazyLock};

use crate::{
    entry::{EntryMetaData, HTMLMetaData, MetaData},
    slug::Slug,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionOption {
    pub numbering: bool, // default: false

    /// Display children catalog
    pub details_open: bool, // default: true

    /// Display in catalog
    pub catalog: bool, // default: true
}

impl Default for SectionOption {
    fn default() -> Self {
        SectionOption::new(false, true, true)
    }
}

impl SectionOption {
    pub fn new(numbering: bool, details_open: bool, catalog: bool) -> SectionOption {
        SectionOption {
            numbering,
            details_open,
            catalog,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedContent {
    pub url: String,
    pub title: Option<String>,
    pub option: SectionOption,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalLink {
    pub url: String,
    pub text: Option<String>,
}

/// Which sections a [`QuerySpec`] draws from.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QueryScope {
    /// Sections whose parent is the querying section.
    Children,
    /// The whole subtree beneath the querying section.
    Descendants,
    /// Sections sharing the querying section's parent.
    Siblings,
    /// Every visible section.
    All,
    /// Sections nothing links to and nothing embeds.
    Orphans,
    /// Every section whose slug starts with this prefix.
    Prefix(String),
}

impl QueryScope {
    pub fn parse(raw: &str) -> QueryScope {
        match raw {
            "children" => QueryScope::Children,
            "descendants" => QueryScope::Descendants,
            "siblings" => QueryScope::Siblings,
            "all" => QueryScope::All,
            "orphans" => QueryScope::Orphans,
            prefix => QueryScope::Prefix(prefix.trim_start_matches('/').to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum QueryOrder {
    Ascending,
    Descending,
}

impl QueryOrder {
    pub fn parse(raw: &str) -> QueryOrder {
        match raw {
            "desc" | "descending" => QueryOrder::Descending,
            _ => QueryOrder::Ascending,
        }
    }
}

/// A listing of other sections, resolved once the whole graph is known.
///
/// The querying section's slug is captured at parse time so that resolution
/// stays correct no matter where the compiled section is later cloned to — an
/// embedded copy must still list *its* children, not its host's.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuerySpec {
    pub owner: Slug,
    pub scope: QueryScope,
    pub taxon: Option<String>,
    pub key: Option<String>,
    pub value: Option<String>,
    pub sort: String,
    pub order: QueryOrder,
    pub limit: Option<usize>,
    pub title: Option<String>,
}

/// Plain HTMLs & lazy embedding HTMLs, This means that
/// the embedded structure within are not expanded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LazyContent {
    Plain(String),
    Embed(EmbedContent),
    Local(LocalLink),
    Query(QuerySpec),
}

pub type LazyContents = Vec<LazyContent>;

/// The purpose of this structure is to handle cases like [`LocalLink`],
/// where full information cannot be directly obtained during the parsing stage.
///
/// Additionally, it is designed with the consideration that
/// when all contents in `Vec<LazyContent>` are [`LazyContent::Plain`],
/// this structure will naturally be lifted to [`HTMLContent::Plain`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HTMLContent {
    Plain(String),
    Lazy(LazyContents),
}

impl HTMLContent {
    pub fn as_string(&self) -> Option<&String> {
        if let HTMLContent::Plain(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn remove_all_tags(&self) -> String {
        static RE_TAGS: LazyLock<Regex> = LazyLock::new(|| {
            let attrs = r#"(\s+[a-zA-Z-]+(="([^"\\]|\\[\s\S])*")?)*"#;
            Regex::new(&format!(r#"<[A-Za-z]+{}\s*/?>|</[A-Za-z]+>"#, attrs)).unwrap()
        });

        let remove_tag = |s| {
            let mut cursor = 0;
            let mut string = String::new();
            for capture in RE_TAGS.captures_iter(s) {
                let all = capture.get(0).unwrap();
                string.push_str(&s[cursor..all.start()]);
                cursor = all.end();
            }
            string.push_str(&s[cursor..]);
            string
        };

        match self {
            HTMLContent::Plain(s) => remove_tag(s),
            HTMLContent::Lazy(contents) => {
                let mut str = String::new();
                for content in contents {
                    let s = match content {
                        LazyContent::Plain(s) => remove_tag(s),
                        LazyContent::Embed(embed) => embed
                            .title
                            .as_ref()
                            .map(|s| remove_tag(s))
                            .unwrap_or_default(),

                        LazyContent::Local(local) => local
                            .text
                            .as_ref()
                            .map(|s| remove_tag(s))
                            .unwrap_or_default(),
                        // A listing contributes no text of its own before it is
                        // resolved against the graph.
                        LazyContent::Query(_) => String::new(),
                    };
                    str.push_str(&s);
                }
                str
            }
        }
    }
}

pub struct HTMLContentBuilder {
    contents: LazyContents,
    content: String,
}

impl HTMLContentBuilder {
    pub fn new() -> HTMLContentBuilder {
        HTMLContentBuilder {
            contents: vec![],
            content: String::new(),
        }
    }
    pub fn push_str(&mut self, s: &str) {
        if !s.is_empty() {
            self.content.push_str(s);
        }
    }
    fn push_content(&mut self) {
        if !self.content.is_empty() {
            self.contents
                .push(LazyContent::Plain(mem::take(&mut self.content)));
        }
    }
    pub fn push(&mut self, c: LazyContent) {
        match c {
            LazyContent::Plain(s) => {
                self.push_str(&s);
            }
            _ => {
                self.push_content();
                self.contents.push(c);
            }
        }
    }
    pub fn build(mut self) -> HTMLContent {
        if self.contents.is_empty() {
            return HTMLContent::Plain(mem::take(&mut self.content));
        }
        self.push_content();
        HTMLContent::Lazy(self.contents)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnresolvedSection {
    pub metadata: HTMLMetaData,
    pub content: HTMLContent,
}

impl UnresolvedSection {
    pub fn slug(&self) -> eyre::Result<Slug> {
        self.metadata
            .slug()
            .ok_or_else(|| eyre!("missing required metadata `slug` in section"))
    }

    pub fn ext(&self) -> eyre::Result<&str> {
        self.metadata.ext().map(String::as_str).ok_or_else(|| {
            eyre!(
                "missing required metadata `ext` in section. Please delete the expired `.cache` folder"
            )
        })
    }
}

pub type SectionContents = Vec<SectionContent>;

#[derive(Debug, Clone)]
pub enum SectionContent {
    Plain(String),
    Embed(Section),
    /// Placeholder swapped for rendered HTML after graph compilation finishes.
    Query(QuerySpec),
}

#[derive(Debug, Clone)]
pub struct Section {
    pub metadata: EntryMetaData,
    pub children: SectionContents,
    pub option: SectionOption,
    pub references: HashSet<Slug>,
}

impl Section {
    pub fn new(
        metadata: EntryMetaData,
        children: SectionContents,
        references: HashSet<Slug>,
    ) -> Section {
        Section {
            metadata,
            children,
            option: SectionOption::new(false, true, true),
            references,
        }
    }

    pub fn slug(&self) -> eyre::Result<Slug> {
        self.metadata
            .slug()
            .ok_or_else(|| eyre!("missing required metadata `slug` in compiled section"))
    }

    pub fn spanned(&self) -> String {
        let mut html = String::new();
        for content in &self.children {
            match content {
                SectionContent::Plain(text) => html.push_str(text),
                // Metadata values reject embeds and listings at parse time.
                SectionContent::Embed(_) | SectionContent::Query(_) => unreachable!(),
            }
        }
        html
    }
}
