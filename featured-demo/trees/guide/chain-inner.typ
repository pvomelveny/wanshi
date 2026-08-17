#import "/_lib/wanshi.typ": *
#import "/_showcase.typ": showcase

#show: showcase

#metadata((
  "title": "The note at the end of the chain",
  "taxon": "remark",
  "date": "2026-08-12",
))

Nothing is embedded here, so the chain stops. Its breadcrumb points at
#local("/guide/chain-middle") rather than at the guide, because the note that
embeds a note becomes its parent — one link per step, all the way up.
