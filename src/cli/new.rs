// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use eyre::Context;

use crate::{
    config::{self, wanshi},
    environment, path_utils,
    slug::Ext,
};

#[derive(Parser)]
pub struct NewCommandCli {
    #[command(subcommand)]
    pub command: NewCommand,
}

#[derive(clap::Subcommand)]
pub enum NewCommand {
    /// Create a new wanshi site.
    #[command(visible_alias = "s")]
    Site(NewSiteCommand),

    /// Create a new config file.
    #[command(visible_alias = "c")]
    Config(NewConfigCommand),

    /// Create a new post.
    #[command(visible_alias = "p")]
    Post(NewPostCommand),

    /// Create `import-math.html`, the KaTeX loader needed by `tex()`.
    Katex(NewKatexCommand),
}

#[derive(clap::Args)]
pub struct NewKatexCommand {
    /// Path to the configuration file.
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG_PATH.into())]
    pub config: String,
}

/// Install the KaTeX loader.
///
/// Equations are MathML and render without a library, so nothing loads KaTeX
/// until a project asks for it. Only the `tex()` helper needs this.
pub fn new_katex(command: &NewKatexCommand) -> eyre::Result<()> {
    environment::init_environment(command.config.clone().into(), environment::BuildMode::Serve)?;

    let loader_path = environment::root_dir().join("import-math.html");
    if loader_path.exists() {
        color_print::ceprintln!(
            "<y>Note: `{}` already exists; leaving it alone.</>",
            loader_path
        );
        return Ok(());
    }

    std::fs::write(&loader_path, environment::default_import_math_html())
        .wrap_err_with(|| format!("failed to write KaTeX loader to `{}`", loader_path))?;
    color_print::ceprintln!("<g>[new]</> wrote {}", loader_path);

    Ok(())
}

#[derive(clap::Args)]
pub struct NewSiteCommand {
    /// Path to the new site.
    #[arg(required = true)]
    pub path: Utf8PathBuf,
}

pub fn new_site(command: &NewSiteCommand) -> eyre::Result<()> {
    let site_path = &command.path;
    if site_path.exists() {
        return Err(eyre::eyre!("Already exists: {}", site_path));
    }

    std::fs::create_dir_all(site_path).wrap_err("failed to create site directory")?;
    println!("Created new site at: {}", site_path);

    add_project_files(site_path)?;
    Ok(())
}

pub fn add_project_files(site_path: &Utf8Path) -> eyre::Result<()> {
    let default_config_path = site_path.join(config::DEFAULT_CONFIG_PATH);
    let default_source_dir = site_path.join(wanshi::DEFAULT_SOURCE_DIR);
    let default_assets_dir = site_path.join(wanshi::DEFAULT_ASSETS_DIR);
    let default_gitignore_path = site_path.join(".gitignore");

    // Create default config file in the new site directory
    new_config_inner(&default_config_path)?;

    // Create the default source directory `trees`
    std::fs::create_dir(&default_source_dir)
        .wrap_err("failed to create default source directory")?;

    // Create the default assets directory `assets`
    std::fs::create_dir(&default_assets_dir)
        .wrap_err("failed to create default assets directory")?;

    // Every generated page links a favicon, so ship one: an empty assets
    // directory means a 404 on every request until the author supplies it.
    std::fs::write(default_assets_dir.join("favicon.ico"), DEFAULT_FAVICON)
        .wrap_err("failed to create default favicon")?;

    // Create the .gitignore
    std::fs::write(&default_gitignore_path, DEFAULT_GITIGNORE)
        .wrap_err("failed to create .gitignore")?;

    // Create the default index section in the new site directory
    new_section_inner(
        &Utf8PathBuf::from(DEFAULT_SECTION_PATH),
        DEFAULT_TEMPLATE,
        &default_config_path,
    )?;

    // Create the default Typst library directory `trees/_lib`
    let default_lib_dir = default_source_dir.join("_lib");
    std::fs::create_dir(&default_lib_dir).wrap_err("failed to create default _lib directory")?;
    std::fs::write(
        default_lib_dir.join("wanshi.typ"),
        include_str!("../include/wanshi.typ"),
    )?;

    Ok(())
}

#[derive(clap::Args)]
pub struct NewConfigCommand {
    /// Path to the new configuration file.
    #[arg(default_value_t = config::DEFAULT_CONFIG_PATH.into())]
    pub path: String,
}

pub fn new_config(command: &NewConfigCommand) -> eyre::Result<()> {
    new_config_inner(&Utf8PathBuf::from(&command.path))
}

pub fn new_config_inner(config_path: &Utf8PathBuf) -> Result<(), eyre::Error> {
    let config = config::Config::default();
    let toml = toml::to_string(&config).wrap_err("failed to serialize default config")?;

    std::fs::write(config_path, toml).wrap_err("failed to create default config file")?;
    println!("Created new config at: {}", config_path);
    Ok(())
}

/// Icon shipped into a new site's assets directory.
///
/// Bytes rather than text: an ICO is binary, and `include_bytes!` keeps it in
/// the binary so `wanshi new site` stays a single self-contained command.
pub const DEFAULT_FAVICON: &[u8] = include_bytes!("../include/favicon.ico");

pub const DEFAULT_SECTION_PATH: &str = "./index.typ";

pub const DEFAULT_TEMPLATE: &str = "./template";

/// The library import is root-absolute so the skeleton compiles at any depth in
/// the source tree; a tree-relative import only resolves for notes sitting at
/// the very top.
pub const DEFAULT_TEMPLATE_CONTENT_TYPST: &str = r#"
#import "/_lib/wanshi.typ": *

#show: wanshi

#metadata((
  "title": "<FILE_NAME>",
))
"#;
pub const DEFAULT_GITIGNORE: &str = r#"# Generated by wanshi
/.cache
.DS_Store
"#;

#[derive(clap::Args)]
pub struct NewPostCommand {
    /// Path to the new section.
    #[arg(required = true)]
    pub path: Utf8PathBuf,

    /// Path to the template file to use for the new section.
    #[arg(short, long, default_value_t = DEFAULT_TEMPLATE.to_string())]
    pub template: String,

    /// Path to the configuration file (e.g., "Wanshi.toml").
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG_PATH.into())]
    pub config: String,
}

/// This function invoked the [`config::init_environment`] function to initialize the environment]
pub fn new_section(command: &NewPostCommand) -> eyre::Result<()> {
    new_section_inner(
        &command.path,
        &command.template,
        Utf8Path::new(&command.config),
    )
}

/// This function invoked the [`config::init_environment`] function to initialize the environment]
fn new_section_inner(path: &Utf8Path, template: &str, config: &Utf8Path) -> eyre::Result<()> {
    environment::init_environment(config.to_owned(), environment::BuildMode::Publish)?;

    let section_relative_path = normalize_new_section_path(path)?;
    let section_relative_path = strip_new_post_tree_prefix(
        section_relative_path.as_path(),
        &environment::trees_dir_without_root(),
    );
    let default_not_exists = template == DEFAULT_TEMPLATE && !std::fs::exists(template)?;

    let content = if default_not_exists {
        DEFAULT_TEMPLATE_CONTENT_TYPST.to_string()
    } else {
        std::fs::read_to_string(template)
            .map_err(|e| eyre::eyre!("failed to read template file: {}", e))?
    };

    let filestem = section_relative_path.file_stem().ok_or_else(|| {
        eyre::eyre!(
            "invalid section path (missing file name): {}",
            section_relative_path
        )
    })?;
    let content = substitute_file_name(&content, filestem);

    let section_path = environment::trees_dir().join(&section_relative_path);

    if section_path.exists() {
        return Err(eyre::eyre!("already exists: {}", section_path));
    } else {
        let parent = section_path.parent().ok_or_else(|| {
            eyre::eyre!(
                "failed to resolve parent directory for section path: {}",
                section_path
            )
        })?;
        std::fs::create_dir_all(parent)
            .map_err(|e| eyre::eyre!("failed to create section directory: {}", e))?;
    }

    std::fs::write(&section_path, content)
        .map_err(|e| eyre::eyre!("failed to create section file: {}", e))?;
    println!("Created new section at: {}", section_path);

    Ok(())
}

/// The placeholder `wanshi new post` replaces with the new file's stem.
const FILE_NAME_PLACEHOLDER: &str = "<FILE_NAME>";

/// Substitute [`FILE_NAME_PLACEHOLDER`] everywhere in a template *except* inside
/// comments.
///
/// A plain `str::replace` also rewrites the placeholder where a template
/// *documents* itself — a comment reading "the title placeholder below is
/// replaced with the file's stem" came out as "the title monoids below is
/// replaced …". A template that cannot explain its own placeholder without
/// having it clobbered is a poor template.
///
/// String literals are deliberately *not* skipped: the placeholder's usual home
/// is inside one (`"title": "<FILE_NAME>"`). They are tracked only so that a
/// `//` or `/*` appearing in a string is not mistaken for the start of a
/// comment.
///
/// Typst block comments nest, so the depth is counted rather than treated as a
/// flag.
fn substitute_file_name(content: &str, filestem: &str) -> String {
    #[derive(Clone, Copy)]
    enum State {
        Code,
        Str,
        LineComment,
        BlockComment(usize),
    }

    let mut out = String::with_capacity(content.len());
    let mut state = State::Code;
    let bytes = content.as_bytes();
    let mut i = 0;

    while i < content.len() {
        // Only ever advance to a char boundary, so slicing below is safe.
        let rest = &content[i..];
        match state {
            State::Code => {
                if rest.starts_with("//") {
                    state = State::LineComment;
                    out.push_str("//");
                    i += 2;
                } else if rest.starts_with("/*") {
                    state = State::BlockComment(1);
                    out.push_str("/*");
                    i += 2;
                } else if rest.starts_with(FILE_NAME_PLACEHOLDER) {
                    out.push_str(filestem);
                    i += FILE_NAME_PLACEHOLDER.len();
                } else {
                    if bytes[i] == b'"' {
                        state = State::Str;
                    }
                    let c = rest.chars().next().expect("non-empty by loop condition");
                    out.push(c);
                    i += c.len_utf8();
                }
            }
            State::Str => {
                if bytes[i] == b'\\' {
                    // Copy the escape and whatever it escapes, together.
                    let mut chars = rest.chars();
                    let bs = chars.next().expect("non-empty by loop condition");
                    out.push(bs);
                    i += bs.len_utf8();
                    if let Some(escaped) = chars.next() {
                        out.push(escaped);
                        i += escaped.len_utf8();
                    }
                } else if rest.starts_with(FILE_NAME_PLACEHOLDER) {
                    out.push_str(filestem);
                    i += FILE_NAME_PLACEHOLDER.len();
                } else {
                    if bytes[i] == b'"' {
                        state = State::Code;
                    }
                    let c = rest.chars().next().expect("non-empty by loop condition");
                    out.push(c);
                    i += c.len_utf8();
                }
            }
            State::LineComment => {
                let c = rest.chars().next().expect("non-empty by loop condition");
                if c == '\n' {
                    state = State::Code;
                }
                out.push(c);
                i += c.len_utf8();
            }
            State::BlockComment(depth) => {
                if rest.starts_with("/*") {
                    state = State::BlockComment(depth + 1);
                    out.push_str("/*");
                    i += 2;
                } else if rest.starts_with("*/") {
                    state = if depth == 1 {
                        State::Code
                    } else {
                        State::BlockComment(depth - 1)
                    };
                    out.push_str("*/");
                    i += 2;
                } else {
                    let c = rest.chars().next().expect("non-empty by loop condition");
                    out.push(c);
                    i += c.len_utf8();
                }
            }
        }
    }

    out
}

fn normalize_new_section_path(path: &Utf8Path) -> eyre::Result<Utf8PathBuf> {
    match path.extension() {
        // Any recognized source extension is kept as written, so existing sites
        // using `.typst` can keep creating notes that match their convention.
        Some(ext) if ext.parse::<Ext>().is_ok() => Ok(path.to_owned()),
        Some(other) => Err(eyre::eyre!(
            "unsupported section extension `.{}`; expected `.{}` or `.{}`",
            other,
            Ext::Typ,
            Ext::Typst
        )),
        None => {
            let mut section_path = path.to_owned();
            section_path.set_extension(Ext::Typ.to_string());
            Ok(section_path)
        }
    }
}

fn strip_new_post_tree_prefix(path: &Utf8Path, trees_dir_without_root: &str) -> Utf8PathBuf {
    let normalized_path = Utf8PathBuf::from(path_utils::pretty_path(path));
    let normalized_trees_dir = Utf8PathBuf::from(path_utils::pretty_path(Utf8Path::new(
        trees_dir_without_root,
    )));

    if normalized_path.as_str().is_empty() || normalized_trees_dir.as_str().is_empty() {
        return normalized_path;
    }

    match normalized_path.strip_prefix(normalized_trees_dir.as_path()) {
        Ok(stripped) if !stripped.as_str().is_empty() => stripped.to_owned(),
        _ => normalized_path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_new_section_path_appends_typ_when_missing_extension() {
        let path =
            normalize_new_section_path(Utf8Path::new("notes/foo")).expect("should normalize path");

        assert_eq!(path, Utf8PathBuf::from("notes/foo.typ"));
    }

    #[test]
    fn test_normalize_new_section_path_respects_explicit_extension() {
        // Both source extensions are preserved as written, so a site that still
        // uses `.typst` keeps creating notes that match its own convention.
        for name in ["notes/foo.typ", "notes/foo.typst"] {
            let path = normalize_new_section_path(Utf8Path::new(name))
                .expect("should preserve an explicit source extension");
            assert_eq!(path, Utf8PathBuf::from(name));
        }
    }

    #[test]
    fn test_default_favicon_is_a_well_formed_icon() {
        // Every generated page links this file, so a truncated or wrongly typed
        // include would 404 on every request of every scaffolded site.
        let reserved = u16::from_le_bytes([DEFAULT_FAVICON[0], DEFAULT_FAVICON[1]]);
        let kind = u16::from_le_bytes([DEFAULT_FAVICON[2], DEFAULT_FAVICON[3]]);
        let count = u16::from_le_bytes([DEFAULT_FAVICON[4], DEFAULT_FAVICON[5]]);

        assert_eq!(reserved, 0, "ICONDIR reserved field must be zero");
        assert_eq!(kind, 1, "resource type must be icon, not cursor");
        assert!(count > 0, "icon must declare at least one image");

        // Each directory entry must address bytes that are actually present.
        for index in 0..count as usize {
            let entry = 6 + 16 * index;
            let size = u32::from_le_bytes(
                DEFAULT_FAVICON[entry + 8..entry + 12]
                    .try_into()
                    .expect("4 bytes"),
            ) as usize;
            let offset = u32::from_le_bytes(
                DEFAULT_FAVICON[entry + 12..entry + 16]
                    .try_into()
                    .expect("4 bytes"),
            ) as usize;

            assert!(
                offset + size <= DEFAULT_FAVICON.len(),
                "entry {index} points past the end of the file"
            );
        }
    }

    #[test]
    fn test_default_template_import_resolves_at_any_depth() {
        // A tree-relative import only resolves for notes at the top of the
        // source tree, so the scaffold must use the root-absolute form.
        assert!(DEFAULT_TEMPLATE_CONTENT_TYPST.contains(r#"#import "/_lib/wanshi.typ""#));
    }

    #[test]
    fn test_normalize_new_section_path_rejects_unsupported_extension() {
        let err = normalize_new_section_path(Utf8Path::new("notes/foo.md")).unwrap_err();
        assert!(format!("{err}").contains("unsupported section extension"));
    }

    #[test]
    fn test_strip_new_post_tree_prefix_strips_leading_tree_directory() {
        let stripped = strip_new_post_tree_prefix(Utf8Path::new("trees/notes/a.typst"), "trees");
        assert_eq!(stripped, Utf8PathBuf::from("notes/a.typst"));
    }

    #[test]
    fn test_strip_new_post_tree_prefix_handles_dot_prefix_path() {
        let stripped = strip_new_post_tree_prefix(Utf8Path::new("./trees/notes/a.typst"), "trees");
        assert_eq!(stripped, Utf8PathBuf::from("notes/a.typst"));
    }

    #[test]
    fn test_strip_new_post_tree_prefix_keeps_non_tree_prefixed_path() {
        let stripped = strip_new_post_tree_prefix(Utf8Path::new("notes/a.typst"), "trees");
        assert_eq!(stripped, Utf8PathBuf::from("notes/a.typst"));
    }

    #[test]
    fn test_strip_new_post_tree_prefix_supports_nested_tree_root() {
        let stripped = strip_new_post_tree_prefix(
            Utf8Path::new("content/trees/notes/a.typst"),
            "content/trees",
        );
        assert_eq!(stripped, Utf8PathBuf::from("notes/a.typst"));
    }

    #[test]
    fn test_substitute_file_name_replaces_in_code_and_strings() {
        let out = substitute_file_name(r#"#metadata(("title": "<FILE_NAME>"))"#, "monoids");
        assert_eq!(out, r#"#metadata(("title": "monoids"))"#);
    }

    #[test]
    fn test_substitute_file_name_skips_line_comments() {
        // A template documenting its own placeholder must survive intact.
        let template = "// replaced with <FILE_NAME> on creation\n\"<FILE_NAME>\"";
        let out = substitute_file_name(template, "monoids");
        assert_eq!(out, "// replaced with <FILE_NAME> on creation\n\"monoids\"");
    }

    #[test]
    fn test_substitute_file_name_skips_block_comments() {
        let out = substitute_file_name("/* <FILE_NAME> */ <FILE_NAME>", "alice");
        assert_eq!(out, "/* <FILE_NAME> */ alice");
    }

    #[test]
    fn test_substitute_file_name_handles_nested_block_comments() {
        // Typst block comments nest, so the inner close must not end the outer.
        let out = substitute_file_name("/* a /* <FILE_NAME> */ b */ <FILE_NAME>", "alice");
        assert_eq!(out, "/* a /* <FILE_NAME> */ b */ alice");
    }

    #[test]
    fn test_substitute_file_name_ignores_comment_markers_inside_strings() {
        // "//" in a string does not start a comment, so the later placeholder
        // is still substituted.
        let out = substitute_file_name(r#""http://x" <FILE_NAME>"#, "alice");
        assert_eq!(out, r#""http://x" alice"#);
    }

    #[test]
    fn test_substitute_file_name_handles_escaped_quote_in_string() {
        let out = substitute_file_name(r#""a\"b" <FILE_NAME>"#, "alice");
        assert_eq!(out, r#""a\"b" alice"#);
    }

    #[test]
    fn test_substitute_file_name_preserves_multibyte_content() {
        let out = substitute_file_name("// 参考 <FILE_NAME>\n\"<FILE_NAME>\" — é", "alice");
        assert_eq!(out, "// 参考 <FILE_NAME>\n\"alice\" — é");
    }

    #[test]
    fn test_substitute_file_name_replaces_every_occurrence_outside_comments() {
        let out = substitute_file_name("<FILE_NAME> and <FILE_NAME>", "alice");
        assert_eq!(out, "alice and alice");
    }
}
