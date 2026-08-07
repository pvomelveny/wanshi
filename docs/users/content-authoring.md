# Content Authoring

This is the reference for what goes *inside* a `.typ` source file: the metadata
keys wanshi understands and the helpers its Typst library provides.

If you are looking for the process of writing a note, start with
[Writing Notes](writing-notes.md). For the linking primitives in depth, see
[Links and References](links-and-references.md).

## Sections and Slugs

wanshi turns each source file into one or more **sections**. A section is the
unit that receives metadata, a slug, an HTML page, graph relationships,
backlinks, references, and optional footer content.

The configured source tree defaults to `trees/`. Files ending in `.typ` are
section sources; the slug is the tree-relative path with the extension removed:

- `trees/index.typ` becomes `index`
- `trees/notes/alice.typ` becomes `notes/alice`

`.typst` is also accepted, for sites created before `.typ` was adopted. The two
extensions can coexist in one tree, and because the slug is the path *minus* the
extension, renaming `alice.typst` to `alice.typ` changes no slug and no URL. Two
files that would produce the same slug — `alice.typ` and `alice.typst` side by
side — are a hard error.

### Helpers: the `_` Prefix

**Files and directories whose names begin with `_` or `.` are skipped.** They are
never notes, never get a slug, and never produce a page — but they are still
ordinary Typst that your notes can import. This is how shared code lives next to
the notes that use it:

```
trees/
  index.typ          a note
  _macros.typ        a helper, not a note
  _lib/wanshi.typ    the bundled library
  notes/
    alice.typ        a note
    _shared.typ      a helper, not a note
```

The rule applies at every depth, so `notes/_shared.typ` is as excluded as
`_macros.typ`. See [Sharing Typst Code](#sharing-typst-code) below for how to
import them.

An `index` section is strongly recommended. Without it, wanshi still compiles
everything, but validation and builds warn, because it is the compiler's entry
point and the default parent for otherwise unattached sections.

## Anatomy of a Source File

```typst
#import "/_lib/wanshi.typ": *

#show: wanshi

#metadata((
  "title": "Alice",
  "taxon": "remark",
  "date": "2026-05-16",
))

Content written in Typst.
```

- **`#import`** — pulls in the library. `"/_lib/wanshi.typ"` (root-absolute,
  resolved against `[build].typst-root`) works at any directory depth;
  `"_lib/wanshi.typ"` only works for files at the top of the tree.
- **`#show: wanshi`** — installs the show rules that convert Typst math into
  correctly-aligned inline and block SVG in HTML output. Omit it and equations
  render wrong.
- **`#metadata(...)`** — a dictionary of the keys below.

Everything after that is ordinary Typst, compiled by your local `typst`
installation, so the whole language and the Typst Universe package ecosystem are
available.

## Metadata Reference

### Identity and Display

| Key | Type | Meaning |
| --- | --- | --- |
| `title` | content or string | Section title. May be rich Typst content. Used in the header, as the default text of inbound links, and as the fallback page title. |
| `page-title` | string | Plain-text browser title override. Defaults to `title` with markup stripped. |
| `taxon` | string | Display category — `definition`, `remark`, `theorem`, `reference`, … Rendered as a label before the title. |
| `data-taxon` | string | Plain taxonomy attribute. Auto-derived from `taxon`; override only if you need the attribute to differ from the label. |
| `date` | string | Conventional, but privileged: it gets its own column in page headers and catalog entries, and is the natural `footer-sort-by` key. |

### Graph and Navigation

| Key | Type | Default | Meaning |
| --- | --- | --- | --- |
| `parent` | slug | inferred | Explicit parent for "previous level" navigation. Beats both the embedding page and the directory index — see [Grouping Notes into Categories](writing-notes.md#grouping-notes-into-categories). |
| `backlinks` | bool | `true` | Whether this section *displays* backlinks. |
| `transparent-backlinks` | bool | `false` | Display backlinks even when this section is embedded elsewhere (except in footers). |
| `references` | bool | `true` | Whether referenced sections appear in this section's footer. |
| `asref` | bool | `[build].asref` | Whether this section is treated as a reference when linked to. |
| `asback` | bool | `true` | Whether this section's outbound links generate backlinks on their targets. |
| `collect` | bool | `false` | Marks the page as a collection page; excludes it from RSS items. |
| `footer-mode` | `embed` \| `link` | `[build].footer-mode` | Footer rendering for this section. |
| `footer-sort-by` | metadata key | `[build].footer-sort-by` | Sort key for this section's footer entries. |

Booleans are written as the strings `"true"` and `"false"`. An unrecognised value
is a build error naming the section and key.

### Custom Keys

Any key not listed above is a custom key. Custom keys are:

- preserved in `wanshi.json`,
- rendered in the page header's metadata row (value only — the key name is not
  printed),
- usable as a `footer-sort-by` sort key,
- allowed to contain rich content, including `local()` links.

```typst
#metadata((
  "title": "Free monoids",
  "author": "Patrick",
  "see-also": local("/notes/monoids"),
))
```

The structural keys in the two tables above must be plain text; supplying content
for one of them fails the build with an explicit message. Only custom keys may
be rich.

## Library Reference

Everything below comes from `trees/_lib/wanshi.typ`, imported with
`#import "/_lib/wanshi.typ": *`.

### Linking

```typst
#local(slug, text: none)
#embed(url, title, numbering: false, open: true, catalog: true, display-options: false)
#external(dest, content)
```

- **`local`** — inline link to another section. Without `text:`, the label is the
  target's title.
- **`embed`** — render another section inside this one. `title` is a per-embed
  title override. Sets the parent of the target; propagates the target's
  references upward when `open` is true.
- **`external`** — underlined off-site link. Not part of the graph.

Fully documented in [Links and References](links-and-references.md).

### Subtrees

A subtree lets one source file define additional sections inline. Each named
subtree gets its own slug and its own page.

```typst
#subtree(slug: "groups", title: "Groups", taxon: "definition")[
  A group is a set with an associative operation, identity, and inverses.
]
```

Parameters:

| Parameter | Default | Meaning |
| --- | --- | --- |
| `slug` | `none` | Explicit slug, resolved relative to the containing **directory**. Omit for an anonymous subtree. |
| `title` | `none` | Title for the generated section. |
| `taxon` | `none` | Taxon for the generated section. |
| `numbering` | `false` | Number the section. |
| `open` | `true` | Whether the `<details>` starts expanded. |
| `catalog` | `true` | Include in the table of contents. |

Anonymous subtrees (no `slug:`) get an internal identifier, are never linkable,
and are stripped from `wanshi.json`, `wanshi.graph.json`, and the
reference/backlink graph.

#### Semantic Helpers

Sixteen sugar helpers wrap `subtree` with a preset taxon. They take the same
parameters, and `taxon:` still overrides the preset:

`exegesis`, `definition`, `proposition`, `remark`, `conjecture`, `postulate`,
`claim`, `observation`, `fact`, `hypothesis`, `axiom`, `lemma`, `theorem`,
`corollary`, `example`, `proof`.

```typst
#definition(slug: "groups", title: "Groups")[
  A group is a set with an associative operation, identity, and inverses.
]

#theorem(slug: "lagrange", title: "Lagrange")[
  The order of a subgroup divides the order of the group.
]

#proof[
  Consider the left cosets.
]
```

The last one is anonymous — a proof usually belongs to the statement above it and
does not need its own URL.

### Listings

A listing renders a set of *other* sections, chosen when the site is built. It is
how a hub page stays current without being edited every time you add a note.

```typst
#query(
  from: "all",     // which sections to consider
  taxon: none,     // keep only this taxon
  key: none,       // keep only sections carrying this metadata key…
  value: none,     // …and, with `value`, only when it equals this
  sort: "date",    // metadata key, or "slug" / "title" / "taxon"
  order: "asc",    // "asc" or "desc"
  limit: none,     // keep at most this many
  title: none,     // optional heading above the list
)
```

`from` accepts:

| Value | Selects |
| --- | --- |
| `"children"` | Sections whose parent is this one |
| `"descendants"` | The whole subtree beneath this one |
| `"siblings"` | Sections sharing this one's parent |
| `"all"` | Every visible section |
| `"orphans"` | Sections nothing links to and nothing embeds. Includes unlinked directory index pages; excludes the root `index`. |
| `"notes/"` | Every section whose slug starts with that prefix |

Four shorthands cover the common cases:

```typst
#children()                      // this page's direct children, oldest first
#recent(count: 10)               // newest notes anywhere in the forest
#by-taxon("definition")          // every definition, by title
#orphans()                       // notes that fell out of the graph
```

A listing never includes the page it is written on. Rows show taxon, title, and
the date column, and link to the section's own page.

See [Building Pages That Maintain Themselves](writing-notes.md#building-pages-that-maintain-themselves)
for how to use these in practice.

### Figures and Diagrams

```typst
#auto-frame(content)
#auto-figure(content)
```

`auto-frame` renders Typst-drawn content as inline SVG in HTML output while
leaving paged output untouched. `auto-figure` centres content, using a real
`<figure>` element in HTML. Package-drawn diagrams need both:

```typst
#import "@preview/fletcher:0.5.8" as fletcher: node, edge

#auto-figure(auto-frame(fletcher.diagram(
  node((0, 0), $Z$),
  node((-1, 1.5), $X$),
  edge((-1, 1.5), (0, 0), "->", $sigma_X$, bend: 15deg),
)))
```

### Math

Inline and block equations written in normal Typst syntax are handled
automatically by the `#show: wanshi` rule — inline math is rendered as SVG with
its baseline corrected, and block math is wrapped in a centred container.

For KaTeX-rendered math instead, `tex()` wraps a raw string in `$…$` delimiters
for the bundled KaTeX auto-render pass:

```typst
#tex(`\frac{1}{2}`)
```

Generate VS Code snippets for KaTeX with `wanshi snip --katex`.

### Constants

`wanshi.typ` also exports the design system's values, for content that needs to
match the site's palette in paged output: `html-font-size`, `ink-color`,
`muted-color`, `accent-color`, `slug-color`, `taxon-color`, and
`heading-font-weight`.

```typst
#set text(size: html-font-size, top-edge: "bounds", bottom-edge: "bounds")
```

## Sharing Typst Code

Anything you want to reuse across notes — a figure, a helper function, shared
styling — goes in a file or directory whose name starts with `_`. Those are
skipped by section discovery, so they never become pages, and they are imported
like any other Typst module:

```typst
// trees/_figures/shapes.typ   (or just trees/_shapes.typ)
#let brand-circle = circle(radius: 20pt, fill: maroon)
```

```typst
// any note, at any depth
#import "/_lib/wanshi.typ": *
#import "/_figures/shapes.typ": brand-circle

#show: wanshi
#metadata(( "title": "Uses a shared figure" ))

#auto-figure(auto-frame(brand-circle))
```

The figure is defined once and rendered inline into every note that imports it.
Because it is inlined rather than linked, there is no URL to keep correct, it is
unaffected by `base-url`, and it participates in the site's color handling like
any other Typst content.

Use a single `_`-prefixed file for one-off helpers and a `_`-prefixed directory
once you have several. Either way the exclusion is the same, and editing a helper
during `wanshi serve` rebuilds every note that imports it.

## Assets

Put static files in the configured assets directory (`assets/` by default).
wanshi copies the whole directory into the output on both build and serve, and
recognises asset links by checking whether the resolved path falls inside the
assets root.

## Paged Output

Every helper is written to degrade gracefully when Typst targets paged output
rather than HTML — metadata renders as a styled heading block, subtrees render as
inline headed blocks, and links render as underlined text. This means you can
compile any single note straight to a PDF for reading or printing, without wanshi
in the loop:

```sh
typst compile --root trees trees/notes/alice.typ alice.pdf
```

`--root trees` is what makes the root-absolute `#import "/_lib/wanshi.typ"`
resolve; match it to your `[build].typst-root`.
