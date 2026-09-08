# Code Review — wanshi

Reviewed at trunk `726a733`, 2026-09-04. Baseline: `cargo test` 312 passed,
`cargo clippy --all-targets` 0 warnings.

Findings are grouped by severity. File references are to `src/`.

> **Status update (2026-09-08):** everything actionable in this review is
> fixed on the `fix/review-bugs` branch, one commit per finding: the five
> bugs (B1–B5), the improvements (C1–C9), and the notes (N1–N5; N3 was
> folded into the B3 commit). The one deliberate exception is the
> `slug_owner` linear scan under N5, deferred until it shows in a profile,
> as noted there.

---

## Bugs

### B1. Duplicate section numbers after an unnumbered wrapper (confirmed)

`compiler/counter.rs` — `Counter::passthrough()` returns a **clone**, so
numbers taken inside an unnumbered section never advance the parent's
counter. The doc comment promises transparency ("what is inside it goes on
counting where the section itself would have"), but the increment is lost on
the way back out.

Confirmed with a probe test: a numbered page containing

1. `#embed(a)` (taxon definition)
2. `#embed(wrapper, numbering: false)` which itself contains
   `#embed(inner, numbering: true)` (taxon remark)
3. `#embed(c)` (taxon definition)

renders labels `["Definition 1.", "Remark 2.", "Definition 2."]` — two
different sections both numbered **2** on the same page.

The existing test `test_passthrough_leaves_an_unnumbered_section_transparent`
only checks that the clone *starts* in the right place, not that the parent
sequence reflects what happened inside.

Fix sketch: an unnumbered section's children must share the parent's counter,
not a copy. E.g. make `Label.children` an `Option<Counter>` (None = "use the
parent's `&mut Counter` directly"), or give `Counter` interior sharing
(`Rc<RefCell<..>>` is overkill; restructuring `section_to_html` to reuse
`counter` when `label` passes through is cleaner).

### B2. A failed page write does not fail the build

`compiler/writer.rs:110-117` — `Writer::write` handles `std::fs::write`
failure by printing to stderr and returning `Ok(())`:

```rust
Err(err) => color_print::ceprintln!("<r>{:?}</>", err),
```

A build hitting a permissions problem, disk-full, or a bad output path prints
a red line, then exits 0. Verified in `environment/hashing.rs`: the hash
store is updated *before* the write is attempted (`verify_update_hash`
persists the new hash, then `fs::write` runs). Consequences:

- CI/publish scripts see success with missing or stale pages.
- If an **old version** of the page exists on disk, the next build sees a
  matching hash *and* an existing file, so the stale page is never rewritten
  until the source changes again. (The `!filepath.exists()` guard only
  rescues the fully-missing case.)

Suggest propagating the error (`?`) — the caller `write_needed_slugs` already
returns `eyre::Result` — or updating the hash only after a successful write.

### B3. Note titles are interpolated unescaped into HTML attributes

The `html!` macro (`html_macro.rs:18`) writes attribute values with no
escaping. `html_link` escapes carefully with `htmlize::escape_attribute` —
but the macro call sites don't:

- `html_flake/core.rs:150-162` (`catalog_item`): `title={page_title} [{slug}]`
  and `onclick={...}`.
- `html_flake/core.rs:201-207` (`html_query_item`): same `title` pattern.
- `html_flake/header.rs` (`html_header_nav` via caller): `title={page_title}`.

A note titled `A "quoted" title` produces `title="A "quoted" title [slug]"`
— the attribute terminates early and the remainder leaks into the tag. Since
authors write their own sites this is a rendering-correctness bug more than a
security one, but a single quote character in any title malforms every TOC
row and query listing that names the note. Either escape at these call
sites, or make the `html!` macro escape attribute values by default (the
safer fix; the few places that intentionally pass pre-built HTML are element
*bodies*, which the macro distinguishes already).

Related: `html_doc` (`html_flake/document.rs:33`) drops `page_title` into
`<title>` unescaped. Entities survive `remove_all_tags`, so `&amp;` stays
escaped, but a literal `<` in a page-title would break the tag.

### B4. `wanshi init` / `wanshi new config` silently overwrite `Wanshi.toml`

`cli/new.rs:144-151` — `new_config_inner` writes the default config
unconditionally. Two ways this loses data:

- `wanshi new config` in an existing site replaces a customized `Wanshi.toml`
  with defaults, no prompt, no backup.
- `wanshi init` (default path `./`) calls `add_project_files`, which writes
  the config **first** and then fails on `create_dir(trees)` if the site
  already exists — so the command errors out *and* the config has already
  been reset. Running `init` twice by accident is enough.

Compare `new_katex` (checks `exists()` and declines) and `new_section_inner`
(errors on collision). The config path should do the same; `wanshi upgrade
config` already exists as the sanctioned "regenerate my config" path and
preserves the user's values.

### B5. Watcher drain loops drop one queued event

`cli/serve/watch/runtime.rs:105-112` —

```rust
while let Ok(Ok(event)) = rx.try_recv() { … }
while let Ok(Err(error)) = rx.try_recv() { … }
```

The first loop exits when it receives `Ok(Err(_))` — but that error has
already been *consumed* and is silently discarded, not printed. Symmetrically
the second loop discards an `Ok(Ok(event))` it happens to pull, dropping a
real file-change notification. Rare (needs an error interleaved with events
in one batch), but the fix is a single loop matching on the payload:

```rust
while let Ok(msg) = rx.try_recv() {
    match msg { Ok(event) => …, Err(error) => … }
}
```

---

## Improvements / clarity

### C1. `compile_all_with_missing_index_warning` control flow

`compiler/state.rs:63-72` — the `if emit && compile(..)?.is_none() { warn }
else if !emit { compile(..) }` shape forces the reader to check that both
branches really compile the index. Simpler and equivalent:

```rust
let missing = state.compile(shallows, index)?.is_none();
if emit_missing_index_warning && missing { … warn … }
```

### C2. Footer builder duplicates its three blocks

`compiler/writer.rs:247-353` — references, backlinks and embedded-by are
three near-identical ~25-line blocks (collect slugs → sort → loop → warn on
missing → wrap in `html_footer_section`). A helper taking
`(slugs: &[Slug], mode: Option<FooterMode>, css_class, label)` would collapse
~70 lines to ~15 and make the one real difference (embedded-by forces
`FooterMode::Link`) stand out instead of hiding in the third copy.

### C3. String literals where key constants exist

- `entry.rs:305` — `self.0.get("title")` inside `to_header`; `KEY_TITLE`
  exists and is used two lines away in other files.
- `compiler/writer.rs:379` — `footer_sort_value` matches `"date"` and reads
  `get_str("date")`; `KEY_DATE` exists precisely so date stays identifiable.
- `entry.rs:335-344` — `to_slug_text` hardcodes `"/index"`; `INDEX_SLUG` was
  introduced to centralize this (`AGENTS.md` calls it out). Also, the
  `match … ends_with` would read better as
  `slug.strip_suffix("/index").unwrap_or(slug)`.

### C4. `items.is_empty().not()`

`compiler/writer.rs:190` — imports `std::ops::Not` for one call site;
`!items.is_empty()` is the idiom everywhere else in the file.

### C5. Redundant digit check in `shift_heading_levels`

`compiler/writer.rs:66-67` — `d.is_ascii_digit() && (b'1'..=b'6').contains(d)`:
the range check already implies the digit check.

### C6. Query `sort:` silently ignores custom metadata keys

`compiler/query.rs:308-316` — `sort_value` recognizes `slug`/`title`/`taxon`/
`date` and returns `""` for anything else, so `#query(sort: "author")`
silently sorts by the slug tiebreak. The footer sorter
(`writer.rs::footer_sort_value`) *does* support arbitrary metadata keys via
`get_str(key)`. Either carry the metadata into `QueryHit` so listings match
the footer's behavior, or warn on an unrecognized sort key — silence is the
worst of the three. (Also: query `title` sorts by `page_title` but displays
`title`; deliberate or not, worth a comment.)

### C7. Missing-`typst` error doesn't name the missing program

`typst_cli.rs:44-54` — if the `typst` binary is not on PATH, `.output()?`
propagates a bare `No such file or directory (os error 2)` wrapped in
"failed to compile typst file …", which reads as if the *source file* were
missing. `cli/serve/process.rs::explain_spawn_failure` already solves exactly
this for miniserve with a "was not found on PATH … install it" message —
typst, being a hard requirement for every command, deserves at least the
same. Match on `ErrorKind::NotFound` and say "the `typst` program was not
found on PATH".

### C8. Compiling metadata by hijacking the section's slot in `compiled`

`compiler/state.rs:280-291` — a lazy custom-metadata value is compiled by
wrapping it in a synthetic `UnresolvedSection` carrying the *real* section's
slug, recursively calling `compile_unresolved`, and reading the result back
out of `self.compiled[slug]` — which is then overwritten by the real section
at the end. It works, but it means `compiled` transiently holds a bogus entry
for `slug` mid-compile, and `fetch_section`'s early-return
(`compiled.contains_key`) could hand that bogus entry to anything that
resolves `slug` during the metadata loop. A local
`compile_content(...) -> SectionContents` helper that doesn't touch the map
would remove the aliasing hazard and the two `ok_or_else` contortions.

### C9. Small duplications

- Leap-year/days-in-month logic exists twice: `footer_sort.rs:107-121`
  (`validate_ymd`) and `compiler/rss.rs:292-314` (`is_valid_calendar_date` /
  `days_in_month` / `is_leap_year`). One shared calendar helper would do.
- `next_atomic_write_stamp` + its `AtomicU64` exist in both
  `atomic_text.rs:82-89` and `cli/build.rs:261-268`, character-for-character.
- `entry.rs::to_slug_text` re-implements the `"/index"`-stripping idea that
  `slug::directory_of_index` owns.

---

## Notes / lower priority

### N1. `parse_bool` / `parse_tristate` treat typos as `true`

`compiler/typst.rs:31-45` — any unrecognized attribute value
(`numbering="flase"`, `open="off"`) silently parses as `true` via the `_`
arm. A warning naming the section would turn a silent mis-render into a
build-time nudge. (The test suite even relies on the fallthrough: `"yes"` is
asserted to be `true`.)

### N2. `remove_all_tags` still can't match namespaced attributes

`compiler/section.rs:178` — attribute regex `[a-zA-Z-]+` can't match
`xmlns:xlink`, the trap documented in AGENTS.md. `search.rs` works around it
by stripping `<svg>` wholesale, but the display-side function remains
vulnerable to any future colon-attributed markup. Adding `:` (and `_`,
digits) to the attribute-name class is a one-character hardening.

### N3. `html_macro.rs` doc examples are wrong

Both doc-comment examples (`html_macro.rs:5-10`, `53-62`) would not compile
or assert the wrong output: `html_write!(&mut s; br)` where `s: &str`;
`id=(id.to_string())` isn't a `tt` the attr arm accepts;
`r#"<p class="c",id="id">1</p><br />"abc""# ` is not what the macro emits.
They aren't doctested (the macros are `pub(crate)`), so they've drifted.
Fix or delete; a wrong example is worse than none.

### N4. `wanshi snip` with no flags silently succeeds doing nothing

`cli/snip.rs` — only `--katex` does anything; a bare `wanshi snip` exits 0
with no output. Since KaTeX generation is the only remaining function, either
make `--katex` the default action or print "nothing to do — did you mean
`--katex`?".

### N5. Odds and ends

- `compiler/subtree_slug.rs` starts with a UTF-8 BOM (U+FEFF before the
  copyright line). Rust tolerates it; tooling diffs don't always.
- `config/mod.rs:180` — `mod test` lacks `#[cfg(test)]`, unlike every other
  test module in the crate. Harmless today (only `#[test]` fns inside) but
  inconsistent.
- `compiler/serve_session.rs:157-161` — `slug_owner` linearly scans all
  sources per parsed section (O(sources × sections) per rebuild batch). Fine
  at personal-site scale; a reverse map would be trivial if it ever shows up
  in a profile.
- `compiler/state.rs:63-72` — see C1.

---

## What's notably good

Worth saying explicitly, since the bar here is high:

- **Test discipline.** 312 tests, and the interesting ones pin *decisions*
  (`test_a_subsection_and_a_statement_may_share_a_number`,
  `test_two_hosts_on_the_same_page_are_not_an_ambiguity`) with comments
  explaining why the behavior is intended. Regression tests name the bug they
  fix. This is rare and valuable.
- **Doc comments explain *why*, not *what*** — `heading_sections.rs`,
  `counter.rs`, and `refs.rs` are exemplary.
- **Failure-mode thinking**: the output manifest deleting only what wanshi
  recorded writing; `is_safe_relative_output_path` guarding a corrupt
  manifest; cache-version stamping; atomic writes with temp-file cleanup;
  `explain_spawn_failure` turning ENOENT into actionable advice.
- Clippy-clean at `--all-targets`, zero warnings, consistent error style
  (`eyre` + `wrap_err_with` with named paths).

## Suggested priority

1. **B1** (duplicate numbers) and **B2** (failed writes exit 0) — real
   correctness bugs; B2 is the one that can hurt a publish.
2. **B4** (config clobbering) — one-line guard, prevents data loss.
3. **B3** (attribute escaping) — decide the escaping policy for `html!`.
4. **B5**, **C6**, **C7** — small, high leverage on UX.
5. The rest as cleanup when touching the files anyway.

