#import "/_lib/wanshi.typ": *
#import "/_showcase.typ": showcase

#show: showcase

#metadata((
  "title": "A wanshi Example Forest",
  "date": "2026-08-14",
  "author": "the wanshi authors",
))

This forest documents itself. Every note here exists to demonstrate one wanshi
feature, and says in its own text how it was written — so reading a page beside
its source in `trees/` should be enough to work out how to do the same thing.

Build it and look around:

```sh
wanshi serve    # preview at the port in Wanshi.toml, with live reload
wanshi build    # write ./publish
```

#subtree(title: "Where to start", taxon: "remark")[
  If you are new, read #local("/guide/slugs") first — the slug is the one idea
  everything else is built on. After that the guide notes can be read in any
  order.

  The pages under #local("/refs/index") show a different pattern: notes marked as
  references, which collect in a footer rather than appearing inline.
]

== The sections

#children(sort: "slug", title: none)

== Everything, newest first

This listing is generated, not maintained. It updates when notes are added or
their dates change, which is the point of #local("/guide/listings").

#recent(count: 8, title: none, include-indexes: false)
