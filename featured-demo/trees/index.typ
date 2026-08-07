
#import "/_lib/wanshi.typ": *

#show: wanshi

// Recommended font
#set text(font: "Inria Sans")

#context metadata((
  "title": "wanshi example forest", //
  "taxon": "example", //
  "date": "October 12, 2025", //
  "author": "Anonymous",
  "0AFE": local("0AFE"), // Alice
))

#lorem(32) #local("./0AFF", text: [*Bob being linked*]). 

#embed("./0AFE", "Alice when embedded", numbering: true)

#lorem(48)

#subtree(slug: "0AFF", title: "Bob, subtree with slugs", taxon: "exegesis")[
  - #lorem(64) #local("0AFE", text: "Alice when being linked").
]

#exegesis(title: "Christina, anonymous subtree")[
  #import "@preview/fletcher:0.5.8" as fletcher: node, edge

  #let y = 1.5
  #auto-figure(auto-frame(fletcher.diagram(
    crossing-fill: rgb(0, 0, 0, 0),
    node((0, 0), $Z$),
    node((-1, y), $X$),
    node((1, y), $Y$),
    edge((-1, y), (0, 0), "->", $sigma_X$, bend: 15deg),
    edge((1, y), (0, 0), "->", $sigma_Y$, bend: -15deg),
    edge((0, 0), (1, y), "->", $sigma_Y^(-1)$, bend: -15deg),
    edge((-1, y), (1, y), "->", $sigma$, label-side: right),
  )))
]

#recent(count: 5, title: "Recent notes")
