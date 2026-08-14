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

Embedding a note makes the embedding page its parent, unless the note declares a
`parent` of its own or sits beside a directory index that already claims it.
That is worth knowing before embedding widely: it silently rearranges the
breadcrumb trail.

== When to embed instead of writing inline

Embed when the material is genuinely its own note — something you would want to
link to, cite, or find in a listing — but which also belongs in the middle of
this argument. If it is only ever going to be read here, write it here, or make
it a subtree: see #local("/guide/subtrees").
