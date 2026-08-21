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
    // The URL is kept as metadata as well as rendered, so anything reading
    // `wanshi.json` can reach the work without re-parsing the bibliography.
    if let Some(url) = entry.url() {
        metadata.push(("url".to_string(), url.value.to_string()));
    }
    if let Some(isbn) = entry.isbn() {
        metadata.push(("isbn".to_string(), isbn.to_string()));
    }
    // Declared explicitly: embedding a stub would otherwise reparent it under
    // the embedding note, filing a work under whichever note happened to quote
    // it.
    metadata.push(("parent".to_string(), parent_slug.to_string()));

    let fields = metadata
        .iter()
        .map(|(key, value)| format!("  \"{}\": \"{}\",\n", key, escape(value)))
        .collect::<String>();

    let mut body = full_reference(entry);
    body.push('\n');

    // Links get their own line, separated by a middot: they are a row of
    // addresses rather than part of the sentence above.
    let links = work_links(entry);
    if !links.is_empty() {
        let rendered = links
            .iter()
            .map(|(url, label)| format!("#external(\"{}\", \"{}\")", escape(url), escape(label)))
            .collect::<Vec<_>>()
            .join(" · ");
        body.push_str(&format!("\n{rendered}\n"));
    }

    // An ISBN is not a link, but it is how a book is ordered or found in a
    // library catalogue, so it belongs beside the links rather than buried in
    // metadata.
    if let Some(isbn) = entry.isbn() {
        body.push_str(&format!("\n#emph[ISBN {}]\n", escape_markup(isbn)));
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
/// The arXiv identifier a URL or DOI points at.
///
/// Recognised in both forms because Zotero records both: the abs/pdf URL, and
/// the DataCite DOI `10.48550/arXiv.<id>` that arXiv now mints for every
/// posting. They address the same page, so knowing they are the same is what
/// keeps a preprint from listing two links to itself.
fn arxiv_id(value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    let rest = ["arxiv.org/abs/", "arxiv.org/pdf/", "10.48550/arxiv."]
        .iter()
        .find_map(|marker| lower.split(marker).nth(1))?;

    // Trim a version suffix and anything after the identifier: `2105.10386v2`
    // and `2105.10386.pdf` are the same paper as `2105.10386`.
    let id: String = rest
        .chars()
        .take_while(|ch| ch.is_ascii_digit() || *ch == '.' || *ch == '/')
        .collect();
    let id = id.trim_end_matches('.').to_string();
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

/// A readable label for a bare URL: its host, without `www.`.
///
/// Publisher URLs are frequently enormous — a Cambridge Core book link runs
/// past 90 characters of opaque hash — and printing one in full makes a stub
/// unreadable while telling a reader nothing. The host is the part that says
/// where the link goes.
fn host_label(url: &str) -> String {
    let without_scheme = url
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(url);
    let host = without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(without_scheme);
    host.trim_start_matches("www.").to_string()
}

/// Every way to reach the work, most recognisable first.
///
/// A stub whose whole job is to stand for a work should say where the work is.
/// Both a DOI and a URL are kept when they lead to different places, and
/// neither is dropped for the other -- 57 of the 63 entries in the library this
/// was built against carry a URL, so preferring the DOI silently discarded the
/// only link for most of them.
fn work_links(entry: &Entry) -> Vec<(String, String)> {
    let doi = entry.doi().map(|doi| doi.to_string());
    let url = entry.url().map(|url| url.value.to_string());

    let mut links = Vec::new();

    // An arXiv posting is reachable by DOI and by URL, and the identifier a
    // reader actually recognises is neither -- it is the arXiv id.
    let arxiv = url
        .as_deref()
        .and_then(arxiv_id)
        .or_else(|| doi.as_deref().and_then(arxiv_id));
    if let Some(id) = &arxiv {
        links.push((format!("https://arxiv.org/abs/{id}"), format!("arXiv:{id}")));
    }

    if let Some(doi) = &doi {
        // A DataCite arXiv DOI resolves to the link already listed above.
        if arxiv_id(doi).is_none() {
            links.push((format!("https://doi.org/{doi}"), format!("doi:{doi}")));
        }
    }

    if let Some(url) = &url {
        let is_the_arxiv_link = arxiv.is_some() && arxiv_id(url).is_some();
        // Zotero sometimes stores the DOI resolver itself as the URL.
        let is_the_doi_link = doi
            .as_ref()
            .is_some_and(|doi| url.contains(doi.as_str()));
        if !is_the_arxiv_link && !is_the_doi_link {
            links.push((url.clone(), host_label(url)));
        }
    }

    links
}

/// Upper-case the first character, leaving the rest alone.
fn capitalize_first(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// The entry type as a display word: `Report`, `Thesis`, `Chapter`.
fn type_label(entry: &Entry) -> String {
    capitalize_first(&entry_type_name(entry.entry_type()))
}

/// Where the work appeared, as one sentence.
///
/// Always returns something. The previous version returned `None` whenever an
/// entry had no container, volume or pages, which is every thesis, report and
/// preprint -- so the works least likely to be recognised from their title
/// alone were exactly the ones whose stub said nothing at all. Naming the kind
/// of thing it is beats an empty line.
/// Whether the work is bound inside a larger one.
///
/// "In" belongs to a chapter or a paper in a proceedings, and not to a journal
/// article, which is cited as the periodical and volume alone. The distinction
/// is the *parent's* type, not the entry's: hayagriva files both `@article` and
/// `@inproceedings` as `Article`, and they differ only in what they sit inside.
fn bound_inside(entry: &Entry) -> bool {
    entry.parents().iter().any(|parent| {
        matches!(
            parent.entry_type(),
            EntryType::Proceedings | EntryType::Book | EntryType::Anthology | EntryType::Reference
        )
    })
}

/// The work's own title, marked up by whether it stands alone.
///
/// The convention every reference style shares: a work published *inside*
/// another is quoted and its container italicised, while a work that is itself
/// the publication is italicised. So an article is `"Title." _Journal_` and a
/// book is `_Title_` -- which is also what tells a reader, at a glance, which
/// kind of thing they are looking at.
fn title_markup(entry: &Entry) -> Option<String> {
    let title = escape_markup(&entry.title()?.to_string());
    Some(if container(entry).is_some() {
        format!("\"{title}\"")
    } else {
        format!("_{title}_")
    })
}

/// Where the work appeared: venue, volume, pages, imprint.
///
/// Empty when the entry records none of it. The caller decides what to do with
/// that -- [`full_reference`] falls back to naming the kind of work, so a
/// thesis or preprint still reads as a citation rather than trailing off.
fn publication_details(entry: &Entry) -> String {
    // Comma-separated clauses describing the work's place of publication.
    let mut clauses: Vec<String> = Vec::new();

    let container = container(entry);

    // A publisher's name already says "this is a book", so labelling it as one
    // too reads as filler: "Book. Cambridge University Press, Cambridge."
    let named_publisher = entry
        .publisher()
        .and_then(|publisher| publisher.name())
        .is_some();
    let implied_by_imprint = named_publisher && matches!(entry.entry_type(), EntryType::Book);

    // The opening clause names the venue, or failing that the kind of work.
    // Something has to open it: the clauses that follow are lowercase
    // continuations, so without an opening the citation reads
    // "_Whitney Numbers_. ed. Neil White" -- a fragment starting a sentence.
    let mut opening = match &container {
        Some(title) if bound_inside(entry) => format!("In _{}_", escape_markup(title)),
        Some(title) => format!("_{}_", escape_markup(title)),
        None if implied_by_imprint => String::new(),
        None => type_label(entry),
    };

    // A volume number modifies a named venue -- `_Annals_ *192*(3)` -- so with
    // no venue to attach to it has to say what it is, or the line opens on a
    // bare emphasised number: `*221*, ed. Volker Kaibel…`.
    let volume = volume(entry);
    let issue = issue(entry);
    let opening_was_empty = opening.is_empty();
    if opening.is_empty() {
        if let Some(volume) = &volume {
            clauses.push(format!("vol. {volume}"));
        }
        if let Some(issue) = &issue {
            clauses.push(format!("no. {issue}"));
        }
    } else {
        if let Some(volume) = &volume {
            opening.push_str(&format!(" *{volume}*"));
            if let Some(issue) = &issue {
                opening.push_str(&format!("({issue})"));
            }
        } else if let Some(issue) = &issue {
            opening.push_str(&format!(", no. {issue}"));
        }
        clauses.push(opening);
    }

    if let Some(editors) = editor_list(entry) {
        clauses.push(format!("ed. {editors}"));
    }

    if let Some(pages) = entry.page_range() {
        clauses.push(format!("pp. {}", en_dash_range(&pages.to_string())));
    }

    // With no venue to open on, the first clause begins the sentence after the
    // title, and `vol.`/`ed.` are lower case: "_Convex Polytopes_. vol. 221".
    if opening_was_empty {
        if let Some(first) = clauses.first_mut() {
            *first = capitalize_first(first);
        }
    }

    let mut line = clauses.join(", ");

    // The publisher opens its own sentence: it modifies the work, not the
    // volume-and-pages clause it would otherwise attach to.
    let publisher = entry.publisher();
    let name = publisher
        .and_then(|publisher| publisher.name())
        .map(|name| name.to_string());
    let location = publisher
        .and_then(|publisher| publisher.location())
        .map(|location| location.to_string());
    let imprint = match (name, location) {
        (Some(name), Some(location)) => Some(format!("{name}, {location}")),
        (Some(name), None) => Some(name),
        (None, Some(location)) => Some(location),
        (None, None) => None,
    };
    if let Some(imprint) = imprint {
        let imprint = escape_markup(&imprint);
        if line.is_empty() {
            line = imprint;
        } else if clauses.len() > 1 {
            // Enough has accumulated that another comma would bury the
            // publisher inside the volume-and-pages clause it does not belong
            // to: "…, ed. Neil White, pp. 139–160. Cambridge University Press".
            line.push_str(&format!(". {imprint}"));
        } else {
            // A lone clause is just the kind of work, and "Thesis. San
            // Francisco" reads as two fragments rather than one citation.
            line.push_str(&format!(", {imprint}"));
        }
    }

    line
}

/// The whole citation, as it would appear in a bibliography.
///
/// The stub's metadata block holds author, title, year, venue and identifiers,
/// and almost none of it reaches the reader: `[build].header-keys` prints a
/// couple of values under the title and the rest stays in `wanshi.json`. A page
/// standing for a work should show the work. So the body is a complete
/// reference -- the thing you would paste into a bibliography -- rather than a
/// fragment that assumes the heading beside it.
fn full_reference(entry: &Entry) -> String {
    let mut sentences: Vec<String> = Vec::new();

    if let Some(authors) = author_citation(entry) {
        sentences.push(authors);
    }
    if let Some(title) = title_markup(entry) {
        sentences.push(title);
    }

    // Naming the kind of work is the fallback when nothing records where it
    // appeared -- every thesis, report and preprint -- so the citation still
    // says what it is rather than ending after the title.
    let mut tail = publication_details(entry);
    if tail.is_empty() {
        tail = type_label(entry);
    }

    // The year closes the citation, attached to the venue clause rather than
    // standing as its own sentence.
    if let Some(year) = year(entry) {
        tail.push_str(&format!(", {year}"));
    }
    sentences.push(tail);

    format!("{}.", sentences.join(". "))
}

/// Join names the way a citation reads them: the last one after "and".
fn name_list(people: &[hayagriva::types::Person]) -> Option<String> {
    let names: Vec<String> = people
        .iter()
        .map(|person| person.given_first(false))
        .collect();
    match names.as_slice() {
        [] => None,
        [one] => Some(one.clone()),
        [first, second] => Some(format!("{first} and {second}")),
        [rest @ .., last] => Some(format!("{} and {}", rest.join(", "), last)),
    }
}

/// Editors, given-name first, as a readable list.
fn editor_list(entry: &Entry) -> Option<String> {
    name_list(entry.editors()?)
}

/// Authors as a citation reads them, which is not how the `author` metadata
/// field lists them: that stays a plain comma-separated list so anything
/// filtering on it can split cleanly.
fn author_citation(entry: &Entry) -> Option<String> {
    name_list(entry.authors()?)
}

/// Neutralise Typst markup characters in text that is emitted as content.
///
/// Titles carry `_`, `*` and `@` more often than one would like -- a chemistry
/// title with `_2`, a filename, an email -- and each is markup in a Typst body.
/// Unescaped, `Ext_2` silently begins emphasis that runs to the next
/// underscore, or to the end of the paragraph.
fn escape_markup(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if matches!(ch, '_' | '*' | '@' | '#' | '$' | '`' | '<' | '>' | '\\') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
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
        full_reference(entry)
    }

    /// Hayagriva files a journal article's volume and issue on the periodical
    /// parent, so reading them off the entry found them on a book and lost them
    /// on the article — the one entry type where they matter.
    #[test]
    fn test_a_journal_article_keeps_its_volume_and_issue() {
        assert_eq!(
            line("knuth1984literate"),
            "Donald E. Knuth. \"Literate Programming\". \
             _The Computer Journal_ *27*(2), pp. 97–111, 1984."
        );
    }

    #[test]
    fn test_a_book_names_its_publisher_and_nothing_it_does_not_have() {
        assert_eq!(line("book1999"), "Ann Writer. _A Book_. A Press, 1999.");
    }

    #[test]
    fn test_proceedings_read_as_a_container_with_pages() {
        assert_eq!(
            line("proc2001"),
            "Bo Speaker. \"A Paper\". In _Proceedings of Somewhere_, pp. 5–9, 2001."
        );
    }

    /// A citation ends in its year. The body used to omit it to avoid repeating
    /// the heading, which left the reference incomplete -- the body is a whole
    /// bibliography entry now, not a fragment that leans on the heading.
    #[test]
    fn test_the_citation_ends_in_its_year() {
        assert!(line("knuth1984literate").ends_with("1984."));
        assert!(line("book1999").ends_with("1999."));
        assert!(line("proc2001").ends_with("2001."));
    }

    /// Every reference style quotes a work published inside another and
    /// italicises the container, while italicising a work that is itself the
    /// publication. That contrast is what tells a reader which kind of thing
    /// they are looking at.
    #[test]
    fn test_a_contained_work_is_quoted_and_a_standalone_one_italicised() {
        assert!(line("knuth1984literate").contains("\"Literate Programming\""));
        assert!(line("knuth1984literate").contains("_The Computer Journal_"));
        assert!(line("book1999").contains("_A Book_"));
    }

    #[test]
    fn test_en_dash_range_only_touches_a_hyphen_between_digits() {
        assert_eq!(en_dash_range("97-111"), "97–111");
        assert_eq!(en_dash_range("97"), "97");
        assert_eq!(en_dash_range("A-1"), "A-1");
        assert_eq!(en_dash_range("12-"), "12-");
    }

    /// An entry with no container, publisher, volume or pages used to produce
    /// no line at all -- which is every thesis, report and preprint, so the
    /// works least recognisable from a title alone were exactly the ones whose
    /// stub said nothing. Naming the kind of thing it is beats an empty line.
    #[test]
    fn test_an_entry_with_no_detail_still_names_its_kind() {
        let library =
            hayagriva::io::from_biblatex_str("@misc{bare, title = {Bare}, year = {2020}}")
                .expect("parses");
        assert_eq!(
            full_reference(library.get("bare").expect("entry")),
            "_Bare_. Misc, 2020."
        );
    }

    /// Zotero records an arXiv posting twice -- as the abs URL and as the
    /// DataCite DOI arXiv mints for it -- and both resolve to the same page.
    /// Listing them separately gave a preprint two links to itself.
    #[test]
    fn test_arxiv_is_recognised_in_both_the_url_and_the_doi() {
        assert_eq!(arxiv_id("http://arxiv.org/abs/2105.10386").as_deref(), Some("2105.10386"));
        assert_eq!(arxiv_id("https://arxiv.org/pdf/2105.10386v2").as_deref(), Some("2105.10386"));
        assert_eq!(arxiv_id("10.48550/arXiv.2105.10386").as_deref(), Some("2105.10386"));
        assert_eq!(arxiv_id("10.1017/CBO9781107325715.010"), None);
        assert_eq!(arxiv_id("https://www.cambridge.org/core/books/x"), None);
    }

    /// The DOI used to win and the URL was discarded. 57 of the 63 entries in
    /// the library this was built against carry a URL, so that silently threw
    /// away the only link for most of them.
    #[test]
    fn test_a_doi_and_a_separate_url_are_both_kept() {
        let library = hayagriva::io::from_biblatex_str(
            r#"@book{both,
                 title = {A Book},
                 year = {2021},
                 doi = {10.1017/9781108606806},
                 url = {https://www.cambridge.org/core/books/x/ABC123},
               }"#,
        )
        .expect("parses");
        let links = work_links(library.get("both").expect("entry"));
        assert_eq!(
            links,
            vec![
                (
                    "https://doi.org/10.1017/9781108606806".to_string(),
                    "doi:10.1017/9781108606806".to_string()
                ),
                (
                    "https://www.cambridge.org/core/books/x/ABC123".to_string(),
                    "cambridge.org".to_string()
                ),
            ]
        );
    }

    /// An arXiv preprint carrying both forms yields one link, labelled with the
    /// identifier a reader recognises rather than either raw address.
    #[test]
    fn test_an_arxiv_preprint_yields_one_link() {
        let library = hayagriva::io::from_biblatex_str(
            r#"@report{pre,
                 title = {A Preprint},
                 year = {2021},
                 doi = {10.48550/arXiv.2105.10386},
                 url = {http://arxiv.org/abs/2105.10386},
               }"#,
        )
        .expect("parses");
        let links = work_links(library.get("pre").expect("entry"));
        assert_eq!(
            links,
            vec![(
                "https://arxiv.org/abs/2105.10386".to_string(),
                "arXiv:2105.10386".to_string()
            )]
        );
    }

    /// A publisher URL runs past 90 opaque characters and says nothing in full.
    #[test]
    fn test_host_label_strips_scheme_path_and_www() {
        assert_eq!(host_label("https://www.cambridge.org/core/books/x/ABC"), "cambridge.org");
        assert_eq!(host_label("http://arxiv.org/abs/1"), "arxiv.org");
        assert_eq!(host_label("https://scholarworks.calstate.edu/x?y=1"), "scholarworks.calstate.edu");
    }

    /// `_` and `*` are markup in a Typst body, so an unescaped title beginning
    /// emphasis runs it to the end of the paragraph.
    #[test]
    fn test_markup_characters_in_a_title_are_escaped() {
        assert_eq!(escape_markup("Ext_2 and Tor_1"), "Ext\\_2 and Tor\\_1");
        assert_eq!(escape_markup("a@b"), "a\\@b");
        assert_eq!(escape_markup("plain"), "plain");
    }
}
