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

#proof[
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
