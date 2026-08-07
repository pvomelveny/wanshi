# Getting Started

wanshi is a single-command static site generator for interlinked notes written in
Typst. It reads `.typ` sources, builds HTML pages, copies static assets, and
can produce optional machine-readable indexes.

## Prerequisites

- The `wanshi` binary.
- [Typst](https://typst.app) installed and available on `PATH`. wanshi shells out
  to it, so the whole language and package ecosystem are available in notes.
- A local static file server for `wanshi serve`. The default serve command uses
  [`miniserve`](https://github.com/svenstaro/miniserve); install it with
  `cargo install miniserve` or `brew install miniserve`, or point
  `[serve].command` at any other static server.

## Create a Site

Create a new site in a new directory:

```sh
wanshi new site my-site
```

Or initialize a directory that already exists:

```sh
wanshi init .
```

Either way you get:

```
Wanshi.toml           configuration
trees/                source tree
trees/index.typ     starter index section
trees/_lib/wanshi.typ the bundled Typst library
assets/               static files, copied verbatim into the output
.gitignore
```

## Write Your First Note

```sh
wanshi new post notes/alice
```

This creates `trees/notes/alice.typ` with a starter skeleton. Fill it in:

```typst
#import "/_lib/wanshi.typ": *

#show: wanshi

#metadata((
  "title": "Alice",
  "taxon": "remark",
  "date": "2026-08-06",
))

A first note. It links to #local("/index").
```

The note's slug — its permanent identity and URL — is its path under `trees/`
without the extension, so this one is `notes/alice`, published at
`/notes/alice.html`.

Then link *to* it from `trees/index.typ` so it is reachable:

```typst
See #local("/notes/alice").
```

[Writing Notes](writing-notes.md) covers this loop in full, and
[Links and References](links-and-references.md) covers everything you can do with
`local`, `embed`, and references.

## Preview Locally

```sh
wanshi serve
```

Serve mode builds into the configured serve output directory, starts the
configured server, watches sources, config, themes, and assets, and rebuilds
incrementally on change. Leave it running while you write.

By default serve does *not* emit `wanshi.json` or `wanshi.graph.json`; pass
`--indexes` / `--graph` if you want them during preview.

```sh
wanshi serve --disable-reload   # turn off live reload
wanshi serve --watch-stats      # print dirty-path stats per rebuild
wanshi serve --no-server        # build and watch without starting a server
```

## Validate

```sh
wanshi check
```

Check parses the site and validates the section graph without writing anything.
It reports parse errors, missing or duplicate slugs, a missing index section,
dangling local links, Typst rendering errors, and embed-graph failures such as
cycles.

```sh
wanshi check --strict   # treat warnings as failures — use this in CI
```

## Build

```sh
wanshi build
```

Writes the publish output directory configured in `Wanshi.toml` (`./publish` by
default), copies assets, writes `main.css` and `main.js` unless they are inlined,
and emits:

- `wanshi.json` — section metadata index
- `wanshi.graph.json` — parent, reference, and backlink graph

To force a complete rebuild, ignoring caches:

```sh
wanshi build --no-cache
```

## Next Steps

- [Writing Notes](writing-notes.md) — the day-to-day authoring workflow.
- [Links and References](links-and-references.md) — connecting notes together.
- [Content Authoring](content-authoring.md) — metadata and Typst library reference.
- [Configuration Reference](configuration.md) — every `Wanshi.toml` key.
- [Command Reference](commands.md) — every command and flag.
- [Publishing and Workflows](publishing-and-workflows.md) — deployment, RSS, upgrades.
