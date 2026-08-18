#import "/_lib/wanshi.typ": *
#import "/_showcase.typ": showcase

#show: showcase

#metadata((
  "title": "The Banach fixed-point theorem",
  "taxon": "theorem",
  "date": "2026-08-09",
  "status": "stable",
  "numbering": "true",
))

A worked note: real mathematics, written the way any Typst document would be,
using the semantic subtree helpers from #local("/guide/subtrees").

The statements below are numbered, following the usual convention of a single
shared counter across statement types — so the theorem is 2 because the
definition took 1. The proof is left unnumbered, as proofs conventionally are.
How that works, and when it is a bad idea, is at the bottom of this page.

#definition(slug: "contraction", title: "Contraction")[
  Let $(X, d)$ be a metric space. A map $T colon X -> X$ is a *contraction* if
  there is some $q < 1$ with

  $ d(T(x), T(y)) <= q dot d(x, y) quad "for all" x, y in X. $

  The smallest such $q$ is the *Lipschitz constant* of $T$.
]

#theorem(title: "Banach, 1922")[
  Every contraction on a non-empty complete metric space has exactly one fixed
  point $x^*$, and for any starting $x_0$ the sequence $x_(n+1) = T(x_n)$
  converges to it.
]

#proof(numbering: false)[
  Iterating the contraction bound gives $d(x_(n+1), x_n) <= q^n d(x_1, x_0)$, so
  for $m > n$

  $ d(x_m, x_n) <= sum_(k=n)^(m-1) q^k d(x_1, x_0) <= q^n / (1 - q) d(x_1, x_0). $

  Since $q < 1$ the right-hand side vanishes, the sequence is Cauchy, and
  completeness supplies a limit $x^*$. Continuity of $T$ gives $T(x^*) = x^*$.
  If $y^*$ were another fixed point then $d(x^*, y^*) <= q dot d(x^*, y^*)$,
  which forces $d(x^*, y^*) = 0$.
]

#remark(title: "Why completeness is not optional")[
  On $X = (0, 1]$ with the usual metric, $T(x) = x/2$ is a contraction with no
  fixed point in $X$. The iterates converge, but to a point that was removed
  from the space.
]

= Two ways to write mathematics

Native Typst maths needs nothing added, and is what every formula above uses:

```typst
$ d(T(x), T(y)) <= q dot d(x, y) $
```

Typst emits these as MathML, which the browser lays out as text — on the
surrounding baseline, in this page's serif, in the same ink as the prose. Select
a formula above and you will find you can copy it. It scales with the reader's
font size, a screen reader can read it, and nothing is fetched to render it.

The `tex` helper instead passes TeX source through to KaTeX, rendered in the
browser:

```typst
#tex(`\frac{q^n}{1-q} d(x_1, x_0)`)
```

#tex(`\frac{q^n}{1-q} d(x_1, x_0)`) — useful when pasting TeX from elsewhere.

KaTeX is not loaded unless a site asks for it, so `tex` renders nothing until
you run

```sh
wanshi new katex
```

which writes the loader into `import-math.html`. This forest has done that,
which is the only reason the formula above appears. A site that never uses `tex`
makes no request to a maths CDN at all.

= Diagrams

Typst packages work normally. `auto-frame` renders a diagram to SVG and
`auto-figure` wraps it so it behaves in both HTML and paged output:

#auto-figure(auto-frame({
  import "@preview/fletcher:0.5.8" as fletcher: node, edge
  let y = 1.2
  fletcher.diagram(
    crossing-fill: rgb(0, 0, 0, 0),
    node((0, 0), $x_0$),
    node((1, 0), $x_1$),
    node((2, 0), $x_2$),
    node((3, 0), $x^*$),
    edge((0, 0), (1, 0), "->", $T$),
    edge((1, 0), (2, 0), "->", $T$),
    edge((2, 0), (3, 0), "-->", $dots$),
  )
}))

Formulas and diagrams are both recoloured to match the page, so they do not
arrive as black-on-white rectangles pasted onto a parchment background.

= Numbering

Numbering is decided once, by the note:

```typst
#metadata((
  "title": "The Banach fixed-point theorem",
  "numbering": "true",
))
```

Everything on the page follows from that — the statements above, and the
headings you have been reading. A site can set `[build].numbering` instead and
have every note start numbered; a note's own key overrides it either way.

Where the number lands depends on whether there is a word for it to attach to. A
block with a taxon reads `Definition 1.`, the digits inside the label, the way a
statement is written on paper. A heading has no taxon, so its number goes in
front of the title — `3. Numbering`, above. Both are the ordinary convention for
what they label.

An individual block overrides the note with `numbering:`, which is how the proof
below the theorem stays out of the sequence:

```typst
#proof(numbering: false)[ ... ]
```

Three details of how the counter behaves:

#subtree(title: "One counter, shared by every taxon", taxon: "observation")[
  The theorem above is numbered 2, not 1, because the definition took 1. There
  is no separate counter per statement type: definitions, theorems and remarks
  share one sequence, which is why a numbered remark can appear as `Remark 3.`

  Headings count separately. This section is 3 because two headings preceded it,
  not 6 — the three statements above are in their own sequence and do not push
  it along.

  Opting a block out, as the proof does, takes it out of the sequence entirely
  rather than giving it a number nobody cites.
]

#subtree(title: "Nesting adds a level", taxon: "observation")[
  Statements are numbered inside the section that holds them, so the
  observations here start again at `3.1` rather than continuing the `1, 2, 3`
  of the statements above. Nesting deepens it again: an observation inside `3.2`
  reads `3.2.1`.

  The three statements at the top of this page carry no section number because
  no heading had opened yet. Write one after a heading — `#outdent()` gets you
  back out — and it picks the top-level sequence up where it left off rather
  than restarting.
]

#subtree(title: "A number can name two things", taxon: "observation")[
  Look down this page: `3.1` names both *One counter, shared by every taxon* at
  the top of this section and the subsection *Why the rest of this forest does
  not use it* below it. Statements and subsections are separate sequences
  hanging off the same section number, so they collide by construction.

  That is what a paper does too — `Theorem 3.1` and `Section 3.1` coexist
  everywhere — and the label is what resolves it. Say "Observation 3.1" or
  "Section 3.1" and there is no ambiguity; say "3.1" alone and there is.
]

#subtree(title: "The page itself is not numbered", taxon: "observation")[
  The title at the top of this page reads `Theorem.`, with no number, while the
  statements under it are 1, 2 and 3. A page is the only thing at its level, so
  a number there would distinguish it from nothing — and taking one would push
  every number below it a level deeper, turning `Definition 1.` into
  `Definition 1.1.`
]

== Why the rest of this forest does not use it

A number is a property of *where a note is rendered*, not of the note. Embed the
same note in two pages and it takes a different number in each — first on one
page, third on another if two numbered blocks precede it. Insert a statement
earlier and everything after it renumbers.

So "see Theorem 2" is not a durable reference here. Within one long,
self-contained note like this one it is fine, because the context is fixed and
nothing embeds these statements elsewhere. Across the forest it is not: every
page starts its own counter, so several notes are `Theorem 1.`

The alternative is the one #local("/guide/links") describes. Writing
#local("/math/contraction") renders the target's own title, survives renaming,
and means the same thing from every page — which is what the slug being the
address space buys you.

== The two combine, and that is the arrangement to want

They are orthogonal, so there is no choice to make between them:

- `numbering` affects *display*, on the page rendering the block and in that
  page's table of contents.
- `local` links by *slug*, and renders the target's title.

The definition above shows as `Definition 1.` while the link to it a paragraph
ago rendered as "Contraction". Insert another statement above it and the
displayed number shifts; the link does not change at all, because nothing about
it ever mentioned a number. Numbers for reading, slugs for referring.

#subtree(title: "The number does not travel with the note", taxon: "observation")[
  #local("/math/contraction") is `Definition 1.` here, and on its own page it is
  plain `Definition.` — no number at all. The counter belongs to the page doing
  the rendering, so there is no such thing as *the* number of a note, only its
  number in one place.

  This is worth sitting with before writing "as we saw in Definition 1": read
  somewhere else, that sentence is pointing at nothing.
]

#subtree(title: "A link cannot show a live number", taxon: "observation")[
  There is no way to make a link render as `Definition 1` and keep it correct.
  The `text:` override takes a fixed string:

  ```typst
  #local("/math/contraction", text: "Definition 1")
  ```

  That is typed by hand, so it is exactly what goes stale when a statement is
  inserted above. Use `text:` to fix grammar, never to restate a number.
]

Numbering earns its place in writing that behaves like a paper. For notes meant
to be recombined, prefer the link.
