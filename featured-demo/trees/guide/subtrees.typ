#import "/_lib/wanshi.typ": *
#import "/_showcase.typ": showcase

#show: showcase

#metadata((
  "title": "Subtrees",
  "taxon": "exegesis",
  "date": "2026-08-13",
))

A subtree is a note defined inside another note's file. It gets its own heading
and its own place in the table of contents, without needing a file.

== Anonymous subtrees

With no slug, a subtree is part of this page and nothing else. It cannot be
linked to, and does not appear in listings:

```typst
#subtree(title: "A passing thought", taxon: "remark")[
  Some text.
]
```

#subtree(title: "A passing thought", taxon: "remark")[
  This block was written inline. It has a heading and a taxon, and it shows up
  in the sidebar, but no other note can point at it.
]

== Named subtrees

Give it a slug and it becomes addressable — a real note that happens to live
inside this file:

```typst
#subtree(slug: "subtrees-named", title: "A named subtree")[
  Some text.
]
```

#subtree(slug: "subtrees-named", title: "A named subtree", taxon: "observation")[
  This one is addressable as `guide/subtrees-named`, so
  #local("/guide/listings") can list it and any note can link to it, exactly as
  if it had its own file.
]

The slug is a *single component*, resolved against the host note's directory:
written inside `trees/guide/subtrees.typ`, `subtrees-named` becomes
`guide/subtrees-named`. Spelling out the directory is an error — a slug
containing `/` is rejected with a message saying so.

== Semantic helpers

The bundled library wraps `subtree` for the taxons that come up most, so the
common case reads as what it is:

```typst
#definition(title: "Contraction")[ ... ]
#theorem(title: "Banach fixed point")[ ... ]
#proof[ ... ]
```

Each is exactly `subtree` with the taxon filled in. The full set is
`exegesis`, `definition`, `proposition`, `remark`, `conjecture`, `postulate`,
`claim`, `observation`, `fact`, `hypothesis`, `axiom`, `lemma`, `theorem`,
`corollary`, `example`, and `proof`. See #local("/math/fixed-point") for them in
use.

#example(title: "The helper form")[
  This block was written `#raw("#example(title: \"The helper form\")[ ... ]")`
  rather than as a `subtree` call with `taxon: "example"`. The output is
  identical.
]

== Subtree or separate file?

A separate file when the note stands on its own, has its own citations, or you
expect to link to it from elsewhere. A subtree when it only makes sense in the
argument you are currently making — and give it a slug later if that changes.
