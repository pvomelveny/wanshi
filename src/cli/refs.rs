// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.

//! Reference stubs: generating them from a bibliography, and collecting them
//! back out for a paper.
//!
//! The forest cites works the same way it cites notes — `#local("/refs/key")` —
//! so a citation produces the ordinary two graph edges, and the questions the
//! graph can already answer become bibliographic ones. `sync` fills in the note
//! a citation promised; `export` reverses the arrow, turning "what have I
//! written about this" into the bibliography a paper on it would need.

use std::collections::BTreeSet;

use camino::Utf8PathBuf;
use clap::Parser;
use eyre::{bail, eyre, WrapErr};

use crate::{
    compiler::{self, links},
    config,
    entry::MetaData,
    environment::{self, BuildMode},
    refs::{self, Bibliography},
    slug::{Ext, Slug},
};

#[derive(Parser)]
pub struct RefsCommandCli {
    #[command(subcommand)]
    pub command: RefsCommand,
}

#[derive(clap::Subcommand)]
pub enum RefsCommand {
    /// Generate a note for every cited work that does not have one yet.
    #[command(visible_alias = "s")]
    Sync(RefsSyncCommand),

    /// Print the bibliography a set of notes cites.
    #[command(visible_alias = "e")]
    Export(RefsExportCommand),
}

#[derive(clap::Args)]
pub struct RefsSyncCommand {
    /// Path to the configuration file (e.g., "Wanshi.toml").
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG_PATH.into())]
    config: String,

    /// Report what would be written without writing it.
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

#[derive(clap::Args)]
pub struct RefsExportCommand {
    /// Path to the configuration file (e.g., "Wanshi.toml").
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG_PATH.into())]
    config: String,

    /// Only consider sections whose slug starts with this prefix.
    ///
    /// Omit to export everything the whole forest cites.
    #[arg(long)]
    from: Option<String>,

    /// Output format. Defaults to the bibliography's own.
    ///
    /// Matching the source is the lossless choice both ways: BibTeX entries are
    /// copied verbatim, and Hayagriva is this crate's native model. Converting
    /// Hayagriva to BibTeX is not offered because hayagriva parses BibTeX but
    /// cannot emit it.
    #[arg(long, value_enum)]
    format: Option<ExportFormat>,
}

#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ExportFormat {
    /// BibTeX, copied verbatim from the source.
    Bib,
    /// Hayagriva YAML.
    Yaml,
}

/// Every section in the forest, with the links that resolve to nothing.
struct Forest {
    shallows: std::collections::HashMap<Slug, compiler::section::UnresolvedSection>,
}

impl Forest {
    /// Parse the whole forest, refusing to continue if any of it is broken.
    ///
    /// Stubs are written into the tree, so a half-understood forest is the
    /// wrong thing to write into: a file that failed to parse may hold the very
    /// citation being resolved.
    fn parse() -> eyre::Result<Self> {
        let trees_dir = environment::trees_dir();
        let workspace = compiler::all_trees_source(trees_dir.as_path())
            .wrap_err_with(|| eyre!("failed to scan trees dir `{trees_dir}`"))?;

        let (shallows, failures) = links::parse_all_sections(&workspace);
        if !failures.is_empty() {
            for failure in &failures {
                color_print::ceprintln!("<r>Error:</> {}", failure);
            }
            bail!(
                "refusing to continue: {} section(s) failed to parse",
                failures.len()
            );
        }
        Ok(Self { shallows })
    }
}

/// Why a slug is a candidate for generation.
///
/// The distinction decides what counts as a problem. A work that was cited but
/// is not in the bibliography is a real one — the citation will stay dangling.
/// A note that merely lives in the references directory and is not in the
/// bibliography is not: it is hand-written, which is allowed.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Origin {
    /// A dangling citation: promised by a link, not yet imported.
    Cited,
    /// An existing note under the references directory.
    Existing,
}

pub fn sync(command: &RefsSyncCommand) -> eyre::Result<()> {
    environment::init_environment(command.config.clone().into(), BuildMode::Check)?;

    let forest = Forest::parse()?;
    let refs_dir = environment::refs_dir();

    // Two sources of work. A dangling link under the references directory is a
    // work cited but not yet imported — the note the citation promised. An
    // existing generated note is one whose details may have changed upstream;
    // refreshing those is what keeps the forest from drifting away from the
    // reference manager.
    let mut wanted: std::collections::BTreeMap<Slug, Origin> =
        links::dangling_local_links(&forest.shallows)
            .into_iter()
            .filter(|dangling| dangling.target.as_str().starts_with(&refs_dir))
            .map(|dangling| (dangling.target, Origin::Cited))
            .collect();
    for slug in forest.shallows.keys() {
        if slug.as_str().starts_with(&refs_dir) {
            wanted.entry(*slug).or_insert(Origin::Existing);
        }
    }

    if wanted.is_empty() {
        println!("Nothing to sync: no works are cited.");
        return Ok(());
    }

    let bibliography_path = environment::refs_bibliography();
    if !bibliography_path.exists() {
        bail!(
            "no bibliography at `{}`. Set `[refs].bibliography` in the configuration, \
             or export one there from your reference manager.",
            bibliography_path
        );
    }
    let bibliography = Bibliography::load(&bibliography_path)?;

    // A citation key indexes the bibliography, but the slug is sanitised, so
    // recover the key by matching every candidate's sanitised form rather than
    // assuming the two are identical.
    let by_slug_segment: std::collections::HashMap<String, &str> = bibliography
        .keys()
        .map(|key| (refs::slug_segment(key), key))
        .collect();

    let parent_slug = format!("{refs_dir}index");
    let mut created = Vec::new();
    let mut refreshed = Vec::new();
    let mut unchanged = 0usize;
    let mut hand_written = 0usize;
    let mut uncitable = Vec::new();

    for (target, origin) in &wanted {
        let segment = target
            .as_str()
            .strip_prefix(&refs_dir)
            .unwrap_or(target.as_str());

        // The hub is an ordinary note that happens to live here.
        if segment == crate::compiler::INDEX_SLUG {
            continue;
        }

        let path = environment::trees_dir().join(format!("{target}.{}", Ext::Typ));
        let existing = if path.exists() {
            Some(
                std::fs::read_to_string(&path)
                    .wrap_err_with(|| format!("failed to read `{path}`"))?,
            )
        } else {
            None
        };

        // Checked before the bibliography lookup, so a note written by hand is
        // never reported as missing from a file it was never meant to be in.
        if existing
            .as_deref()
            .is_some_and(|existing| !existing.contains(refs::STUB_MARKER))
        {
            hand_written += 1;
            continue;
        }

        let Some(&key) = by_slug_segment.get(segment) else {
            if *origin == Origin::Cited {
                uncitable.push(target.to_string());
            }
            continue;
        };
        let entry = bibliography
            .get(key)
            .ok_or_else(|| eyre!("bibliography lost key `{key}` between listing and lookup"))?;

        let source = refs::stub_source(entry, &parent_slug);

        // Writing identical bytes would touch the mtime for nothing, and a
        // running `wanshi serve` would rebuild the site over it.
        if existing.as_deref() == Some(source.as_str()) {
            unchanged += 1;
            continue;
        }

        if existing.is_some() {
            refreshed.push(path.clone());
        } else {
            created.push(path.clone());
        }

        if !command.dry_run {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .wrap_err_with(|| format!("failed to create `{parent}`"))?;
            }
            std::fs::write(&path, &source).wrap_err_with(|| format!("failed to write `{path}`"))?;
        }
    }

    let verb = if command.dry_run {
        "would write"
    } else {
        "wrote"
    };
    for path in &created {
        color_print::cprintln!("<g>[refs]</> {} {}", verb, path);
    }
    for path in &refreshed {
        color_print::cprintln!("<c>[refs]</> {} {} (updated)", verb, path);
    }
    for slug in &uncitable {
        color_print::ceprintln!(
            "<y>Warning:</> `{}` is cited but has no entry in `{}`.",
            slug,
            bibliography_path
        );
    }

    if created.is_empty() && refreshed.is_empty() && uncitable.is_empty() {
        println!("Refs sync: up to date.");
    } else {
        println!(
            "Refs sync: {} new, {} updated, {} unchanged, {} written by hand, {} cited but not in the bibliography.",
            created.len(),
            refreshed.len(),
            unchanged,
            hand_written,
            uncitable.len()
        );
    }
    Ok(())
}

pub fn export(command: &RefsExportCommand) -> eyre::Result<()> {
    environment::init_environment(command.config.clone().into(), BuildMode::Check)?;

    let forest = Forest::parse()?;
    let refs_dir = environment::refs_dir();
    let prefix = command.from.as_deref().unwrap_or("");

    // Read citations straight from the parsed sections rather than from
    // `wanshi.graph.json`: the artifact is only as current as the last build,
    // and an export that quietly omits this morning's citation is worse than
    // one that takes a moment longer.
    let mut cited: BTreeSet<String> = BTreeSet::new();
    for (&from, section) in &forest.shallows {
        if !from.as_str().starts_with(prefix) {
            continue;
        }
        let compiler::section::HTMLContent::Lazy(contents) = &section.content else {
            continue;
        };
        for content in contents {
            let compiler::section::LazyContent::Local(local) = content else {
                continue;
            };
            let target = links::resolve_local_target(from, &local.url);
            if !target.as_str().starts_with(&refs_dir) {
                continue;
            }
            // The citation key lives on the work's own note, which is the only
            // place the sanitised slug can be mapped back.
            if let Some(citekey) = forest
                .shallows
                .get(&target)
                .and_then(|section| section.metadata.get_str("citekey"))
            {
                cited.insert(citekey.clone());
            }
        }
    }

    if cited.is_empty() {
        let scope = if prefix.is_empty() {
            "the forest".to_string()
        } else {
            format!("`{prefix}`")
        };
        color_print::ceprintln!("<y>Note:</> {} cites no works.", scope);
        return Ok(());
    }

    let bibliography_path = environment::refs_bibliography();
    let bibliography = Bibliography::load(&bibliography_path)?;

    let source_format = if bibliography.is_biblatex() {
        ExportFormat::Bib
    } else {
        ExportFormat::Yaml
    };
    let format = command.format.unwrap_or(source_format);

    if format == ExportFormat::Bib && source_format == ExportFormat::Yaml {
        bail!(
            "`{}` is a Hayagriva bibliography and cannot be exported as BibTeX: \
             hayagriva reads BibTeX but does not write it. Export as `--format yaml`, \
             or point `[refs].bibliography` at a `.bib` file.",
            bibliography_path
        );
    }

    let mut missing = Vec::new();
    match format {
        ExportFormat::Bib => {
            for key in &cited {
                match bibliography.verbatim(key) {
                    Some(entry) => println!("{entry}\n"),
                    None => missing.push(key.clone()),
                }
            }
        }
        ExportFormat::Yaml => {
            let mut selected = hayagriva::Library::new();
            for key in &cited {
                match bibliography.get(key) {
                    Some(entry) => selected.push(entry),
                    None => missing.push(key.clone()),
                }
            }
            let yaml = hayagriva::io::to_yaml_str(&selected)
                .wrap_err("failed to serialise the selected works as Hayagriva YAML")?;
            print!("{yaml}");
        }
    }

    for key in &missing {
        color_print::ceprintln!(
            "<y>Warning:</> `{}` is cited but not in `{}`.",
            key,
            bibliography_path
        );
    }
    Ok(())
}

/// Where a stub for `slug` would be written.
#[allow(dead_code)]
fn stub_path(slug: Slug) -> Utf8PathBuf {
    environment::trees_dir().join(format!("{slug}.{}", Ext::Typ))
}
