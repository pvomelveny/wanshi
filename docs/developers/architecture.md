# Architecture

wanshi is a Rust command-line application that turns a configured forest of Typst sources into static HTML. The central design is a two-stage compiler:

1. Parse every source into shallow, unresolved sections.
2. Resolve the section graph, then write visible pages and optional artifacts.

This split keeps source parsing, graph semantics, and HTML rendering separate enough to support caching, validation, and incremental serving.

## High-Level Components

wanshi is organized around these responsibilities:

- CLI layer: parses command-line arguments and selects build, check, serve, creation, snippet, and upgrade workflows.
- Environment layer: loads configuration, derives project paths, exposes mode-aware accessors, imports themes and HTML snippets, and manages cache/hash paths. Optional project-root HTML snippets extend or replace the generated document head; two of them are additive (extra metadata, extra styles) and two replace built-in defaults (font imports, math imports). All of them, plus configured theme fragments, participate in serve-mode watching.
- Source scanner: discovers `.typ` source files, records their slug, and handles read-only scans for checks.
- Parser layer: converts Typst source files into unresolved sections containing metadata plus plain or lazy content.
- Compiler state: resolves embeds and links into a graph of compiled sections, detects cyclic embeds, records parent relationships, references, and backlinks.
- Writer: renders compiled sections into complete HTML documents, footers, catalogs, headers, and RSS-safe content.
- Artifact writer: writes optional metadata and graph JSON, RSS feeds, static runtime files, and copied assets.
- Serve session: maintains in-memory state for local preview and performs incremental rebuilds based on watcher dirty sets.
- Upgrade and scaffolding tools: generate new projects, sections, snippets, config files, and the current Typst library file.

## Data Model

The main domain objects are:

- Slug: stable section identifier derived from source paths or subtree declarations.
- Source section: a source file can produce one or more sections.
- Unresolved section: metadata plus content that may contain lazy local links and embeds.
- Compiled section: fully resolved content whose children, references, metadata, options, and footer behavior are known.
- Callback graph: parent and backlink information collected while resolving lazy content.
- Compile state: the full set of compiled sections plus graph state.

## Source Discovery

Source discovery walks the configured source tree and builds a map from section slug to source path.

Discovery behavior:

- Directories whose names begin with `.` or `_` are ignored. This keeps cache directories, the Typst library, and private implementation folders out of the section graph.
- Only `.typ` files are admitted into the workspace.
- The slug is the source-tree-relative path with the extension removed and path separators normalized.
- If two source files would produce the same slug, discovery fails immediately.
- Non-UTF-8 paths discovered during recursive walking are skipped with a warning.
- A missing source tree is not fatal; it produces an empty workspace and a warning.

Files that are not section sources are simply not discovered. Shared Typst code — helper functions, reusable figures — lives in an ignored directory and is pulled into notes with a normal Typst import, so it never needs a discovery rule of its own.

An earlier design compiled every `.typ` file under the source tree into a standalone SVG output asset. That was removed: the generated file could only be referenced by a hand-written absolute URL that ignored `base-url`, it silently swallowed any note that used the extension, and rendering shared Typst content inline through the library's frame helper covers the same need without leaving an intermediate artifact.

## Path and URL Resolution

wanshi resolves paths in two related but distinct forms:

- Source-relative paths identify files inside the configured source tree.
- Site URLs identify generated HTML pages and copied assets.

Local section links, embeds, and named subtree slugs all resolve through one rule: the target is joined onto the directory containing the current section, then normalized with `.`/`..` segments collapsed and any known source extension stripped. A target beginning with `/` replaces the current directory entirely, which makes it absolute from the source tree root. This gives authors both a relative and a tree-absolute form without a separate resolution path for each.

Asset links are recognized by checking whether the normalized target path starts within the configured assets root.

A section named `index` is linked as its containing directory rather than as a file: the root index as the site root, and a directory index as `<dir>/`. Nothing about the output changes — the page is still written to `<dir>/index.html`, which is precisely the file a server returns for a directory request — so this is purely which URL gets generated. Two consequences follow from that. The previously generated `<dir>/index.html` form keeps working, since the file is still there, making the change non-breaking for anything already published or linked externally. And it removes an inconsistency, because the slug shown beside a title already omits a trailing `/index` and so already read as the bare directory.

The trailing slash is load-bearing and the URL is therefore assembled directly rather than through the general path helper, which normalizes and would discard it. Without the slash a relative reference from the page would resolve against the parent directory instead of the page's own.

## CLI Workflows

### Build

Build mode initializes the environment in publish mode, ensures cache compatibility, writes or inlines runtime assets, scans sources, parses changed or cached sources, resolves the graph, writes pages, reconciles the output directory, copies assets, and writes optional publish artifacts.

By default, build mode emits metadata and graph JSON. RSS is emitted only when configured.

### Check

Check mode initializes the environment in validation mode and uses read-only source scanning. It parses sections without relying on normal build output, reports diagnostics, validates dangling local links, and compiles the section graph to catch graph-level failures.

Check mode does not write build artifacts.

### Serve

Serve mode performs an initial build into the serve output directory, starts the configured static server, watches source, asset, config, theme, and import paths, then decides whether each change batch requires an incremental source rebuild, a global rewrite from memory, or a server restart.

Serve mode defaults metadata and graph JSON off to keep preview output lightweight. It can enable them through output flags.

Two flags exist for embedding serve mode in other tooling. One suppresses the external server process so the caller can serve the output itself, and one switches stdout to line-delimited JSON build events for editor integrations. The JSON mode requires the no-server mode, so that ordinary progress output never contaminates the machine-readable stream.

### Scaffolding and Upgrade

Project creation and initialization generate a config file, source directory, assets directory, starter section, ignore file, and the Typst library. Upgrade workflows deserialize the current config, serialize it into the current schema, and sync the bundled Typst library.

## Typst Processing

Typst source processing first asks Typst to render the source into HTML. wanshi then parses structured marker elements emitted by wanshi's Typst library.

The marker parser recognizes metadata, local links, embeds, and subtrees:

- Metadata markers (`wanshi-meta`) either carry a plain value attribute or nested marker/body HTML that is recursively parsed into rich content.
- Embed markers (`wanshi-embed`) carry a target URL, optional title, and boolean-like options. Missing or `auto` option values keep defaults; `false`, `0`, and `none` disable an option; other present values enable it.
- Local markers (`wanshi-local`) produce lazy local-link content with an already resolved target slug or URL.
- Subtree markers (`wanshi-subtree`) produce additional unresolved sections and insert a lazy embed into the current section.

Subtree parsing resolves named subtrees relative to the directory of the current slug, generates internal slugs for anonymous subtrees, and treats `title`/`taxon` attributes as defaults only when the subtree content does not provide those metadata fields itself.

Failures are wrapped with source and slug context because the Typst phase crosses a process boundary and may fail due to syntax errors, missing packages, unavailable fonts, or environmental issues.

## Graph Resolution

The graph compiler starts from `index` when present, then compiles any remaining unlinked sections. During compilation:

- Plain content is copied into the compiled section.
- Embed content fetches the target section, detects cycles, records parent relationships, applies embed options, and can override the embedded title.
- Local links resolve target metadata, produce final HTML links, and may add references and backlinks.
- Metadata values that are rich content are compiled using the same section compiler path so title and taxon HTML remain consistent.

The compiler keeps a visiting stack to detect embed cycles and reports the full cycle chain. Missing embed targets are hard errors. Missing local link targets are surfaced by check diagnostics.

Anonymous internal subtree slugs are normalized so they do not leak into the visible reference/backlink graph.

## Graph Algorithm Details

Graph compilation is depth-first. The compiler maintains:

- A residual set of slugs that have not yet been compiled.
- A compiled map of finished sections.
- A visiting set and compile stack for cycle detection.
- A callback graph for inferred parents and backlinks.

The compiler tries `index` first so sites with a conventional entry point get stable parent inference. It then compiles any residual sections so orphan pages still receive output.

When a lazy embed is encountered:

1. Resolve the target slug relative to the current slug.
2. Fetch and compile the target section recursively if needed.
3. Fail if the target does not exist.
4. Fail if the target is already on the active compile stack, reporting the whole cycle.
5. Record the target's inferred parent as the current slug.
6. Clone the compiled child section into the parent content and apply embed options.
7. Apply an embed title override if the embed supplied one.
8. If the embedded details are open, propagate the embedded section's references into the parent reference set.

When a lazy local link is encountered:

1. Resolve the target slug relative to the current slug.
2. Read the target metadata if available.
3. Use explicit link text when present, otherwise use the target title.
4. Build the final generated URL with the current URL policy.
5. Add the target to the current section's references if the target is considered reference-like.
6. Add the current section to the target's backlink list if both source and target metadata allow it.

A target counts as reference-like when its `asref` metadata is true, or when
`asref` is unset and the global build default is true, or when its `data-taxon`
begins with a reference marker (`reference`, case-insensitively, or its CJK
equivalent). The taxon rule is what lets a bibliography work without any
configuration.

Self-links are excluded from both references and backlinks: a section that links
to itself neither cites nor backlinks itself.

After all content is resolved, rich metadata values are compiled through the same unresolved-section machinery. This keeps formatted titles and taxons consistent with normal content and avoids a separate rendering path for metadata.

Parent behavior is intentionally layered, resolved in strict precedence:

1. `parent` metadata, when the section declares one.
2. The embedding section, when something embeds it.
3. The nearest enclosing directory index, found by walking the slug's path upward.
4. The root index.

Only the first two are recorded in the graph while compiling; the rest are a resolution rule applied when a section is asked for its parent. That split matters, because a section that nothing embeds has no recorded parent at all, and inventing one during compilation would make "unattached" indistinguishable from "deliberately attached to the root".

For the same reason a recorded parent is optional rather than defaulted. Using the root index as a stand-in for "unknown" is ambiguous — the root is itself a legitimate parent — and that ambiguity previously let a genuine parent be silently overwritten during callback merging.

A directory index never adopts itself, so it resolves upward to the next enclosing index instead of becoming its own parent. The root index does not point to itself in generated header navigation. If a parent cannot be found during writing, header navigation is skipped with a warning rather than failing the build.

Anything that walks parent relationships — notably incremental rebuild expansion — must use the resolved parent rather than the recorded one, and must consider every compiled section rather than only those with recorded callbacks. Otherwise a section whose parent comes from a directory index would never be marked dirty when that index changes, and its navigation would go stale.

## Listings

Listings ask questions about the finished graph — which sections are children of this one, which are most recent, which nothing links to. None of that is knowable while sections are still being compiled, so listings cannot be resolved inline the way embeds and local links are.

They are handled in three stages. The marker parser lowers a listing into lazy content carrying a specification rather than a result. Graph compilation turns that into a placeholder in the compiled section, contributing nothing to the graph: a listing creates no parent, no reference, and no backlink, because it is a view of the graph rather than an edge in it. A final pass over the completed state replaces every placeholder with rendered HTML.

The specification records the slug of the section that wrote it. This matters because a compiled section can be cloned into any number of embedding pages, and an embedded copy must still list *its own* children rather than its host's. Binding the owner at parse time makes resolution independent of where a copy ends up.

Identical specifications render once and are shared, and a listing never includes the section it appears on.

Because a listing's output depends on sections other than its own source, listing pages cannot participate in normal source-level incremental rebuilds. The resolution pass reports which sections own listings, and incremental rebuilds add those to the affected set whenever anything changed. Output hashing keeps this cheap: a listing page that resolves to the same HTML is recompiled but not rewritten.

## Search Index

Search is client-side, so the whole question is what to ship. The obvious option — every section's text alongside its metadata — is rejected on two grounds: it publishes a second, readable copy of every note, and it grows linearly with the corpus.

The index is inverted instead. Each token maps to the sections containing it, and the text itself is never emitted. Word order, punctuation and repetition are discarded, a token used across many sections is stored once, and a note cannot be reconstructed from the result. Measured against prose corpora it runs roughly a third the size of shipping the text, and the ratio holds as the corpus grows because vocabulary saturates while text does not.

Titles, taxons and slugs *are* shipped as text, because results cannot be rendered without them. They are also what the client matches against directly, which is what makes ranking possible: a title hit outranks a body hit, and the inverted index only has to answer "which sections contain this word".

Two details matter for index quality. Sections embedded into a page are excluded from that page's text, since embedded content belongs to the note that wrote it and indexing it twice would make a hub match everything it displays. And elements whose contents are markup rather than prose are removed wholesale before tokenizing — Typst renders diagrams and any `auto-frame` content as SVG, and the general-purpose tag stripper cannot match attribute names containing a colon, so without this the index fills with coordinates and namespace URLs. Equations are not among them: they are MathML, whose text nodes extract cleanly.

The index is a normal optional artifact: written when enabled, removed when disabled, and resolved against `base-url` at build time so the client never has to guess the deploy prefix.

## Output Model

Metadata keys are classified into three groups: *fancy* keys that may hold rich content and drive rendering (title, taxon), *plain* keys that must be flat text and drive graph or output behavior (parent, footer mode, the boolean switches, and internally derived fields), and everything else, which is custom. Custom keys survive into the metadata index and are rendered in header metadata rows.

One custom key is privileged at render time: `date` is pulled out of the custom set and given its own column, both in section headers and in catalog/footer entry lists, following the design system's fixed-date/flexible-content row pattern. It remains an ordinary custom key everywhere else — it is not required, not validated, and still available as a footer sort key.

For each visible compiled section, the writer creates:

- Header navigation based on the recorded parent or the default index parent.
- Article content with embedded child sections.
- Catalog/table-of-contents content where applicable.
- Footer references and backlinks, rendered either as compact links or embedded content.
- A complete HTML document with configured imports, themes, runtime assets, and page title.

The artifact writer emits optional JSON snapshots for metadata and graph consumers. RSS output is generated in publish mode when enabled and requires an absolute HTTP(S) base URL.

## Rendering and Artifact Details

HTML writing is hash guarded. Before writing a generated page, wanshi compares the new payload with the stored hash for the relative output path. Unchanged pages are skipped, which is important for fast rebuilds and for deployment systems that care about file modification times.

Footer rendering is driven by the section's effective footer mode:

- `link` mode renders compact summaries of referenced or backlink sections.
- `embed` mode renders the referenced or backlink section content recursively into the footer context.

Footer entries are sorted deterministically. Built-in sort keys include slug, date, taxon, and title; arbitrary metadata keys can also be used. Date sorting uses a parser that gives chronological ordering when dates are recognized and stable fallback ordering when they are not.

The graph JSON artifact is a normalized snapshot of compiled graph relationships. Each visible section records its parent, whether the parent was explicitly specified, sorted references, and sorted backlinks. The metadata JSON artifact records visible section metadata only; internal anonymous subtree sections are excluded.

RSS generation uses compiled sections after graph resolution. Collection pages and the index page are excluded from feed items.

A subtree is excluded as well whenever the section that declared it is itself in the feed, because its content is rendered inside that item and listing both hands a subscriber the same text twice. The test is on the recorded source rather than the parent, since a file's whole subtree chain renders into its root section's item. That single rule covers both ways of writing: a file that is one post carries its subtrees, and a file marked as a collection is not a post, so the notes inside it become the items instead — which is what declaring it a collection already means. Subtrees carry no date of their own, so a subtree that does become an item inherits its source's; without that it would have no publication date and sort below every dated note. Item order is reverse date order with slug fallback. Full item content is included as encoded HTML content, with the sequence that would close a CDATA section split so it cannot. Invalid or relative RSS base URLs are rejected before the feed is written, because a feed is read away from the site and its links must stand alone. Pages carry a feed autodiscovery link, gated on the same condition that writes the feed so preview builds never advertise one that does not exist.

Item descriptions are plain text recovered from the rendered HTML, which is more delicate than it sounds and was wrong in three ways at once. The article header is dropped, or a summary opens with the taxon, title, slug and date that the reader already sees as the item's own title and pubDate. Entities are unescaped, because the text is escaped again on its way into XML and skipping that shows readers a literal `&amp;`. And the extraction cannot use the display-side tag stripper: that is a regex whose tag-name pattern matched letters only, so every `<h1>` survived into the summary as visible markup.

## Caching and Incrementality

wanshi uses source-entry caches for parsed sections and output hashes to avoid unnecessary writes. A cache version check protects against incompatible cache shape changes.

Incremental builds are driven by dirty paths:

- Dirty source paths map to dirty source slugs.
- The compiler expands affected slugs to include pages impacted by graph dependencies.
- Stale cache entries and hash records are cleaned when a source disappears.
- Generated pages whose sections no longer exist are removed from the output directory.
- Serve mode can rewrite all pages from memory when global non-source inputs change.

## Output Reconciliation

Cleaning caches is not enough on its own: a page whose source has been deleted would otherwise remain in the output directory and continue to be deployed. After every write pass, the compiler reconciles the output directory against the set of currently visible sections.

Reconciliation is manifest-driven rather than derived from scanning the output directory. Each build records the relative paths it generated, and the next build deletes only those recorded paths that are no longer produced. The consequences of that choice are deliberate:

- Files the tool never wrote are never candidates for deletion, so hand-maintained additions to the publish directory — a domain file, a custom error page — and the copied assets tree are structurally safe.
- A missing or unparsable manifest disables reconciliation for one build instead of guessing, then rebuilds itself.
- Manifest entries that are not plain relative paths are rejected, so a corrupted manifest cannot reach outside the output directory.
- The manifest is keyed by build mode, because publish and serve builds share a cache directory but write to different output directories.

Removing a generated page also removes its output hash record. The writer skips any page whose content hash is unchanged, so a page that was deleted and later recreated with identical content would otherwise never be written back.

This covers three cases that the cache-level cleanup alone does not: a deleted source file, a source that still exists but no longer declares a subtree it used to, and a source whose slug changed. Directories left empty by reconciliation are removed as well.

This model favors correctness for graph relationships while still reducing the amount of parsing and writing during local development.

## Cache and Incremental Algorithms

For each source file, wanshi decides whether to parse from source or load a cached unresolved-section entry.

The source modification decision follows this order:

1. If `--no-cache` is active, treat the source as modified.
2. If a dirty set is supplied, only paths in that set are treated as modified, but dirty source paths still update their hash baseline for future cold builds.
3. Without a dirty set, compare the current file hash with the stored hash.

Dirty path expansion is conservative: any dirty path that is not a known `.typ` section source (a shared Typst library file, a `.typ` asset, or anything else) marks every known source dirty, since wanshi does not maintain a dependency graph for arbitrary include relationships. A dirty path that is itself a known section source stays scoped to that source.

After graph compilation, dirty source slugs are expanded to affected output slugs:

1. Include dirty source slugs.
2. Include embedded descendants whose parent chain starts at a dirty slug. This covers generated subtree sections and embedded ownership.
3. If a changed page contributes backlinks, include the target pages whose backlink lists change.
4. Walk parent and backlink relationships from the affected set until no new affected slugs are found.

If stale slugs are detected because source files disappeared, wanshi writes all visible pages from the current graph to ensure navigation and footers converge to the new state.

Serve mode keeps a compile session in memory. Source changes update the session incrementally when possible. Global changes, such as theme or import changes, can reuse the in-memory graph and rewrite all pages. Config changes trigger a full build and server restart because configuration can affect paths, URL policy, runtime imports, and the external server command.

## Safety Model

Typst execution is delegated to the user's local Typst installation, so Typst availability and package access are environmental requirements rather than embedded application behavior.

## Check Diagnostics

Check mode is designed to validate behavior that users would otherwise discover after a failed or broken build.

It reports:

- No sections found under the source tree as a hint.
- Missing `index` as a warning.
- Source parse failures as errors.
- Duplicate generated slugs as errors.
- Dangling local links as warnings.
- Graph compilation failures, including cyclic embeds and missing embed targets, as errors.

Strict mode upgrades warnings into command failure. Hints remain informational.

## Future Considerations

Known limits of the current design, recorded so they read as decisions rather than oversights. Each is deferred because the cost of addressing it currently exceeds the harm, not because it is unnoticed.

### Mathematics is not indexed, though it is no longer invisible

This was once a much larger problem: equations were rendered as vector SVG containing only path, symbol, and use elements, with no title, no accessible label, and no text node of any kind. Plain-text extraction produced summaries with holes where the formulas had been, so a sentence could end on stranded punctuation — "acting on ." — and a note was unfindable by any symbol it contained.

Emitting MathML instead of SVG resolved most of it. Equations are now text in the document, so extraction keeps them: "Let ( 𝑋 , 𝑑 ) be a metric space." RSS summaries read as sentences again, and screen readers reach the mathematics.

What remains is narrower. The symbols are Unicode mathematical alphanumerics — `𝑋`, not `X` — and the tokenizer does not treat them as searchable words, so search still cannot find a note by a symbol in it. That is close to the right behaviour: indexing single italic variables would mostly add noise, since almost every note about algebra contains an `𝑥`. Searching for mathematical *content* would want a different representation entirely — the equation's source, or a normalised form — and that is a retrieval design question rather than an extraction bug.

### Search index growth is bounded only by vocabulary

The inverted index is far smaller than the prose it derives from, and the ratio improves with corpus size as vocabulary saturates. It is nonetheless unbounded: the postings grow with the number of sections, and the client fetches the whole index on first use. There is no incremental or sharded loading, and no server to query against. A forest large enough for that to matter would want a different retrieval design, not a larger version of this one; the escape hatch until then is disabling body indexing, which reduces the index to titles.
