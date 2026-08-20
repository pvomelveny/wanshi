// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::collections::HashMap;

use eyre::{bail, eyre, WrapErr};

use crate::{
    compiler::{self, links, section::UnresolvedSection},
    config,
    environment::{self, BuildMode},
    slug::Slug,
};

#[derive(clap::Args)]
pub struct CheckCommand {
    /// Path to the configuration file (e.g., "Wanshi.toml").
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG_PATH.into())]
    config: String,

    /// Treat warnings as errors.
    #[arg(long, default_value_t = false)]
    strict: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Severity {
    Error,
    Warning,
    Hint,
}

struct Diagnostic {
    severity: Severity,
    message: String,
}

impl Diagnostic {
    fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
        }
    }

    fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
        }
    }

    fn hint(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Hint,
            message: message.into(),
        }
    }
}

pub fn check(command: &CheckCommand) -> eyre::Result<()> {
    environment::init_environment(command.config.clone().into(), BuildMode::Check)?;

    let trees_dir = environment::trees_dir();
    let workspace = compiler::all_trees_source(trees_dir.as_path())
        .wrap_err_with(|| eyre!("failed to scan trees dir `{}`", trees_dir))?;

    let mut diagnostics = Vec::new();
    if workspace.slug_exts.is_empty() {
        diagnostics.push(Diagnostic::hint(format!(
            "No sections found under `{}`.",
            trees_dir
        )));
    }
    if !workspace
        .slug_exts
        .contains_key(&Slug::new(compiler::INDEX_SLUG))
    {
        diagnostics.push(Diagnostic::warning(format!(
            "Missing `{}` section. Add `{}.{}`.",
            compiler::INDEX_SLUG,
            compiler::INDEX_SLUG,
            crate::slug::Ext::Typ
        )));
    }

    let shallows = parse_shallows_no_cache(&workspace, &mut diagnostics);
    collect_dangling_local_links(&shallows, &mut diagnostics);
    let has_parse_errors = diagnostics.iter().any(|d| d.severity == Severity::Error);
    if !has_parse_errors {
        validate_compile_graph(&shallows, &mut diagnostics);
    }

    for diagnostic in &diagnostics {
        print_diagnostic(diagnostic);
    }

    let errors = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .count();
    let hints = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Hint)
        .count();

    let strict_note = if command.strict { " (strict mode)" } else { "" };
    println!(
        "Check result: {} error(s), {} warning(s), {} hint(s){}.",
        errors, warnings, hints, strict_note
    );

    if errors > 0 {
        bail!("check failed with {} error(s)", errors);
    }
    if command.strict && warnings > 0 {
        bail!("check failed in strict mode with {} warning(s)", warnings);
    }
    Ok(())
}

fn parse_shallows_no_cache(
    workspace: &compiler::Workspace,
    diagnostics: &mut Vec<Diagnostic>,
) -> HashMap<Slug, UnresolvedSection> {
    let (shallows, failures) = links::parse_all_sections(workspace);
    diagnostics.extend(failures.into_iter().map(Diagnostic::error));
    shallows
}

fn collect_dangling_local_links(
    shallows: &HashMap<Slug, UnresolvedSection>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let refs_dir = environment::refs_dir();
    for dangling in links::dangling_local_links(shallows) {
        let mut message = format!(
            "Dangling local link in `{}`: `{}` resolves to missing section `{}`.",
            dangling.from, dangling.url, dangling.target
        );
        // A missing work is a different problem from a missing note: the note
        // has to be written, the work only has to be imported.
        if dangling.target.as_str().starts_with(&refs_dir) {
            message.push_str(" Run `wanshi refs sync` to generate it from the bibliography.");
        }
        diagnostics.push(Diagnostic::warning(message));
    }
}

fn validate_compile_graph(
    shallows: &HashMap<Slug, UnresolvedSection>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if shallows.is_empty() {
        return;
    }
    if let Err(err) = compiler::state::compile_all_without_missing_index_warning(shallows) {
        diagnostics.push(Diagnostic::error(format!(
            "Failed to compile section graph: {err:#}"
        )));
    }
}

fn print_diagnostic(diagnostic: &Diagnostic) {
    match diagnostic.severity {
        Severity::Error => color_print::ceprintln!("<r>Error:</> {}", diagnostic.message),
        Severity::Warning => color_print::ceprintln!("<y>Warning:</> {}", diagnostic.message),
        Severity::Hint => color_print::ceprintln!("<dim>Hint:</> {}", diagnostic.message),
    }
}
