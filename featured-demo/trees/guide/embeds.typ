#import "/_lib/wanshi.typ": *
#import "/_showcase.typ": showcase

#show: showcase

#metadata((
  "title": "Embedding one note in another",
  "taxon": "exegesis",
  "date": "2026-08-12",
))

`embed` pulls another note's content into this page, rendered in place:

```typst
#embed("/refs/typst", "Typst, embedded here in full")
```

The embedded note keeps its own identity — it is still published at its own URL,
still appears in listings, still owns its backlinks. Embedding shows it in a
second place; it does not move or copy it.

#embed("/refs/typst", "Typst, embedded here in full")

== Options

```typst
#embed("/refs/typst", "A title", numbering: true, open: false, catalog: false)
```

- `numbering` — number the embedded section.
- `open` — whether it starts expanded. `false` collapses it behind a summary.
- `catalog` — whether it appears in the table of contents.

Collapsed, with no catalog entry:

#embed("/refs/knuth-1984", "Knuth, collapsed", open: false, catalog: false)

== Embedding and parents

Embedding a note makes the embedding page its parent. That is the part worth
knowing before embedding widely, because it rearranges the breadcrumb trail
without saying so.

Both notes embedded above are references, and both would otherwise have been
filed under *this* page — a reference note reached by a trail through the guide.
Follow #local("/refs/typst") and the breadcrumb reads *References*, because it
declares one:

```typst
#metadata((
  "title": "Typst",
  "taxon": "reference",
  "parent": "refs/index",
))
```

An explicit `parent` beats an embedding one and never loses to it.

== Embeds chain

An embedded note may embed one of its own, and the whole chain renders in
place. The block below is #local("/guide/chain-middle"), which embeds
#local("/guide/chain-inner") — so this page shows three levels at once,
indented, with the table of contents matching.

#embed("/guide/chain-middle", "A note that embeds another")

Structurally this is what #local("/guide/subtrees") does with nesting, and it
looks the same on the page. The difference is where the nesting lives: a subtree
is written inside its host file, while a chain of embeds is assembled from
separate notes, each of which is still an ordinary page of its own.

Parents follow the chain a link at a time — the innermost note's breadcrumb
points at the note that embeds it, not at this page.

#subtree(title: "A chain can close into a loop", taxon: "observation")[
  Because embedding is by reference rather than containment, a chain can come
  back round on itself. Nesting subtrees cannot: they are literally inside one
  file.

  wanshi refuses to build one, naming the whole path:

  ```
  Caused by:
      cyclic embed detected: a -> b -> c -> a
  ```

  Worth having seen once. A two-note loop is obvious; the one that catches you
  is assembled from four or five files written weeks apart.
]

=== Embedding the same note in several places

Nothing stops it, and it is often the point — a definition used by three
arguments should be written once and shown in all three. Each page renders it in
full, and it stays a note of its own with its own URL.

But a note has exactly one parent, because the breadcrumb is a single trail. So
when several pages embed the same note and it declares no parent, one embedder
is chosen and the rest are not represented. Which one wins follows compilation
order, not intent — renaming an unrelated file can move it. wanshi warns:

```
Warning: `shared-def` is embedded in both `alpha` and `beta`; using `alpha` as its parent.
         Set `"parent"` in its metadata to choose deliberately.
```

Declaring the parent is the whole fix. It silences the warning, pins the
breadcrumb where it belongs, and changes nothing about the embedding — every
page still shows the note in full.

The one-parent rule only constrains the breadcrumb. Backlinks stay many-to-many,
so all the embedding pages remain visible from the note regardless of which one
became its parent.

== When to embed instead of writing inline

Embed when the material is genuinely its own note — something you would want to
link to, cite, or find in a listing — but which also belongs in the middle of
this argument. If it is only ever going to be read here, write it here, or make
it a subtree: see #local("/guide/subtrees").
