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

= Anonymous subtrees

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

= Named subtrees

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

= Semantic helpers

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
  This block was written `#example(title: "The helper form")[ ... ]`
  rather than as a `subtree` call with `taxon: "example"`. The output is
  identical.
]

= Nesting

Subtrees nest. Put one inside another and the page renders it as a section
within a section, indented, with its own collapse control; the table of contents
indents to match, so the structure is legible from the sidebar alone.

```typst
#definition(slug: "monoid", title: "Monoid")[
  A set with an associative operation and an identity.

  #example(slug: "monoid-strings", title: "Strings under concatenation")[ ... ]

  #remark(title: "A note two levels in")[ ... ]
]
```

#definition(slug: "monoid", title: "Monoid")[
  A set $M$ with an associative binary operation and an identity element $e$.

  #example(slug: "monoid-strings", title: "Strings under concatenation")[
    Finite strings over an alphabet, with concatenation and the empty string.
    Associative, and the empty string leaves any string unchanged — so this is a
    monoid, and it is the free one on that alphabet.

    #remark(title: "Three levels deep, anonymous")[
      Nesting has no depth limit. This block sits inside the example, which sits
      inside the definition, which sits inside the page. Being anonymous, it
      stops here: nothing can link to it.
    ]
  ]

  #remark(title: "Why not a separate file?")[
    A definition that only earns its place inside this argument is easier to
    read here than one directory over. Give it a slug — as this one has — and it
    becomes addressable without moving: #local("/guide/monoid-strings") is a
    real link to a subtree nested two levels down.
  ]
]

#subtree(title: "Nesting does not deepen the slug", taxon: "observation")[
  The example above is `guide/monoid-strings`, not `guide/monoid/strings`. A
  subtree slug is a single component resolved against the *host file's*
  directory, and that stays true however deeply the subtree is nested. Nesting
  is a statement about reading order, not about the address space.
]

= Subtree or separate file?

A separate file when the note stands on its own, has its own citations, or you
expect to link to it from elsewhere. A subtree when it only makes sense in the
argument you are currently making — and give it a slug later if that changes.
