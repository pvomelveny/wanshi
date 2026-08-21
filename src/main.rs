// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

mod assets_sync;
mod atomic_text;
mod cli;
mod compiler;
mod config;
mod entry;
mod environment;
mod footer_sort;
mod html_flake;
mod html_macro;
mod html_text;
mod ordered_map;
mod path_utils;
mod recorder;
mod refs;
mod slug;
#[cfg(test)]
mod test_io;
mod typst_cli;

use clap::{
    builder::{
        styling::{AnsiColor, Effects},
        Styles,
    },
    Parser,
};

use crate::cli::{
    build::BuildCommand,
    check::CheckCommand,
    init::InitCommand,
    new::{NewCommand, NewCommandCli},
    refs::{RefsCommand, RefsCommandCli},
    serve::ServeCommand,
    snip::SnipCommand,
    upgrade::UpgradeCommand,
};

const STYLES: Styles = Styles::styled()
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Blue.on_default());

#[derive(Parser)]
#[command(version, about, long_about = None, styles=STYLES)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Create a new wanshi site / config / post.
    #[command(visible_alias = "n")]
    New(NewCommandCli),

    /// Create a new wanshi site in an existing directory.
    #[command(visible_alias = "i")]
    Init(InitCommand),

    /// Compile current workspace dir to HTMLs.
    ///
    /// Emits "wanshi.json" and "wanshi.graph.json" by default (override with output flags).
    #[command(visible_alias = "b")]
    Build(BuildCommand),

    /// Validate sections and graph without generating build artifacts.
    #[command(visible_alias = "c")]
    Check(CheckCommand),

    /// Serve a forest at http://localhost:<port>, and rebuilds it on changes.
    ///
    /// Does not emit "wanshi.json" / "wanshi.graph.json" by default.
    ///
    /// Server by default depends on the miniserve program in the user's environment.
    /// Also see the configuration file (e.g., "Wanshi.toml").
    #[command(visible_alias = "s")]
    Serve(ServeCommand),

    /// Manage bibliographic references.
    ///
    /// `sync` generates a note for every cited work that lacks one; `export`
    /// prints the bibliography a set of notes cites.
    #[command(visible_alias = "r")]
    Refs(RefsCommandCli),

    /// Generate VSCode style snippets file.
    #[command()]
    Snip(SnipCommand),

    /// Upgrade config & Typst library files.
    #[command(visible_alias = "u")]
    Upgrade(UpgradeCommand),
}

fn main() -> eyre::Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Command::New(NewCommandCli { command }) => match command {
            NewCommand::Site(command) => crate::cli::new::new_site(command)?,
            NewCommand::Post(command) => crate::cli::new::new_section(command)?,
            NewCommand::Config(command) => crate::cli::new::new_config(command)?,
            NewCommand::Katex(command) => crate::cli::new::new_katex(command)?,
        },
        Command::Init(command) => crate::cli::init::init(command)?,
        Command::Serve(command) => crate::cli::serve::serve(command)?,
        Command::Build(command) => crate::cli::build::build(command)?,
        Command::Check(command) => crate::cli::check::check(command)?,
        Command::Refs(RefsCommandCli { command }) => match command {
            RefsCommand::Sync(command) => crate::cli::refs::sync(command)?,
            RefsCommand::Export(command) => crate::cli::refs::export(command)?,
        },
        Command::Snip(command) => crate::cli::snip::snip(command)?,
        Command::Upgrade(command) => crate::cli::upgrade::upgrade(command)?,
    };
    Ok(())
}
