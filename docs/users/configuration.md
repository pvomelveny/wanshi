# Configuration Reference

wanshi reads `Wanshi.toml` by default. Commands that accept `--config` can point at another file. If the specified config path is not found, wanshi searches from the parent directory in the way used by project commands, so commands can often be run from inside a site.

An empty configuration is valid because every section has defaults.

## `[wanshi]`

```toml
[wanshi]
trees = "trees"
assets = "assets"
base-url = "/"
theme-lock = true
themes = []
```

- `trees`: source directory for `.typ` sections.
- `assets`: static assets directory copied into the output, under the same
  directory name. Renaming it moves both the copied files and the links wanshi
  generates into them, so it is the way to avoid a clash when another site owns
  a directory of that name — see [Sharing an output directory](publishing-and-workflows.md#sharing-an-output-directory).
- `base-url`: URL prefix used for generated links. Use `/` for root-relative local output, or an absolute `https://.../` URL for RSS publishing.
- `theme-lock`: hides the theme picker when true. **Defaults to `true` in wanshi**, because wanshi ships exactly one design and the picker would otherwise be empty. Set it to `false` if you add entries to `themes`.
- `themes`: list of HTML fragment files, resolved from the project root, whose contents are inlined into the theme picker. See [Themes](#themes) below.

## `[toc]`

```toml
[toc]
placement = "right"
sticky = true
mobile-sticky = true
max-width = "45ex"
```

- `placement`: `left` or `right`.
- `sticky`: whether the table of contents stays fixed while scrolling on larger screens.
- `mobile-sticky`: whether sticky behavior is used on mobile.
- `max-width`: CSS width value for the table of contents.

## `[text]`

```toml
[text]
edit = "[edit]"
toc = "Table of Contents"
references = "References"
backlinks = "Backlinks"
search = "Search  /"
```

These values customize interface labels in generated pages.

## `[build]`

```toml
[build]
typst-root = "trees"
short-slug = false
pretty-urls = false
footer-mode = "link"
footer-sort-by = "slug"
inline-css = false
inline-script = false
asref = false
output = "./publish"
search = true
search-content = true
# edit = "https://example.com/edit/"   # optional; unset by default
```

- `typst-root`: root directory passed to Typst compilation. This is what makes root-absolute imports such as `#import "/_lib/wanshi.typ"` resolve inside notes.
- `short-slug`: display only the last path segment of a slug in headers and links.
- `pretty-urls`: emits links without `.html` suffixes.
- `footer-mode`: `link` for compact footer entries, or `embed` for full embedded footer content.
- `footer-sort-by`: sort key for reference and backlink footer entries. Common values are `slug`, `date`, `taxon`, `title`, or a custom metadata key. See [Links and References](links-and-references.md#footer-rendering).
- `inline-css`: embeds wanshi's CSS into each page instead of writing `main.css`.
- `inline-script`: embeds wanshi's JavaScript into each page instead of writing `main.js`.
- `asref`: global default for whether local link targets are treated as references. Individual sections override it with `asref` metadata, and a `reference` taxon makes a section a reference target regardless of this setting.
- `output`: publish output directory used by `wanshi build`.
- `edit`: optional edit URL prefix for generated edit links in publish builds. Unset by default, in which case no edit link is rendered.
- `search`: emit `wanshi.search.json` and show the sidebar search box. Set to `false` to remove both.
- `search-content`: index note body text as well as titles. See [Search](#search) below.

## `[serve]`

```toml
[serve]
edit = "vscode://file/"
output = "./.cache/publish"
command = ["miniserve", "<output>", "--index", "index.html", "--pretty-urls"]
```

- `edit`: edit URL prefix for local preview.
- `output`: output directory used by `wanshi serve`.
- `command`: server command and arguments. The literal `<output>` is replaced with the serve output directory.

The default command expects `miniserve` to be installed. You can replace it with any static server command that serves the output directory.

## `[publish]`

```toml
[publish]
rss = false
```

- `rss`: when true, `wanshi build` writes `feed.xml` and every page gains a
  `<link rel="alternate">` pointing at it, so readers discover the feed
  automatically.

RSS publishing requires `[wanshi].base-url` to be an absolute `http://` or
`https://` URL with a host — a relative one is rejected before the feed is
written. Feeds are publish-only: `wanshi serve` writes none and its pages do not
advertise one.

See [Publishing a Feed](publishing-and-workflows.md#publishing-a-feed) for what
ends up in the feed and how dates are handled.

## Search

With `[build].search = true` (the default) wanshi writes `wanshi.search.json`
and puts a search box at the top of the table-of-contents sidebar. Press `/` to
focus it from anywhere on the page, `Escape` to clear. Results replace the
sidebar contents while you type and restore when the box is empty.

Search is entirely client-side — there is no server, and the index is fetched
lazily the first time you use the box, then cached for the session. Readers with
JavaScript disabled never see the box at all.

Matching is by **token prefix**, and every term must match: typing `mono` finds
"Monoid", and `free mono` narrows rather than widens. Title matches rank above
body matches.

### What Gets Indexed

`search-content = true` (the default) indexes note body text. It does **not**
ship your prose. The index is inverted — it maps each word to the notes
containing it, so word order, punctuation and repetition are all discarded and a
word used in a hundred notes is stored once. You cannot reconstruct a note from
it, and it is roughly a third the size of shipping the text.

Inline math and diagrams are excluded: Typst renders them as SVG, and indexing
that would fill the index with coordinates and colour values. The practical
consequence is that **a note is not findable by a symbol it contains** — search
for the words around a formula, not the formula.

Set `search-content = false` to index only titles, taxons, and slugs. That makes
the index negligible in size at any scale, but you can then only find a note by
roughly what it was called — a note about associativity titled "Free monoids"
becomes unfindable by its content.

As a rough guide, body indexing costs on the order of 30–70 KB gzipped for a few
hundred notes.

The placeholder text is configurable via `[text].search`.

## Customizing the Page Head

wanshi ships one built-in design ("Parchment & walnut"), but you can adjust the
generated page without rebuilding the binary. Six optional files are read from
the **project root** — next to `Wanshi.toml`, not inside `trees/`.

Four of them affect the `<head>`:

| File | Default | Effect |
| --- | --- | --- |
| `import-meta.html` | empty | Appended to the head. Use for favicons, Open Graph tags, analytics. |
| `import-style.html` | empty | Appended to the head. Use for a `<style>` block or an extra stylesheet that overrides `main.css`. |
| `import-font.html` | bundled Google Fonts link (Playfair Display, Source Serif 4, Fira Code) | **Replaces** the built-in font imports. Use to self-host fonts or drop the CDN dependency. |
| `import-math.html` | empty | Appended to the head. Holds the KaTeX loader, if you want one — `wanshi snip --katex` writes it. Only needed for the `tex()` helper; ordinary equations are MathML and need nothing. |

Three of them are appended to whatever wanshi already emits. `import-font.html`
is the exception: it **replaces** the built-in font links, so copy the bundled
content out of the wanshi source before editing if you only want a tweak.

Nothing here is needed to render mathematics. Equations are MathML, which the
browser lays out on its own; `import-math.html` exists for the `tex()` helper,
which hands TeX source to KaTeX instead. A site with no such file makes no
requests to a maths CDN at all.

`import-style.html` is the right hook for restyling. wanshi's own stylesheet is
written to `main.css` in the output, so a `<style>` block here — or a `<link>` to
a stylesheet you place in `assets/` — can override any of it without touching the
binary.

### Favicon

Every page links an icon from the assets directory, and `wanshi new site` ships
one so the link is never left dangling. **To change it, replace the file:**

```sh
cp my-icon.ico assets/favicon.ico
```

The name is fixed — pages always link `<assets>/favicon.ico` — but the directory
follows `[wanshi].assets`, so renaming that moves the link with it. An `.ico`
holding several sizes (16, 32, 48) lets the browser pick per context; a
single-size file works too.

**Do not simply delete it.** The link is emitted unconditionally, so removing
the file leaves every page requesting one that is not there. If you want no
icon, keep a file at that path rather than removing it.

To point at an icon the surrounding site already publishes, see
[Using the host site's favicon](publishing-and-workflows.md#using-the-host-sites-favicon).

### Page chrome

The other two put your own markup inside `<body>`, which is what you want when
wanshi's pages sit inside a larger site and need to carry its navigation:

| File | Default | Effect |
| --- | --- | --- |
| `import-header.html` | empty | Inserted at the very top of `<body>`, above wanshi's breadcrumb. |
| `import-footer.html` | empty | Inserted at the very end of `<body>`, below the content grid. |

Both are inserted verbatim and sit **outside** `#grid-wrapper`, so they span the
full page width and do not disturb the article/sidebar layout:

```html
<!-- import-header.html -->
<nav class="site-nav">
  <a href="/">Home</a>
  <a href="/notes/">Notes</a>
</nav>
```

Style them from `import-style.html`. wanshi applies none of its own rules to
this markup beyond the page defaults, so anything you inject keeps whatever
appearance your stylesheet gives it.

Note that `import-footer.html` is site chrome, distinct from the per-note footer
that carries backlinks and references — that one is controlled by
`[wanshi].footer-mode` and lives inside the article.

## Themes

`[wanshi].themes` is a legacy multi-theme hook, largely vestigial in wanshi since
the fork ships a single design. Each entry is a path, relative to the project
root, to an HTML fragment whose contents are inlined into a `<div
id="theme-options">` in the table-of-contents sidebar. Fragments are expected to
contain `<theme-option name="...">` elements, each of which becomes a radio
button; choosing one persists the name under the `wanshi-theme` `localStorage`
key and re-runs the dynamic color-inversion pass that tints SVG formulas and
Typst-rendered images to match.

With `theme-lock = true` (the default), the picker is hidden with CSS regardless
of what `themes` contains.

Changes to theme files, and to the six `import-*.html` files above, are watched
by `wanshi serve` and trigger a rewrite of the whole site.

## Generated Artifacts

Depending on command flags and configuration, wanshi writes:

- HTML pages for visible sections.
- `main.css` unless CSS is inlined.
- `main.js` unless JavaScript is inlined.
- A copied assets directory, including `favicon.ico`, which every page links.
- `wanshi.json` when metadata indexes are enabled.
- `wanshi.graph.json` when graph output is enabled.
- `wanshi.search.json` when search is enabled.
- `feed.xml` when RSS is enabled for publish builds.

Serve mode defaults index and graph outputs off. Build mode defaults them on.
