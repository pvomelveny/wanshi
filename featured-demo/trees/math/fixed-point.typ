#import "/_lib/wanshi.typ": *
#import "/_showcase.typ": showcase

#show: showcase

#metadata((
  "title": "The Banach fixed-point theorem",
  "taxon": "theorem",
  "date": "2026-08-09",
  "status": "stable",
))

A worked note: real mathematics, written the way any Typst document would be,
using the semantic subtree helpers from #local("/guide/subtrees").

The statements below are numbered, following the usual convention of a single
shared counter across statement types — so the theorem is 2 because the
definition took 1. The proof is left unnumbered, as proofs conventionally are.
How that works, and when it is a bad idea, is at the bottom of this page.

#definition(slug: "contraction", title: "Contraction", numbering: true)[
  Let $(X, d)$ be a metric space. A map $T colon X -> X$ is a *contraction* if
  there is some $q < 1$ with

  $ d(T(x), T(y)) <= q dot d(x, y) quad "for all" x, y in X. $

  The smallest such $q$ is the *Lipschitz constant* of $T$.
]

#theorem(title: "Banach, 1922", numbering: true)[
  Every contraction on a non-empty complete metric space has exactly one fixed
  point $x^*$, and for any starting $x_0$ the sequence $x_(n+1) = T(x_n)$
  converges to it.
]

#proof[
  Iterating the contraction bound gives $d(x_(n+1), x_n) <= q^n d(x_1, x_0)$, so
  for $m > n$

  $ d(x_m, x_n) <= sum_(k=n)^(m-1) q^k d(x_1, x_0) <= q^n / (1 - q) d(x_1, x_0). $

  Since $q < 1$ the right-hand side vanishes, the sequence is Cauchy, and
  completeness supplies a limit $x^*$. Continuity of $T$ gives $T(x^*) = x^*$.
  If $y^*$ were another fixed point then $d(x^*, y^*) <= q dot d(x^*, y^*)$,
  which forces $d(x^*, y^*) = 0$.
]

#remark(title: "Why completeness is not optional", numbering: true)[
  On $X = (0, 1]$ with the usual metric, $T(x) = x/2$ is a contraction with no
  fixed point in $X$. The iterates converge, but to a point that was removed
  from the space.
]

== Two ways to write mathematics

Native Typst maths is rendered at build time and embedded as SVG. It needs no
JavaScript, and it is what every formula above uses:

```typst
$ d(T(x), T(y)) <= q dot d(x, y) $
```

The `tex` helper instead passes TeX source through to KaTeX, rendered in the
browser:

```typst
#tex(`\frac{q^n}{1-q} d(x_1, x_0)`)
```

#tex(`\frac{q^n}{1-q} d(x_1, x_0)`) — useful when pasting TeX from elsewhere,
at the cost of a CDN dependency. Native Typst maths is the better default;
`import-math.html` can drop KaTeX entirely if you never use `tex`.

== Diagrams

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

== Numbering

Every subtree and embed takes `numbering`, off by default. Turning it on
prefixes the *taxon* with a counter, which is what produces `Definition 1.` and
`Theorem 2.` above:

```typst
#definition(title: "Contraction", numbering: true)[ ... ]
#theorem(title: "Banach, 1922", numbering: true)[ ... ]
#proof[ ... ]
```

Two details of how the counter behaves:

#subtree(title: "One counter, shared by every taxon", taxon: "observation")[
  The theorem above is numbered 2, not 1, because the definition took 1. There
  is no separate counter per statement type. That matches the common convention
  of a single shared counter, and it is the reason a numbered remark can appear
  as `Remark 3.`

  Leaving `numbering` off, as the proof does, takes a block out of the sequence
  entirely rather than giving it a number nobody cites.
]

#subtree(title: "Nesting adds a level", taxon: "observation")[
  A numbered subtree inside a numbered subtree counts as `1.1`, `1.2`, and so
  on, so a definition with numbered sub-remarks reads the way it would on paper.
]

=== Why the rest of this forest does not use it

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

=== The two combine, and that is the arrangement to want

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
