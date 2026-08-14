# wanshi

A Typst-only static Zettelkästen site generator.

wanshi is a fork of [kodama](https://github.com/kodama-community/kodama) (see `NOTICE.md`).

## Status: a personal project

**wanshi is built primarily for its author's own use.** It is public because the
license requires it and because the code may be useful to read, not because it
is looking for users. In practice that means:

- No stability guarantees. Configuration keys, the Typst library API, and output
  markup change whenever it suits the author, without deprecation periods.
- Features exist because they were needed, not to round out a feature matrix.
  Gaps are gaps, and there is no roadmap for closing them.
- Issues and pull requests may go unanswered. Forking is welcome and probably
  the better option if you want something changed.

**If you want a maintained tool of this kind, use
[kodama](https://github.com/kodama-community/kodama)** — the upstream project
this was forked from. It has a community, a release cadence, and support for
Markdown alongside Typst. wanshi drops Markdown entirely, replaces the visual
design, and diverges in ways that suit one person's notes; none of that is an
improvement in general, just a different set of choices.

## Features

- Single binary, command-line program.

- Typst-only: content is written entirely in Typst, compiled via Typst installed on the user's device and embedded as SVG/HTML, so all Typst features are available. Markdown is not a supported input format.

- Ships a single built-in "Parchment & walnut" visual design (warm parchment background, Playfair Display headings, Source Serif 4 body text), including automatic recoloring of formulas and Typst-rendered images to match. Dark mode is not implemented yet. Users can still adjust any detail of the site style without rebuilding the wanshi tool itself.

- Organize Typst files in the manner of [Jon Sterling](https://www.jonmsterling.com/index/index.xml)'s [Forester](https://www.forester-notes.org/index/index.xml).

## Requirements

[Typst](https://typst.app) on your `PATH` — wanshi compiles every note with it.
`wanshi serve` additionally needs a static file server, by default
[`miniserve`](https://github.com/svenstaro/miniserve); see
[Getting Started](/docs/users/getting-started.md#prerequisites) for the
alternatives.

## Quick Start

```sh
wanshi new site my-site      # scaffold a project
cd my-site
wanshi new post notes/alice  # create a note
wanshi serve                 # preview with live reload
wanshi check --strict        # validate the note graph
wanshi build                 # write ./publish
```

A note is a Typst file that declares some metadata and links to its neighbours:

```typst
#import "/_lib/wanshi.typ": *

#show: wanshi

#metadata((
  "title": "Alice",
  "taxon": "remark",
  "date": "2026-08-06",
))

Builds on #local("/notes/semigroups"), cited in #local("/refs/knuth").
```

References and backlinks are derived from those links automatically.

# Docs

- [Getting Started](/docs/users/getting-started.md) — set up a site and build it.
- [Writing Notes](/docs/users/writing-notes.md) — the workflow for producing a new note.
- [Links and References](/docs/users/links-and-references.md) — linking, citations, backlinks.
- [User Documentation](/docs/users/README.md) — full index.
- [Developer Documentation](/docs/developers/README.md) — architecture and maintenance.

# License

wanshi is licensed under the GNU General Public License v3.0. See `LICENSE` and `NOTICE.md`.
