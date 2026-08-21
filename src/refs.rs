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
                Ok(Self {
                    library,
                    raw,
                    is_biblatex: true,
                })
            }
            Some("yaml") | Some("yml") => {
                let library = hayagriva::io::from_yaml_str(&raw)
                    .wrap_err_with(|| format!("failed to parse Hayagriva bibliography `{path}`"))?;
                Ok(Self {
                    library,
                    raw,
                    is_biblatex: false,
                })
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
/// **Case is preserved.** A citation key is what the author reads in their
/// reference manager and types into `#local("/refs/…")`, so the slug has to be
/// the key itself wherever it legally can be. Lowercasing looked tidier and was
/// wrong: Better BibTeX generates keys like `odonnellAnalysisBooleanFunctions2021`,
/// and folding the case meant the link an author naturally wrote never matched
/// the file generated for it — the citation stayed dangling through a
/// successful sync.
///
/// Only characters that cannot sit in a path segment are replaced. The original
/// key is kept in the stub's `citekey` metadata regardless, since that is what
/// `refs export` joins on.
pub fn slug_segment(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    let mut last_was_dash = false;
    for ch in key.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
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
///
/// Written the way a reference list writes it — `_The Computer Journal_ *27*(2),
/// 97–111.` — rather than with `vol.`/`pp.` labels, so a generated stub reads
/// like the hand-written ones it sits beside in a References footer.
///
/// The year is deliberately absent. It is already in the title and again in the
/// date column, and a third copy at the end of this line was the most visible
/// thing on the page that did not need to be there.
fn reference_line(entry: &Entry) -> Option<String> {
    let mut line = String::new();

    if let Some(container) = container(entry) {
        line.push_str(&format!("_{container}_"));
    } else if let Some(name) = entry.publisher().and_then(|publisher| publisher.name()) {
        line.push_str(&name.to_string());
    }

    if let Some(volume) = volume(entry) {
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(&format!("*{volume}*"));
    }
    if let Some(issue) = issue(entry) {
        line.push_str(&format!("({issue})"));
    }

    if let Some(pages) = entry.page_range() {
        if !line.is_empty() {
            line.push_str(", ");
        }
        line.push_str(&en_dash_range(&pages.to_string()));
    }

    if line.is_empty() {
        None
    } else {
        Some(format!("{line}."))
    }
}

/// A work's volume, which for a journal article belongs to the periodical.
///
/// Hayagriva models a journal article as a child of its periodical and files
/// volume and issue on that parent, so reading only `entry.volume()` finds a
/// volume on a book — where it rarely means anything — and misses it on exactly
/// the entry type that needs one.
fn volume(entry: &Entry) -> Option<String> {
    inherited(entry, |entry| entry.volume().map(|v| v.to_string()))
}

fn issue(entry: &Entry) -> Option<String> {
    inherited(entry, |entry| entry.issue().map(|v| v.to_string()))
}

/// Read a field from the entry, falling back to the first parent that has it.
fn inherited(entry: &Entry, read: impl Fn(&Entry) -> Option<String> + Copy) -> Option<String> {
    read(entry).or_else(|| entry.parents().iter().find_map(read))
}

/// `97-111` becomes `97–111`.
///
/// BibTeX writes the range as `97--111` and hayagriva normalises it to a single
/// ASCII hyphen; a reference list uses an en dash, which is what the
/// hand-written notes in the demo already do.
fn en_dash_range(pages: &str) -> String {
    let mut out = String::with_capacity(pages.len());
    let mut chars = pages.chars().peekable();
    let mut previous: Option<char> = None;
    while let Some(ch) = chars.next() {
        let between_digits = ch == '-'
            && previous.is_some_and(|p| p.is_ascii_digit())
            && chars.peek().is_some_and(char::is_ascii_digit);
        out.push(if between_digits { '–' } else { ch });
        previous = Some(ch);
    }
    out
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
    fn test_slug_segment_preserves_case_and_replaces_unsafe_characters() {
        // Case must survive: a Better BibTeX key is what the author types into
        // `#local("/refs/…")`, and folding it would leave that link dangling
        // through a successful sync.
        assert_eq!(
            slug_segment("odonnellAnalysisBooleanFunctions2021"),
            "odonnellAnalysisBooleanFunctions2021"
        );
        assert_eq!(slug_segment("kkl1988"), "kkl1988");
        assert_eq!(slug_segment("Kahn:1988aa"), "Kahn-1988aa");
        assert_eq!(slug_segment("a++b"), "a-b");
        assert_eq!(slug_segment("--trim--"), "trim");
        assert_eq!(slug_segment("with_underscore"), "with_underscore");
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
        assert!(
            entry.contains("{Boolean}"),
            "nested braces survive: {entry}"
        );
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

    const JOURNAL: &str = r#"
@article{knuth1984literate,
  title     = {Literate Programming},
  author    = {Knuth, Donald E.},
  journal   = {The Computer Journal},
  volume    = {27},
  number    = {2},
  pages     = {97--111},
  year      = {1984},
  publisher = {Oxford University Press},
}

@book{book1999,
  title     = {A Book},
  author    = {Writer, Ann},
  publisher = {A Press},
  year      = {1999},
}

@inproceedings{proc2001,
  title     = {A Paper},
  author    = {Speaker, Bo},
  booktitle = {Proceedings of Somewhere},
  pages     = {5--9},
  year      = {2001},
}
"#;

    fn line(key: &str) -> String {
        let library = hayagriva::io::from_biblatex_str(JOURNAL).expect("sample parses");
        let entry = library.get(key).expect("entry");
        reference_line(entry).expect("a line")
    }

    /// Hayagriva files a journal article's volume and issue on the periodical
    /// parent, so reading them off the entry found them on a book and lost them
    /// on the article — the one entry type where they matter.
    #[test]
    fn test_a_journal_article_keeps_its_volume_and_issue() {
        assert_eq!(
            line("knuth1984literate"),
            "_The Computer Journal_ *27*(2), 97–111."
        );
    }

    #[test]
    fn test_a_book_names_its_publisher_and_nothing_it_does_not_have() {
        assert_eq!(line("book1999"), "A Press.");
    }

    #[test]
    fn test_proceedings_read_as_a_container_with_pages() {
        assert_eq!(line("proc2001"), "_Proceedings of Somewhere_, 5–9.");
    }

    /// The year is in the title and in the date column already; a third copy at
    /// the end of the line was the most visible redundancy on a stub page.
    #[test]
    fn test_the_reference_line_does_not_repeat_the_year() {
        for key in ["knuth1984literate", "book1999", "proc2001"] {
            let line = line(key);
            assert!(!line.contains("1984"), "{line}");
            assert!(!line.contains("1999"), "{line}");
            assert!(!line.contains("2001"), "{line}");
        }
    }

    #[test]
    fn test_en_dash_range_only_touches_a_hyphen_between_digits() {
        assert_eq!(en_dash_range("97-111"), "97–111");
        assert_eq!(en_dash_range("97"), "97");
        assert_eq!(en_dash_range("A-1"), "A-1");
        assert_eq!(en_dash_range("12-"), "12-");
    }

    /// An entry with no container, publisher, volume or pages has no line to
    /// write, and an empty paragraph under the title is worse than none.
    #[test]
    fn test_an_entry_with_no_detail_has_no_line() {
        let library =
            hayagriva::io::from_biblatex_str("@misc{bare, title = {Bare}, year = {2020}}")
                .expect("parses");
        assert!(reference_line(library.get("bare").expect("entry")).is_none());
    }
}
