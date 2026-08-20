// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.

//! Bibliographic references: works, and the note stubs that stand for them.
//!
//! A citation in a forest is an ordinary link — `#local("/refs/kkl1988")` — so
//! citing a paper produces the same two graph edges as citing a note: a
//! `references` entry on the citing note and a backlink on the work. That is
//! the point. It makes the forest able to answer "which works do these notes
//! depend on", which is the question a reference manager cannot.
//!
//! What the forest is *not* is a reference manager. Bibliographic data is
//! authored elsewhere — Zotero, or any tool that writes BibTeX or Hayagriva —
//! and read here. A stub is generated from that data and freely regenerable, so
//! nothing is ever retyped and the two cannot drift. Notes *about* a work
//! belong in an ordinary note that links to its stub, where the stub's
//! backlinks then collect them.

use camino::Utf8Path;
use eyre::{eyre, Context};
use hayagriva::types::EntryType;
use hayagriva::{Entry, Library};

/// A bibliography, plus the text it was parsed from.
///
/// The raw text is kept so that [`Bibliography::verbatim`] can hand back an
/// entry exactly as written. Exporting a subset for a paper must not launder
/// it through this crate's data model: hayagriva parses *from* BibTeX but
/// serialises only to Hayagriva YAML, so a round trip would silently drop any
/// field it does not model.
pub struct Bibliography {
    library: Library,
    raw: String,
    is_biblatex: bool,
}

impl Bibliography {
    /// Read a bibliography, dispatching on the file extension.
    ///
    /// `.bib` is BibTeX/BibLaTeX, `.yaml`/`.yml` is Hayagriva — the same pair
    /// Typst's own `bibliography()` accepts, so one file can serve both this
    /// and a note that wants Typst-native citations.
    pub fn load(path: &Utf8Path) -> eyre::Result<Self> {
        let raw = std::fs::read_to_string(path)
            .wrap_err_with(|| format!("failed to read bibliography `{path}`"))?;

        match path.extension() {
            Some("bib") => {
                let library = hayagriva::io::from_biblatex_str(&raw).map_err(|errors| {
                    // A BibLaTeX parse yields every error at once; reporting one
                    // at a time would mean a fix-and-retry loop per entry.
                    let detail = errors
                        .iter()
                        .map(|error| format!("  {error:?}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    eyre!("failed to parse BibTeX bibliography `{path}`:\n{detail}")
                })?;
                Ok(Self { library, raw, is_biblatex: true })
            }
            Some("yaml") | Some("yml") => {
                let library = hayagriva::io::from_yaml_str(&raw)
                    .wrap_err_with(|| format!("failed to parse Hayagriva bibliography `{path}`"))?;
                Ok(Self { library, raw, is_biblatex: false })
            }
            other => Err(eyre!(
                "unsupported bibliography extension `{}` in `{}` (expected `bib`, `yaml` or `yml`)",
                other.unwrap_or("<none>"),
                path
            )),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Entry> {
        self.library.get(key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.library.keys()
    }

    /// The entry for `key` exactly as it appears in the source file.
    ///
    /// Only meaningful for BibTeX; Hayagriva YAML is re-serialised instead,
    /// since it has no comparable notion of an entry's own text span.
    pub fn verbatim(&self, key: &str) -> Option<&str> {
        if !self.is_biblatex {
            return None;
        }
        extract_biblatex_entry(&self.raw, key)
    }

    pub fn is_biblatex(&self) -> bool {
        self.is_biblatex
    }
}

/// Slice out `@type{key, … }` by counting braces from the opening one.
///
/// Brace counting rather than a regex because a BibTeX field is itself
/// brace-delimited and nests arbitrarily — `title = {The {LLL} algorithm}` ends
/// three braces deep. Quotes are not tracked: a `}` inside a quoted value is
/// still balanced in practice, and miscounting would only ever extend the slice
/// to the next entry, which the leading-`@` check below catches.
fn extract_biblatex_entry<'a>(raw: &'a str, key: &str) -> Option<&'a str> {
    let mut search_from = 0;
    while let Some(at) = raw[search_from..].find('@') {
        let start = search_from + at;
        let open = match raw[start..].find('{') {
            Some(offset) => start + offset,
            None => return None,
        };
        let found_key = raw[open + 1..]
            .split([',', '\n'])
            .next()
            .unwrap_or("")
            .trim();

        if found_key == key {
            let mut depth = 0usize;
            for (offset, ch) in raw[open..].char_indices() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(&raw[start..open + offset + 1]);
                        }
                    }
                    _ => {}
                }
            }
            return None;
        }
        search_from = open + 1;
    }
    None
}

/// Turn a citation key into a slug segment.
///
/// Keys are chosen by the reference manager and can carry characters a URL
/// should not — `:` and `+` appear in Better BibTeX keys. The original is kept
/// in the stub's `citekey` metadata, which is what `refs export` joins on, so
/// this only has to be stable and legible.
pub fn slug_segment(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    let mut last_was_dash = false;
    for ch in key.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash && !out.is_empty() {
            out.push('-');
            last_was_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// Escape a value for a double-quoted Typst string.
fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// `Kahn, Kalai & Linial` — family names only, which is what a title needs.
fn author_summary(entry: &Entry) -> Option<String> {
    let authors = entry.authors()?;
    if authors.is_empty() {
        return None;
    }
    let names: Vec<&str> = authors.iter().map(|person| person.name.as_str()).collect();
    Some(match names.as_slice() {
        [one] => one.to_string(),
        [first, second] => format!("{first} & {second}"),
        [rest @ .., last] => format!("{} & {}", rest.join(", "), last),
        [] => unreachable!("checked non-empty above"),
    })
}

/// Full author list, given names first, for the `author` metadata key.
fn author_list(entry: &Entry) -> Option<String> {
    let authors = entry.authors()?;
    if authors.is_empty() {
        return None;
    }
    Some(
        authors
            .iter()
            .map(|person| person.given_first(false))
            .collect::<Vec<_>>()
            .join(", "),
    )
}

fn year(entry: &Entry) -> Option<i32> {
    entry.date().map(|date| date.year)
}

/// The journal, proceedings or book a work appeared in.
///
/// Hayagriva models containment as parent entries rather than a field, so a
/// journal article's periodical is its parent.
fn container(entry: &Entry) -> Option<String> {
    entry
        .parents()
        .iter()
        .find_map(|parent| parent.title().map(|title| title.to_string()))
}

/// A citation key's stub, as Typst source.
///
/// Deliberately not CSL-formatted. The stub is a graph node carrying legible
/// bibliographic data; the publication-grade rendering happens in the paper,
/// from the verbatim entry `refs export` hands back. Adding CSL later changes
/// only this function — the metadata below is what everything else reads.
pub fn stub_source(entry: &Entry, parent_slug: &str) -> String {
    let key = entry.key();
    let title = entry.title().map(|title| title.to_string());

    // The title carries author and year so that a bare `#local()` — which
    // renders the target's title verbatim — already reads like a citation.
    let heading = match (author_summary(entry), year(entry), title.as_deref()) {
        (Some(authors), Some(year), Some(title)) => format!("{authors} {year} — {title}"),
        (Some(authors), None, Some(title)) => format!("{authors} — {title}"),
        (None, Some(year), Some(title)) => format!("{title} ({year})"),
        (_, _, Some(title)) => title.to_string(),
        (Some(authors), Some(year), None) => format!("{authors} {year}"),
        _ => key.to_string(),
    };

    let mut metadata = vec![
        ("title".to_string(), heading),
        ("taxon".to_string(), "reference".to_string()),
    ];

    // `date` is the bare year. wanshi cannot parse that as a date and falls
    // back to string comparison, which orders four-digit years correctly.
    if let Some(year) = year(entry) {
        metadata.push(("date".to_string(), year.to_string()));
    }
    if let Some(authors) = author_list(entry) {
        metadata.push(("author".to_string(), authors));
    }
    metadata.push(("citekey".to_string(), key.to_string()));

    // The CSL type goes in a custom key, not the taxon: `Taxon::is_reference`
    // matches by prefix but `#by-taxon` matches exactly, so `reference-article`
    // would be a citation target yet vanish from the bibliography listing.
    metadata.push(("type".to_string(), entry_type_name(entry.entry_type())));

    if let Some(container) = container(entry) {
        metadata.push(("container".to_string(), container));
    }
    if let Some(doi) = entry.doi() {
        metadata.push(("doi".to_string(), doi.to_string()));
    }
    // Declared explicitly: embedding a stub would otherwise reparent it under
    // the embedding note, filing a work under whichever note happened to quote
    // it.
    metadata.push(("parent".to_string(), parent_slug.to_string()));

    let fields = metadata
        .iter()
        .map(|(key, value)| format!("  \"{}\": \"{}\",\n", key, escape(value)))
        .collect::<String>();

    let mut body = String::new();
    if let Some(line) = reference_line(entry) {
        body.push_str(&line);
        body.push('\n');
    }
    if let Some(doi) = entry.doi() {
        body.push_str(&format!(
            "\n#external(\"https://doi.org/{}\", \"doi:{}\")\n",
            doi, doi
        ));
    } else if let Some(url) = entry.url() {
        body.push_str(&format!(
            "\n#external(\"{}\", \"{}\")\n",
            url.value, url.value
        ));
    }

    format!(
        "#import \"/_lib/wanshi.typ\": *\n\n\
         #show: wanshi\n\n\
         {STUB_MARKER}\n\
         #metadata((\n{fields}))\n\n{body}"
    )
}

/// Marks a stub as generated, and so safe to overwrite.
///
/// `refs sync` rewrites only files carrying this line. Removing it is the way
/// to take a stub over by hand and have the tool leave it alone.
pub const STUB_MARKER: &str =
    "// Generated by `wanshi refs sync`. Edits are overwritten — notes about\n\
     // this work belong in an ordinary note that links here. Delete this\n\
     // comment to take the file over by hand.";

/// One legible line of bibliographic detail beneath the title.
fn reference_line(entry: &Entry) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();

    if let Some(container) = container(entry) {
        parts.push(format!("_{}_", container));
    } else if let Some(name) = entry.publisher().and_then(|publisher| publisher.name()) {
        parts.push(name.to_string());
    }
    if let Some(volume) = entry.volume() {
        parts.push(format!("vol. {volume}"));
    }
    if let Some(pages) = entry.page_range() {
        parts.push(format!("pp. {pages}"));
    }
    if let Some(year) = year(entry) {
        parts.push(year.to_string());
    }

    if parts.is_empty() {
        None
    } else {
        Some(format!("{}.", parts.join(", ")))
    }
}

/// A stable name for an entry type, for the `type` metadata key.
///
/// Taken from the crate's own serde representation (kebab-case, matching CSL
/// and Hayagriva) rather than a hand-written match, which would silently drift
/// as upstream adds variants.
fn entry_type_name(entry_type: &EntryType) -> String {
    serde_json::to_value(entry_type)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| format!("{entry_type:?}").to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
@article{kkl1988,
  title = {The influence of variables on Boolean functions},
  author = {Kahn, Jeff and Kalai, Gil and Linial, Nathan},
  journal = {Proc. FOCS},
  year = {1988},
  pages = {68--80},
  doi = {10.1109/SFCS.1988.21923},
}

@book{odonnell2014,
  title = {Analysis of {Boolean} Functions},
  author = {O'Donnell, Ryan},
  publisher = {Cambridge University Press},
  year = {2014},
}
"#;

    fn library() -> Library {
        hayagriva::io::from_biblatex_str(SAMPLE).expect("sample parses")
    }

    #[test]
    fn test_slug_segment_sanitises_and_lowercases() {
        assert_eq!(slug_segment("kkl1988"), "kkl1988");
        assert_eq!(slug_segment("Kahn:1988aa"), "kahn-1988aa");
        assert_eq!(slug_segment("a++b"), "a-b");
        assert_eq!(slug_segment("--trim--"), "trim");
    }

    #[test]
    fn test_extract_biblatex_entry_returns_the_entry_verbatim() {
        let entry = extract_biblatex_entry(SAMPLE, "kkl1988").expect("found");
        assert!(entry.starts_with("@article{kkl1988,"));
        assert!(entry.ends_with('}'));
        assert!(entry.contains("10.1109/SFCS.1988.21923"));
        // Must not run on into the next entry.
        assert!(!entry.contains("odonnell"));
    }

    #[test]
    fn test_extract_biblatex_entry_handles_nested_braces() {
        let entry = extract_biblatex_entry(SAMPLE, "odonnell2014").expect("found");
        assert!(entry.contains("{Boolean}"), "nested braces survive: {entry}");
        assert!(entry.ends_with('}'));
    }

    #[test]
    fn test_extract_biblatex_entry_missing_key_is_none() {
        assert!(extract_biblatex_entry(SAMPLE, "nope").is_none());
    }

    #[test]
    fn test_stub_carries_the_citekey_and_reference_taxon() {
        let library = library();
        let entry = library.get("kkl1988").expect("entry");
        let stub = stub_source(entry, "refs/index");

        assert!(stub.contains(r#""taxon": "reference""#));
        assert!(stub.contains(r#""citekey": "kkl1988""#));
        assert!(stub.contains(r#""date": "1988""#));
        assert!(stub.contains(r#""parent": "refs/index""#));
        assert!(stub.contains("10.1109/SFCS.1988.21923"));
        assert!(stub.contains(STUB_MARKER));
        // The import must be root-absolute or the stub breaks at depth.
        assert!(stub.contains(r#"#import "/_lib/wanshi.typ": *"#));
    }

    #[test]
    fn test_stub_title_reads_as_a_citation() {
        let library = library();
        let entry = library.get("kkl1988").expect("entry");
        let stub = stub_source(entry, "refs/index");
        assert!(
            stub.contains("Kahn, Kalai & Linial 1988 —"),
            "title should carry authors and year: {stub}"
        );
    }

    #[test]
    fn test_stub_escapes_quotes_in_metadata() {
        let escaped = escape(r#"a "quoted" \ backslash"#);
        assert_eq!(escaped, r#"a \"quoted\" \\ backslash"#);
    }
}
