# Links and References

Linking is what turns a directory of Typst files into a forest. wanshi has three
connective primitives, and every one of them feeds the same underlying graph:

| Primitive | Helper | What it does |
| --- | --- | --- |
| **Local link** | `local()` | Inline link to another section. May create a reference and a backlink. |
| **Embed** | `embed()` | Renders another section's full content inside this one, makes this section its parent, and records this section under the target's **Found in**. |
| **External link** | `external()` | Ordinary link off-site. Not part of the graph. |

References and backlinks are then *derived* from those links — you never write
them by hand.

## Slugs Are the Address Space

Every link target is a slug, and slugs are always tree-relative paths without the
`.typ` extension:

```
trees/notes/alice.typ   ->   notes/alice
trees/refs/knuth.typ    ->   refs/knuth
trees/index.typ         ->   index
```

Link targets are resolved **relative to the directory containing the current
note**, exactly like relative file paths — with one addition: a leading `/` means
"from the root of the source tree".

Writing from `trees/notes/alice.typ` (whose own slug is `notes/alice`):

| You write | Resolves to |
| --- | --- |
| `#local("bob")` | `notes/bob` |
| `#local("./bob")` | `notes/bob` |
| `#local("../refs/knuth")` | `refs/knuth` |
| `#local("/refs/knuth")` | `refs/knuth` |
| `#local("../index")` | `index` |
| `#local("/index")` | `index` |

Note that a bare `bob` and `./bob` are identical — a target without a leading `/`
is *always* relative. This is the most common source of confusion when a note
moves between directories.

**Recommendation:** use root-absolute targets (`/refs/knuth`) for anything
outside the current directory. They survive moving the note, whereas `../../..`
chains do not.

The `.typ` extension is optional in a target and is stripped if present, so
`#local("./bob")` and `#local("./bob.typ")` are the same link.

## Local Links

```typst
#local("/notes/alice")
```

With no text argument, the link's label is the **target's title**, fetched
automatically. This is usually what you want: retitle a note once and every link
to it updates.

Supply `text:` to override the label:

```typst
#local("/notes/alice", text: "Alice's construction")
#local("/notes/alice", text: [the *free* construction])
```

The text may be a plain string or rich Typst content.

Generated links carry a `title` attribute of the form `Page Title [slug]`, so
hovering a link in the browser reveals where it points.

### Links Inside Metadata

Metadata values may themselves contain links. This is the idiomatic way to
express a typed relationship — "see also", "supersedes", "source" — that should
appear in the note's header rather than in its prose:

```typst
#metadata((
  "title": "Free monoids",
  "taxon": "definition",
  "date": "2026-08-06",
  "see-also": local("/notes/monoids"),
))
```

Two constraints:

- Only **custom** metadata keys may hold rich content. The structural keys
  (`parent`, `footer-mode`, `backlinks`, `asref`, …) must be plain text, and a
  build fails with an explicit error if one of them is given content.
- Only the *value* is rendered, not the key. The header shows the link; it does
  not print "see-also:". If you want a visible label, put it in the value.

You may see `#context metadata(( ... ))` in older sources, including
`featured-demo`. The plain form works; the `#context` wrapper is harmless.

## Embeds

An embed pulls the target's rendered content into the current page:

```typst
#embed("/notes/alice", "Alice, as embedded here")
```

The second argument is a **title override** for this embedding only — the target
page keeps its own title. Pass `none`-free content or a plain string.

Options:

```typst
#embed("/notes/alice", "Alice", numbering: true, open: false, catalog: false)
```

- `numbering` (default `false`) — number the embedded section.
- `open` (default `true`) — whether the embedded `<details>` starts expanded.
- `catalog` (default `true`) — whether it appears in the table of contents.
- `display-options` (default `false`) — paged-output-only debugging aid that
  prints the option values; it has no effect on HTML output.

Two graph effects to be aware of:

- **Embedding sets the parent.** The embedded section's "previous level"
  navigation will point at the embedding page, unless the target sets `parent`
  explicitly.
- **An open embed propagates references upward.** If `open: true` (the default),
  everything the embedded note references is also counted as a reference of the
  embedding page. This is why an index that embeds a note inherits that note's
  bibliography.

Missing embed targets are **hard errors** — unlike dangling local links, they
fail the build, because the page would otherwise be structurally incomplete.
Cycles are detected and reported with the full chain:

```
cyclic embed detected: index -> notes/alice -> notes/bob -> index
```

## External Links

```typst
#external("https://typst.app", [the Typst website])
```

`external` renders an underlined off-site link. It deliberately does not touch
the graph: no references, no backlinks, no validation.

Plain Typst `#link()` also works; `external` just applies the site's link
styling.

## References

A **reference** is a link target that wanshi considers citation-worthy. Reference
targets are collected per page and rendered in a "References" section in the page
footer.

A link target becomes a reference if **either** condition holds:

1. Its `data-taxon` starts with `reference` (case-insensitive), or
2. `asref` is true for it — from the target's own `asref` metadata if set,
   otherwise from the global `[build].asref` default.

### Reference Taxons

The first rule is the low-ceremony one, and it is what you want for a
bibliography. Give the note a `reference` taxon and it becomes a reference target
automatically, with no configuration at all:

```typst
// trees/refs/knuth.typ
#import "/_lib/wanshi.typ": *
#show: wanshi

#metadata((
  "title": "The Art of Computer Programming",
  "taxon": "reference",
  "date": "1968",
))

Knuth, D. E. Addison-Wesley.
```

Now any note that links to it picks it up as a reference:

```typst
// trees/notes/alice.typ
Sorting networks are treated exhaustively in #local("/refs/knuth").
```

`notes/alice` renders a References footer containing the Knuth entry — even
though `[build].asref` is `false`.

### Making Everything a Reference

If your forest is one where nearly every link is a citation, flip the global
default instead of tagging each target:

```toml
[build]
asref = true
```

Then opt individual notes out with `"asref": "false"` in their metadata. This is
what `featured-demo` does.

### Turning the Footer Off

`"references": "false"` on a page suppresses its References footer without
changing what the graph records.

### Citing Papers and Books

The notes above are hand-written, which is fine for a handful of works and
tedious past that — bibliographic detail gets retyped, and drifts from whatever
you keep it in. `wanshi refs` reads a bibliography instead and writes the notes
for you.

Point the configuration at a bibliography:

```toml
[refs]
bibliography = "trees/_bib/refs.bib"   # .bib, .yaml or .yml
dir = "refs"                           # where generated notes live
```

Cite a work by its citation key, as an ordinary link:

```typst
Influence theory begins with #local("/refs/kkl1988", text: [KKL88]).
```

`wanshi check` will report that as a dangling link, because the note does not
exist yet, and point you at the fix. Run it:

```sh
wanshi refs sync
```

That writes `trees/refs/kkl1988.typ` from the bibliography entry with a
`reference` taxon, so it behaves exactly like the hand-written note above. The
page carries where the work appeared and how to reach it:

```
Kahn, Kalai & Linial 1988 — The influence of variables on Boolean functions
1988   Jeff Kahn, Gil Kalai, Nathan Linial

_Proc. 29th FOCS_, pp. 68–80.

doi:10.1109/SFCS.1988.21923 · ieee.org
```

Volume, issue, pages, editors, publisher and location are used when present, and
an entry with none of them still names what kind of work it is rather than
rendering an empty line. **Every link is kept** — a DOI does not suppress a
separate URL, since for most libraries the URL is the only address on file. An
arXiv posting is recognised in either form (the abs URL, or the DataCite
`10.48550/arXiv.…` DOI) and rendered once as `arXiv:2105.10386`, because both
resolve to the same page and the identifier is what a reader recognises. Other
URLs are labelled by host, since a publisher link runs to 90 opaque characters
and says nothing in full. Nothing else changes:
the citation is a link, so the citing note gets a References footer and the work
collects a backlink from every note that cites it.

**Notes are generated on demand.** Only works you have actually cited get a
note, so a bibliography of hundreds does not become hundreds of pages.

#### Where the bibliography lives

Anywhere, but inside the source tree under a `_` directory is the useful place.
Discovery skips `_`-prefixed directories and only `.typ`/`.typst` are section
extensions, so the file can never become a page — while `[build].typst-root` is
the tree, so a note that wants Typst's own citation system can reach the same
file:

```typst
#bibliography("/_bib/refs.bib")
```

That is worth keeping open for a note that is a draft of paper prose. It is not
how you cite across the forest, though: each note compiles as its own document,
so a Typst bibliography is per-note and invisible to the graph.

#### Keeping notes and bibliography in step

`refs sync` also refreshes notes it generated earlier, so correcting a title
upstream reaches the forest on the next run. It knows which are its own by a
marker comment at the top of each generated file:

- **Delete the marker** and the file is yours; sync will say so and leave it
  alone from then on.
- Identical content is never rewritten, so a re-run touches nothing and a
  running `wanshi serve` does not rebuild over it.

Your own thinking about a work belongs in an ordinary note that links to the
generated one, not in the generated one itself. The work's backlinks then
collect every note you have written about it, which is the view a generated stub
cannot give you.

#### Collecting a bibliography for a paper

The graph knows which works a body of notes depends on, which is the question a
reference manager cannot answer:

```sh
wanshi refs export --from boolean/ > paper.bib
```

That prints the entries cited by every note whose slug starts with `boolean/`.
Omit `--from` for the whole forest.

Output matches the bibliography's own format, which is the lossless choice both
ways. A BibTeX source is copied **verbatim**, so brace protection like
`{Boolean}` and any field wanshi does not model survive intact. A Hayagriva
source is re-serialised, which is lossless because Hayagriva is the format this
is parsed into anyway. `--format` overrides — but only from BibTeX to Hayagriva,
since hayagriva reads BibTeX and does not write it. Asking for the impossible
direction is an error rather than a lossy guess.

#### One directory, flat

Generated notes sit directly in `[refs].dir`, never nested by topic. A citation
key identifies one work, so a work gets one page: filing it under the topic that
cites it would force a choice the first time a second topic cited the same work.
The topical view already exists — it is that work's backlinks.

## Backlinks

A **backlink** is the reverse edge: page A links to page B, so B lists A. They
are generated automatically for every local link, with no authoring effort.

Embedding is tracked separately, under **Found in** — see below. Citing a note
and containing one are different relationships, so they are answered separately
rather than pooled.

A backlink from A to B is recorded when all of these hold:

- A and B are different sections (a self-link does not backlink),
- B allows backlinks — `backlinks` is not `false` (default: allowed),
- A contributes backlinks — `asback` is not `false` (default: contributes).

So the switches are:

| Metadata | On | Effect |
| --- | --- | --- |
| `"backlinks": "false"` | the **target** | This page never displays backlinks. |
| `"asback": "false"` | the **linker** | This page's links never create backlinks elsewhere. Useful for indexes and hub pages that would otherwise backlink to everything. |
| `"transparent-backlinks": "true"` | the target | Show backlinks even when this section appears embedded in another page (except inside footers). |

## Found in

The reverse edge of an embed: page A embeds page B, so B lists A. Like backlinks
it is automatic, and it appears as its own footer section.

It exists because an embed's other trace is easy to lose. Embedding also makes A
the parent of B, which is what draws B's breadcrumb — but `parent` holds a single
slug, and one that B declares for itself replaces the embedder entirely. Since
declaring a parent is the recommended fix for a note embedded in several places,
following that advice used to erase every record of the embedding. Found in does
not have that problem: it records every host.

Two properties worth knowing:

- **Direct hosts only.** If A embeds B and B embeds C, then C's Found in lists B,
  not A. The relationship recorded is containment by one level, not reachability.
- **Always rendered as links**, whatever `[build].footer-mode` says. A host
  contains the note whose page you are reading, so rendering one in `embed` mode
  would print the page inside its own footer.

A named subtree is an embed, so it lists the note it was written in — which is
also what its breadcrumb says. `"backlinks": "false"` on the target suppresses
Found in along with backlinks; `"asback": "false"` does **not**, because it
governs a page's links, and a page that embeds notes is usually one you want
listed.

## Footer Rendering

References and backlinks are both rendered in the footer, in one of two modes:

```toml
[build]
footer-mode = "link"     # default: compact entries — taxon, title, slug, date
footer-mode = "embed"    # full content of each referenced/backlinking section
```

Override per note with `"footer-mode": "embed"` in its metadata.

Ordering is deterministic and controlled by a sort key:

```toml
[build]
footer-sort-by = "slug"   # default; also "date", "taxon", "title", or any custom key
```

Override per note with `"footer-sort-by": "date"`.

When sorting by `date`, values are parsed chronologically. `2026-08-06`,
`10/12/2025`, `October 12, 2025`, and `12 October 2025` all parse. Anything that
does not parse — a bare year such as `1968`, or an empty value — falls back to
plain string comparison, which is stable but not chronological.

## Linking to Subtrees

A named subtree is a first-class section: it has a slug, a page, and a place in
the graph, so you link to it exactly like a file-backed note.

```typst
// trees/notes/algebra.typ
#definition(slug: "monoid", title: "Monoid")[
  A set with an associative operation and a two-sided identity.
]
```

The slug resolves relative to the **containing directory**, so this defines
`notes/monoid`, published at `/notes/monoid.html`. From a sibling note:

```typst
// trees/notes/alice.typ
Recall the notion of a #local("monoid").
```

Anonymous subtrees (no `slug:`) cannot be linked. They are stripped from the
reference and backlink graph, omitted from `wanshi.json` and
`wanshi.graph.json`, and their children are re-parented to the nearest visible
ancestor.

## Inspecting the Graph

`wanshi build` writes `wanshi.graph.json`, which is the fastest way to confirm
that what you wrote is what wanshi understood:

```json
{
  "sections": {
    "notes/alice": {
      "parent": "index",
      "parent_specified": false,
      "references": ["refs/knuth"],
      "backlinks": ["index"],
      "embedded_by": []
    },
    "refs/knuth": {
      "parent": "index",
      "parent_specified": false,
      "references": [],
      "backlinks": ["notes/alice", "notes/bob"],
      "embedded_by": ["notes/alice"]
    }
  }
}
```

`parent_specified` is `true` only when the section declared `parent` itself. A
`false` means the parent was derived — from whatever embedded the section, or
failing that from the nearest enclosing directory index. See
[Grouping Notes into Categories](writing-notes.md#grouping-notes-into-categories).

`embedded_by` lists every section that embeds this one, and is the reliable
record of that: `parent` holds a single slug and is displaced when a section
declares its own, so a note embedded in several places, or one that names its
parent, is only fully described here.

## A Worked Example

Four files showing every mechanism together.

```typst
// trees/index.typ
#import "_lib/wanshi.typ": *
#show: wanshi

#metadata((
  "title": "An example forest",
  "taxon": "example",
  "date": "2026-08-06",
  "see-also": local("/notes/alice"),
))

An overview, linking to #local("/notes/bob") and out to
#external("https://typst.app", [Typst]).

#embed("/notes/alice", "Alice, embedded into the index")
```

```typst
// trees/notes/alice.typ
#import "/_lib/wanshi.typ": *
#show: wanshi

#metadata((
  "title": "Alice",
  "taxon": "remark",
  "date": "2026-08-01",
  "footer-sort-by": "date",
))

A sibling link: #local("bob").
A citation: #local("/refs/knuth").
Back up: #local("/index").
```

```typst
// trees/notes/bob.typ
#import "/_lib/wanshi.typ": *
#show: wanshi

#metadata((
  "title": "Bob",
  "taxon": "definition",
  "date": "2026-08-02",
))

Bob also cites #local("/refs/knuth").
```

```typst
// trees/refs/knuth.typ
#import "/_lib/wanshi.typ": *
#show: wanshi

#metadata((
  "title": "The Art of Computer Programming",
  "taxon": "reference",
  "date": "1968",
))

Knuth, D. E. Addison-Wesley.
```

The resulting graph:

- `index` — embeds `notes/alice`, so it is the parent of everything. Because that
  embed is open, it **inherits `refs/knuth` as a reference** from Alice. Gets a
  backlink from `notes/alice`.
- `notes/alice` — references `refs/knuth`; backlinked from `index`. Its footer is
  sorted by date rather than slug.
- `notes/bob` — references `refs/knuth`; backlinked from both `index` and
  `notes/alice`.
- `refs/knuth` — a reference target purely by virtue of its taxon; backlinked
  from `notes/alice` and `notes/bob`.

## Troubleshooting

| Symptom | Cause |
| --- | --- |
| `Dangling local link ... resolves to missing section X` | The target does not exist. The message names the resolved slug — compare it against the slug you expected, since this is usually a relative-vs-absolute mistake. |
| Link renders with empty text | Target exists but has no `title` metadata. |
| `cyclic embed detected: A -> B -> A` | Two sections embed each other. Convert one embed into a `local()` link. |
| `[A] attempting to fetch a non-existent [B]` | The **embed** target `B` referenced from `A` is missing. Unlike links, this fails the build. |
| Expected reference missing from the footer | Target is not reference-like: it lacks a `reference` taxon and `asref` is false both on it and globally. |
| Backlinks missing | The target sets `"backlinks": "false"`, or the linking page sets `"asback": "false"`. |
| Unexpected backlinks from a hub page | Set `"asback": "false"` on the hub. |
| Footer entries in the wrong order | `footer-sort-by` is `slug` by default; set it to `date`, and check that your dates actually parse. |
