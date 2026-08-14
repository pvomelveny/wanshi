#import "/_lib/wanshi.typ": *
#import "/_showcase.typ": showcase

#show: showcase

#metadata((
  "title": "Guide",
  "date": "2026-08-14",
))

Each note here demonstrates one feature and explains how it was written. Read
them beside their sources in `trees/guide/`.

This page is a *directory index*: a file named `index.typ` inside a directory.
Two things follow from the name alone.

#subtree(title: "It becomes the parent of its neighbours", taxon: "observation")[
  Every note in `trees/guide/` gets this page as its parent without declaring
  one. That is why the breadcrumb above each of them leads back here, and why
  the listing below can be written as `#raw("#children()")` rather than by
  naming the notes.

  Precedence, when more than one candidate exists: an explicit `parent` in the
  note's metadata wins, then the page that embeds it, then the nearest directory
  index, then the root.
]

#subtree(title: "It gets a directory URL", taxon: "observation")[
  This page is published at `/guide/`, not `/guide/index.html` — trailing slash,
  no filename.

  The *slug* is still `guide/index`, and that is what links use:
  `#raw("local(\"/guide/index\")")`. Writing `#raw("local(\"/guide\")")` is a
  dangling link, because no note has that slug. The tidy URL is a rendering
  detail; the slug is the address.
]

#children(sort: "slug", title: none)
