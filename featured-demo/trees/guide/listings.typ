#import "/_lib/wanshi.typ": *
#import "/_showcase.typ": showcase

#show: showcase

#metadata((
  "title": "Listings that maintain themselves",
  "taxon": "exegesis",
  "date": "2026-08-13",
))

A listing is a query against the note graph, resolved after every note has been
read. Write one instead of a hand-maintained list of links and it stays correct
as the forest grows.

= children

Every note whose parent is this one. The usual content of a directory index:

```typst
#children(sort: "slug", title: "In this section")
```

#local("/guide/index") uses exactly that.

= recent

The most recently dated notes anywhere in the forest:

```typst
#recent(count: 5, title: "Latest")
```

#recent(count: 5, title: "Latest", include-indexes: false)

`include-indexes: false` leaves out directory index pages, which are usually
navigation rather than news. It works on every listing helper.

= by-taxon

Everything sharing a taxon — how a bibliography page is built:

```typst
#by-taxon("reference", title: "References in this forest")
```

#by-taxon("reference", title: "References in this forest")

= orphans

Notes nothing links to and nothing embeds. Written, then lost track of:

```typst
#orphans(title: "Unreachable")
```

#orphans(title: "Unreachable")

If that listing is not empty, `trees/strays/` is why — it holds a note left
deliberately unlinked so this page has something to show.

= The general form

All four are thin wrappers over `query`, which exposes the rest:

```typst
#query(
  from: "descendants",   // children | descendants | siblings | all | orphans | "prefix/"
  taxon: "definition",   // keep only this taxon
  key: "status",         // keep only notes carrying this key
  value: "stable",       // ... with this value
  sort: "date",          // any metadata key, or slug | title | taxon
  order: "desc",         // asc | desc
  limit: 10,
  title: "Stable definitions",
  include-indexes: false,
)
```

`from` also accepts a slug prefix, which is the escape hatch when the graph
relationships do not line up with what you want to list:

```typst
#query(from: "guide/", sort: "title", title: "Everything under guide/")
```

#query(from: "guide/", sort: "title", title: "Everything under guide/", include-indexes: false)

= A caveat worth knowing

Listings do not create links. A note that appears only in a listing is still an
orphan, because nothing points *at* it — which is why the orphan listing above
can show a note that is, in a sense, right there on this page.
