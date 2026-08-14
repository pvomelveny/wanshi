#import "/_lib/wanshi.typ": *
#import "/_showcase.typ": showcase

#show: showcase

#metadata((
  "title": "Metadata",
  "taxon": "reference",
  "date": "2026-08-11",
  "author": "the wanshi authors",
  "status": "stable",
  "see-also": local("/guide/slugs"),
))

Every note opens with a `metadata` call. The header above this text — taxon,
title, date, author, status, and that link — is rendered entirely from the
block at the top of `trees/guide/metadata.typ`:

```typst
#metadata((
  "title": "Metadata",
  "taxon": "reference",
  "date": "2026-08-11",
  "author": "the wanshi authors",
  "status": "stable",
  "see-also": local("/guide/slugs"),
))
```

== Structural keys

`title` and `taxon` set the heading. `date` is conventional but privileged: it
gets its own column, and it is the natural sort key for listings and feeds.

`parent` overrides the inferred parent. `page-title` overrides the browser
title when the rendered title carries markup that should not appear in a tab.

Booleans are the *strings* `"true"` and `"false"`, never bare Typst booleans:

```typst
#metadata((
  "title": "A note nobody should link back to",
  "asback": "false",
))
```

An unrecognised value fails the build and names the note and the key, so a typo
here is loud rather than silent.

== Custom keys

Anything not structural is a custom key. `status` above is one. Custom keys are
preserved in `wanshi.json`, shown in the header, and usable as a sort key.

They may also hold rich content, which structural keys may not — `see-also`
above is a real link, resolved like any other:

```typst
"see-also": local("/guide/slugs"),
```

That is the one place metadata and the note graph meet: a link written inside
metadata still creates a backlink on its target.

== Taxons

A taxon is a display category — `definition`, `remark`, `theorem`, `reference`.
It carries no behaviour on its own, but listings can filter on it, which is how
#local("/refs/index") collects its entries.

=== They are free-form

There is no fixed set. A taxon is an ordinary string, rendered with its first
letter capitalised and a period appended. Invent one whenever the vocabulary you
already have does not fit:

```typst
#metadata((
  "title": "A note in the margin",
  "taxon": "marginalia",
))
```

The subtree below uses exactly that, and the listing under it finds it. Neither
required registering the word anywhere.

#subtree(slug: "marginalia", title: "A note in the margin", taxon: "marginalia")[
  Tagged `marginalia`, a taxon this forest invented on the spot. It renders like
  any other, and `by-taxon` treats it like any other.
]

#by-taxon("marginalia", title: "Everything tagged marginalia")

The semantic helpers in #local("/guide/subtrees") — `#theorem`, `#definition`,
and the rest — are *not* an enumeration of what is allowed. Each is a one-line
wrapper that fills in `taxon:` on a `subtree` call, so `#theorem(..)` and
`#subtree(taxon: "theorem", ..)` produce identical output. They save typing;
they do not constrain.

=== Two things to know

#subtree(title: "`reference` is the one reserved word", taxon: "observation")[
  Any taxon beginning with `reference` marks a note as a citation, which changes
  where links to it are rendered — see #local("/guide/links"). Every other
  taxon is inert: a label, and something listings can filter on.
]

#subtree(title: "Matching ignores case", taxon: "observation")[
  `#by-taxon("MARGINALIA")` finds notes tagged `marginalia`. Capitalisation in
  metadata is a display choice, not part of the identity.

  What listings *cannot* see is a taxon on an anonymous subtree. Only sections
  with slugs are addressable, so a taxon on a subtree without one labels the
  heading and nothing more.
]
