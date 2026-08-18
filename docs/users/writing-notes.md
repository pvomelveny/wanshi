# Writing Notes

This page is the day-to-day workflow: how a new note goes from "I have an idea"
to a published page that is properly wired into the rest of the forest.

For the exhaustive list of metadata keys and Typst helpers, see
[Content Authoring](content-authoring.md). For everything about connecting notes
together, see [Links and References](links-and-references.md).

## The Loop

Keep a preview running in one terminal and edit in another. wanshi rebuilds only
what changed.

```sh
wanshi serve            # terminal 1: leave this running
```

Then, for each note:

```sh
wanshi new post notes/monoids     # 1. create
$EDITOR trees/notes/monoids.typ # 2. write metadata + body
                                  # 3. link it to its neighbours
wanshi check                      # 4. validate the graph
```

And when you are ready to ship:

```sh
wanshi check --strict
wanshi build
```

The rest of this page expands each step.

## 1. Create the File

```sh
wanshi new post notes/monoids
```

This writes `trees/notes/monoids.typ`. A few things worth knowing:

- The `.typ` extension is added automatically. `.typst` is also accepted, for
  sites created before `.typ` was adopted; any other extension is rejected.
- The path is resolved under the configured source tree (`trees/` by default).
  Writing `wanshi new post trees/notes/monoids` works too; the leading tree name
  is stripped rather than doubled.
- Intermediate directories are created for you.
- The command refuses to overwrite an existing file.

You can of course just create the file by hand. `wanshi new post` exists mainly
to apply a template (see below).

### The Default Skeleton

A freshly created note looks like this:

```typst
#import "/_lib/wanshi.typ": *

#show: wanshi

#metadata((
  "title": "monoids",
))
```

Three parts, all required in practice:

1. **The import** pulls in wanshi's Typst library from `trees/_lib/wanshi.typ`.
2. **`#show: wanshi`** installs the show rules that make math and figures render
   correctly in HTML output. Without it, equations will not be framed properly.
3. **`#metadata(...)`** declares the note's title, taxon, date, and any other
   fields.

### Import Paths

The leading `/` in the import matters. Because `[build].typst-root` is the source
tree, a **root-absolute** import resolves the same way from any depth, so a note
keeps working when you move it between directories:

```typst
#import "/_lib/wanshi.typ": *
```

A tree-relative import (`"_lib/wanshi.typ"`) only resolves for notes sitting at
the very top of the source tree, and a file-relative one (`"../_lib/wanshi.typ"`)
has to be adjusted whenever the note moves. Prefer the root-absolute form
everywhere; it is what `wanshi new post` generates.

### Templates

`wanshi new post` copies a template file and substitutes `<FILE_NAME>` with the
new file's stem. If a file named `template` exists in the project root it is used
automatically; otherwise the built-in skeleton above is used.

```sh
wanshi new post notes/monoids --template templates/note.typ
```

A useful project template:

```typst
#import "/_lib/wanshi.typ": *

#show: wanshi

#metadata((
  "title": "<FILE_NAME>",
  "taxon": "remark",
  "date": "2026-08-06",
))
```

Only `<FILE_NAME>` is substituted — the date is not filled in for you, so treat
it as a placeholder to edit.

## 2. Choose the Slug

The slug is the note's permanent identity: its URL, its link target, and its
sort key. It is simply the source path under the tree, minus the extension:

| Source file | Slug | Linked as |
| --- | --- | --- |
| `trees/index.typ` | `index` | `/` |
| `trees/notes/monoids.typ` | `notes/monoids` | `/notes/monoids.html` |
| `trees/refs/knuth.typ` | `refs/knuth` | `/refs/knuth.html` |
| `trees/notes/index.typ` | `notes/index` | `/notes/` |

A note named `index` is linked as its directory, since that is what serves it —
`/notes/` rather than `/notes/index.html`. The longer form still works, so
anything already linking to it keeps resolving.

Two consequences worth internalising before you have a hundred notes:

- **Renaming a file changes its slug**, which breaks inbound links and public
  URLs. Pick a name you can live with, and prefer stable identifiers over
  descriptive ones if the note's topic may drift.
- **Directories can carry structure.** Put an `index` note in a directory and it
  becomes the parent of everything beside it — see
  [Grouping Notes into Categories](#grouping-notes-into-categories) below. A flat
  `trees/` organised purely by embeds works just as well; both are supported.

Directories beginning with `.` or `_` are skipped entirely, which is why the
bundled library lives in `trees/_lib/` without becoming a page.

### Grouping Notes into Categories

A note named `index` inside a directory is that directory's hub. Any note in the
directory that nothing else claims becomes its child, and the hub itself becomes
a child of the directory above it:

```
trees/
  index.typ              "Root"
  notes/
    index.typ            "Notes"       → parent: index
    alice.typ            "Alice"       → parent: notes/index
    deep/
      index.typ          "Deep"        → parent: notes/index
      carol.typ          "Carol"       → parent: notes/deep/index
  other/
    bob.typ              "Bob"         → parent: index   (no other/index exists)
```

Each page's "previous level" navigation follows that chain, so `carol` links back
to `Deep`, which links back to `Notes`, which links back to `Root`. Directories
without an `index` are pure namespacing — their notes climb to the nearest
ancestor that does have one, or to the root.

This costs nothing to adopt and nothing to ignore: cross-linking is unaffected,
because links never change parentage.

Parent selection has a strict precedence:

1. **`parent` metadata**, if the note declares one — always wins.
2. **The embedding page**, if some note embeds this one.
3. **The nearest enclosing directory index**.
4. **The root `index`**.

An `index` section is strongly recommended: it is the compiler's entry point and
the default parent for notes that nothing embeds. Builds warn if it is missing.

## 3. Write the Metadata

```typst
#metadata((
  "title": [Monoids and their #emph[free] constructions],
  "taxon": "definition",
  "date": "2026-08-06",
  "author": "Patrick",
))
```

The fields you will reach for constantly:

- **`title`** — may be rich Typst content, not just a string. It is used in the
  page header, in the browser title, and as the default text of every link
  pointing at this note.
- **`taxon`** — the note's kind (`definition`, `remark`, `theorem`, `reference`,
  …). It renders as a small label before the title and is available as a sort
  key. A taxon starting with `reference` also changes linking behaviour — see
  [Links and References](links-and-references.md#reference-taxons).
- **`date`** — conventional, but wanshi gives it a dedicated column in page
  headers and in listings, so it is worth setting consistently.

Any other key you invent is preserved, sortable, and queryable, but shown only
if you list it in `[build].header-keys`. That defaults to `["date", "author"]`,
so `"author": "Patrick"` above appears beside the date and a `"status"` would
not — add it to the list if you want it on the page.

For dates, `YYYY-MM-DD` sorts most reliably. `October 12, 2025` and `10/12/2025`
also parse. A bare year like `1968` does not parse as a date and will fall back
to plain string ordering when sorting by date.

See the [metadata key reference](content-authoring.md#metadata-reference) for
the complete list, including the switches that control backlinks and footers.

## 4. Write the Body

Everything after the metadata is ordinary Typst. Inline and display math,
figures, tables, and third-party packages from the Typst Universe all work,
because wanshi shells out to your local `typst` installation.

```typst
A monoid is a set $M$ with an associative operation and an identity:

$ forall a in M, quad e dot a = a dot e = a $
```

Two helpers exist for diagrams that must survive the HTML export:

```typst
#import "@preview/fletcher:0.5.8" as fletcher: node, edge

#auto-figure(auto-frame(fletcher.diagram(
  node((0, 0), $Z$),
  node((-1, 1.5), $X$),
  edge((-1, 1.5), (0, 0), "->", $sigma_X$, bend: 15deg),
)))
```

`auto-frame` renders Typst-drawn content as inline SVG for HTML output while
leaving paged output untouched; `auto-figure` centres it. Use both when embedding
package-drawn diagrams such as `fletcher` or `cetz`.

To reuse a figure across several notes, define it in a directory whose name
starts with `_` and import it — see
[Sharing Typst Code](content-authoring.md#sharing-typst-code).

## Building Pages That Maintain Themselves

A hub page that you have to edit every time you add a note will drift. Listings
solve that: they render a set of sections chosen at build time, so the page
updates itself.

Turn a directory index into a real category page with one line:

```typst
// trees/notes/index.typ
#import "/_lib/wanshi.typ": *
#show: wanshi

#metadata(( "title": "Notes", "taxon": "collection" ))

Everything filed under notes:

#children()
```

Every note in `trees/notes/` now appears here, in date order, forever — because
[directory indexes are real parents](#grouping-notes-into-categories).

A root index that shows what you have been working on:

```typst
#recent(count: 10, title: "Recently written")
```

A page collecting one kind of note:

```typst
#by-taxon("definition", title: "All definitions")
#by-taxon("reference", title: "Bibliography")
```

And the maintenance view — notes that nothing links to and nothing embeds, which
in a forest usually means you wrote them and lost track:

```typst
#orphans(title: "Not linked from anywhere")
```

This includes **directory index pages that nothing links to**. Being a parent
makes a hub reachable *from* its children, not *to* it, so an unlinked
`notes/index` really is invisible to someone browsing from the root — listing it
is a prompt to link it from somewhere. Only the root `index` is exempt, since it
is the entry point.

If that is noisier than useful once you have many hubs, turn it off:

```typst
#orphans(include-indexes: false, title: "Unreachable notes")
```

The same option works on any listing, so `#children(include-indexes: false)`
lists a section's notes without its sub-hubs.

Anything more specific goes through `#query(...)` directly, which the four
shorthands above are built on:

```typst
#query(from: "notes/", key: "author", value: "sam", sort: "date", order: "desc")
#query(from: "descendants", taxon: "theorem", limit: 5, title: "Recent theorems")
```

The full parameter list is in
[Content Authoring](content-authoring.md#listings).

**Two things to know.** A listing never includes the page it is written on. And
because a listing depends on every other note, wanshi rewrites listing pages
whenever anything in the forest changes — so `wanshi serve` shows a new note
appearing in its hub without you touching the hub.

## 5. Connect It

A note that nothing links to is a leaf that no reader will find. Before you
consider a note done, add at least one link in each direction:

```typst
Builds on #local("/notes/semigroups").
```

...and add a link or embed *to* the new note from wherever it belongs — usually
`index.typ` or a hub note.

This is the substance of the system, and it has its own page:
[Links and References](links-and-references.md).

## 6. Decide: New File or Subtree?

Not every idea deserves its own file. wanshi lets one source file define extra
sections inline, each of which still gets its own slug, its own page, and its own
place in the graph.

```typst
#definition(slug: "monoid", title: "Monoid")[
  A set with an associative operation and a two-sided identity.
]
```

Written inside `trees/notes/algebra.typ`, that produces the section
`notes/monoid`, published at `/notes/monoid.html` — the slug resolves relative to
the *containing directory*, not to the parent note's slug.

Rules of thumb:

- **Separate file** when the note is a topic you will link to from many places,
  or when it will keep growing.
- **Named subtree** when several small, tightly related statements are best
  written and edited together — a definition with its lemma and proof — but each
  still deserves a stable URL.
- **Anonymous subtree** (omit `slug:`) for asides that need a heading but never
  need to be linked. These are excluded from `wanshi.json`, from the graph
  artifact, and from the reference/backlink graph entirely.

The full list of subtree helpers (`theorem`, `lemma`, `proof`, `example`, …) is
in [Content Authoring](content-authoring.md#subtrees).

## 7. Check Before Publishing

```sh
wanshi check
```

Check parses everything and validates the graph without writing any output. It
catches the failure modes that a plain build would either hide or blow up on:

- typos in link targets (dangling local links)
- missing `index`
- duplicate slugs from two files colliding
- cyclic embeds
- Typst compilation errors

A dangling link is reported precisely, including the slug it resolved to:

```
Warning: Dangling local link in `notes/carol`: `./nowhere` resolves to missing
section `notes/nowhere`.
```

Dangling links are warnings by design — drafting a link before its target exists
is a legitimate way to work. Use `wanshi check --strict` in CI or before a
release to turn every warning into a failure.

## 8. Build

```sh
wanshi build
```

Writes the site to `[build].output` (`./publish` by default), along with
`wanshi.json` and `wanshi.graph.json`. See
[Publishing and Workflows](publishing-and-workflows.md) for deployment, RSS, and
pretty URLs.

## Habits That Age Well

- **Run `wanshi check` often**, not just before publishing. The graph is the part
  of the system that rots silently.
- **Set `date` on everything.** It costs nothing and unlocks date-sorted footers,
  meaningful RSS ordering, and the date column in listings.
- **Use taxons consistently.** They are the vocabulary of your forest; a site
  where half the notes say `remark` and half say `note` reads as noise.
- **Link generously.** Backlinks are generated automatically, so every link you
  write pays for itself twice.
- **Use search to find what you half-remember.** Press `/` on any page. It
  matches body text, not just titles, so a phrase you recall from a note is
  enough — see [Search](configuration.md#search).
- **Let `index` be a real index.** Embedding your hub notes into `index.typ`
  gives the whole forest a sensible parent chain and a usable table of contents.
