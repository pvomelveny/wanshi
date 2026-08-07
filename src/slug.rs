// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{fmt::Display, str::FromStr};

use camino::Utf8Path;
use internment::Intern;
use serde::{Deserialize, Serialize};

use crate::path_utils;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Slug(Intern<str>);

impl Slug {
    pub fn new<S: AsRef<str>>(s: S) -> Self {
        Self(s.as_ref().into())
    }

    pub fn as_str(&self) -> &'static str {
        self.0.as_ref()
    }
}

impl PartialEq<&str> for Slug {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<Slug> for &str {
    fn eq(&self, other: &Slug) -> bool {
        *self == other.as_str()
    }
}

impl PartialEq<Slug> for String {
    fn eq(&self, other: &Slug) -> bool {
        self == other.as_str()
    }
}

impl Serialize for Slug {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.0.as_ref())
    }
}

impl<'de> Deserialize<'de> for Slug {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Slug::new)
    }
}

impl Display for Slug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Recognized source-file extensions. This enum is the single source of truth
/// for which files are section sources; anything that needs to classify a path
/// should parse into it rather than compare extension strings itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ext {
    /// Typst's standard extension, and the default for new notes.
    Typ,

    /// Accepted for compatibility with sites created before `.typ` was adopted.
    Typst,
}

impl Ext {
    /// Whether `ext` names a section source extension.
    pub fn is_source_extension(ext: Option<&str>) -> bool {
        ext.is_some_and(|ext| ext.parse::<Ext>().is_ok())
    }
}

#[derive(Debug)]
pub struct ParseExtensionError;

impl FromStr for Ext {
    type Err = ParseExtensionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "typ" => Ok(Self::Typ),
            "typst" => Ok(Self::Typst),
            _ => Err(ParseExtensionError),
        }
    }
}

impl Display for Ext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Ext::Typ => "typ",
            Ext::Typst => "typst",
        };
        write!(f, "{s}")
    }
}

pub fn to_hash_id(slug_str: &str) -> String {
    if let Some((prefix, last)) = slug_str.rsplit_once('/') {
        if last.starts_with(':') {
            let mut hash_id = prefix.replace('/', "-");
            hash_id.push_str(last);
            return hash_id;
        }
    }
    slug_str.replace('/', "-")
}

pub fn to_slug<P: AsRef<Utf8Path>>(path: P) -> Slug {
    let path = path.as_ref();
    let normalized = path_utils::pretty_path(path);
    let stripped = match path.extension().and_then(|ext| ext.parse::<Ext>().ok()) {
        // Any recognized source extension is stripped, so a slug is stable
        // across a rename between source extensions.
        Some(_) => path_utils::pretty_path(&path.with_extension("")),
        None => normalized,
    };
    Slug::new(stripped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_hash_id_preserves_anonymous_suffix_separator() {
        assert_eq!(
            to_hash_id("daily-surf/windows-skill/:0"),
            "daily-surf-windows-skill:0"
        );
    }

    #[test]
    fn test_to_hash_id_replaces_slashes_for_regular_slug() {
        assert_eq!(
            to_hash_id("daily-surf/windows-skill"),
            "daily-surf-windows-skill"
        );
    }
    #[test]
    fn test_to_slug_strips_known_source_extension() {
        assert_eq!(to_slug("a/b/c.typst"), Slug::new("a/b/c"));
    }

    #[test]
    fn test_to_slug_keeps_dot_segments_without_known_extension() {
        assert_eq!(to_slug("a.b"), Slug::new("a.b"));
        assert_eq!(to_slug("a.b/c.d"), Slug::new("a.b/c.d"));
        assert_eq!(to_slug("a.b.md"), Slug::new("a.b.md"));
    }

    #[test]
    fn test_is_source_extension_matches_parseable_extensions_only() {
        assert!(Ext::is_source_extension(Some("typst")));
        assert!(!Ext::is_source_extension(Some("md")));
        assert!(!Ext::is_source_extension(Some("svg")));
        assert!(!Ext::is_source_extension(None));
    }

    /// Anything that records an extension and later rebuilds a source path
    /// (edit links, cache keys) depends on this round trip. Add a case here for
    /// every new [`Ext`] variant.
    #[track_caller]
    fn assert_ext_round_trips(ext: Ext) {
        let parsed: Ext = ext
            .to_string()
            .parse()
            .expect("Display output must parse back");
        assert_eq!(parsed, ext);
    }

    #[test]
    fn test_ext_display_round_trips_through_parsing() {
        assert_ext_round_trips(Ext::Typst);
    }
}
