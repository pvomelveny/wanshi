// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.

//! Configuration for bibliographic references.
//!
//! A *reference* in wanshi is an ordinary note that link targets are filed
//! under (see [`crate::compiler::taxon::Taxon::is_reference`]). This section
//! adds the other half: where the bibliographic data for those notes comes
//! from, so that citing a paper does not mean retyping its details.

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Refs {
    /// Bibliography to read works from, resolved against the project root.
    ///
    /// A BibTeX/BibLaTeX `.bib` or a Hayagriva `.yaml`/`.yml`, dispatched on
    /// the extension. The default sits inside the source tree on purpose: a
    /// `_`-prefixed directory is skipped by section discovery, and only `.typ`
    /// and `.typst` are source extensions, so the file can never become a page
    /// — yet `[build].typst-root` is the tree, so a note that wants Typst's own
    /// citations can still reach it as `#bibliography("/_bib/refs.bib")`.
    pub bibliography: String,

    /// Slug prefix under the source tree where generated stubs live.
    ///
    /// One flat directory, not a pattern: a citation key identifies one work,
    /// so a work gets one page. Filing works under the topic that cites them
    /// would force a choice the first time a second topic cites the same one,
    /// and the topical view already exists in the graph as that note's
    /// backlinks.
    pub dir: String,
}

impl Default for Refs {
    fn default() -> Self {
        Self {
            bibliography: "trees/_bib/refs.bib".to_string(),
            dir: "refs".to_string(),
        }
    }
}
