// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Taxon {
    pub numbering: Option<String>,
    pub text: String,
}

impl std::fmt::Debug for Taxon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("\"{}\"", self.display()))
    }
}

impl Taxon {
    pub fn new(numbering: Option<String>, text: String) -> Taxon {
        Taxon { numbering, text }
    }

    /// Format for display. `self.text` is the taxon as authored, so this
    /// capitalises it and either appends the numbering or the ". " separator.
    pub fn display(&self) -> String {
        match &self.numbering {
            Some(numbering) => format!("{} {} ", capitalize(&self.text), numbering),
            None => display_taxon(&self.text),
        }
    }

    /// Whether a taxon marks its section as a citation target.
    ///
    /// A prefix test, not equality: `reference`, `references` and
    /// `reference-book` all qualify.
    pub fn is_reference(s: &str) -> bool {
        s.to_lowercase().starts_with("reference") || s.starts_with("参考")
    }
}

/// Uppercase the first character, leaving the rest alone.
fn capitalize(s: &str) -> String {
    match s.split_at_checked(1) {
        Some((first, rest)) => first.to_uppercase() + rest,
        _ => s.to_string(),
    }
}

/// Format an authored taxon for display: capitalised, with a ". " separator.
///
/// This is applied when a taxon is *rendered*. The metadata keeps the authored
/// string, so `data-taxon`, sorting, and `wanshi.json` all see what the author
/// wrote. Previously the display form was stored instead and the bare value
/// recovered by truncating at the first ".", which destroyed any taxon
/// containing one — "fig. 3 caption" came back as "Fig".
pub fn display_taxon(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    format!("{}. ", capitalize(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_taxon_capitalizes_and_appends_separator() {
        assert_eq!(display_taxon("definition"), "Definition. ");
    }

    #[test]
    fn test_display_taxon_leaves_an_already_capital_taxon_alone() {
        assert_eq!(display_taxon("Theorem"), "Theorem. ");
    }

    #[test]
    fn test_display_taxon_preserves_a_taxon_containing_a_dot() {
        // The whole point of formatting at render time: the authored value is
        // never round-tripped through a form that has to be parsed back.
        assert_eq!(display_taxon("fig. 3 caption"), "Fig. 3 caption. ");
    }

    #[test]
    fn test_display_taxon_of_empty_is_empty() {
        // An empty taxon must not render as a bare ". " pill.
        assert_eq!(display_taxon(""), "");
    }

    #[test]
    fn test_display_taxon_handles_multibyte_first_char() {
        assert_eq!(display_taxon("参考"), "参考. ");
    }

    #[test]
    fn test_taxon_display_with_numbering_capitalizes_and_appends_number() {
        let t = Taxon::new(Some("1.2".to_string()), "theorem".to_string());
        assert_eq!(t.display(), "Theorem 1.2 ");
    }

    #[test]
    fn test_taxon_display_without_numbering_matches_display_taxon() {
        let t = Taxon::new(None, "lemma".to_string());
        assert_eq!(t.display(), display_taxon("lemma"));
    }

    #[test]
    fn test_is_reference_matches_by_prefix_and_ignores_case() {
        assert!(Taxon::is_reference("reference"));
        assert!(Taxon::is_reference("References"));
        assert!(Taxon::is_reference("reference-book"));
        assert!(Taxon::is_reference("参考文献"));
        assert!(!Taxon::is_reference("definition"));
    }
}
