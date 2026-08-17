use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use leo::{
    DerivedFile, ExternalFilter, ImportMode, ImportOptions, InspectSelector, LeoDocument, NodeId,
    OperationBatch, PositionId, import_files, load_matching_external_files, render_compact,
    render_outline_with_options, render_search_compact, search_outline, select_subtrees,
    sync_document,
};
use regex::Regex;
use std::{fs, path::PathBuf};

mod install;
#[cfg(all(feature = "tui", feature = "syntax"))]
mod syntax;
#[cfg(feature = "tui")]
mod tui;

#[derive(Parser)]
#[command(name = "cub", version, about, args_conflicts_with_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    /// Outline to browse interactively when no subcommand is given.
    #[cfg(feature = "tui")]
    file: Option<PathBuf>,
    /// Show only the hierarchy stored directly in the .leo XML.
    #[cfg(feature = "tui")]
    #[arg(long, requires = "file")]
    no_derived: bool,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum InspectFormat {
    #[default]
    Compact,
    Json,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum CliImportMode {
    #[default]
    Auto,
    Edit,
    Clean,
}

impl From<CliImportMode> for ImportMode {
    fn from(value: CliImportMode) -> Self {
        match value {
            CliImportMode::Auto => Self::Auto,
            CliImportMode::Edit => Self::Edit,
            CliImportMode::Clean => Self::Clean,
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// Create a new Leo outline without overwriting an existing file.
    New {
        file: PathBuf,
        /// Headline for the initial root node.
        #[arg(long, default_value = "New Headline")]
        headline: String,
    },
    /// Add nodes using slash-separated headline paths.
    Add {
        /// Outline to modify.
        file: PathBuf,
        /// Headline paths, for example "Project/Tasks/First task".
        #[arg(required = true)]
        paths: Vec<String>,
    },
    /// Install the bundled skill into ~/.claude/skills.
    InstallSkills,
    /// Browse an outline interactively (read-only).
    #[cfg(feature = "tui")]
    Tui {
        file: PathBuf,
        /// Show only the hierarchy stored directly in the .leo XML.
        #[arg(long)]
        no_derived: bool,
    },
    Inspect {
        file: PathBuf,
        /// Show the external @file/@clean subtree matching this path or basename.
        external: Option<String>,
        /// Show all occurrences of the subtree with this GNX.
        #[arg(long, conflicts_with_all = ["external", "position"])]
        gnx: Option<String>,
        /// Show the subtree at this occurrence path (for example, 0/2/1).
        #[arg(long, conflicts_with_all = ["external", "gnx"])]
        position: Option<String>,
        /// Search headlines and bodies; repeat for OR matching.
        #[arg(long, value_name = "REGEX")]
        search: Vec<String>,
        /// Output format. Compact is intended for reading; JSON for scripts.
        #[arg(long, value_enum, default_value_t)]
        format: InspectFormat,
    },
    /// Render a selected outline hierarchy as a Markdown list.
    Render {
        file: PathBuf,
        /// Show the subtree at this occurrence path (for example, 0/2/1).
        #[arg(long, conflicts_with_all = ["external", "gnx"])]
        position: Option<String>,
        /// Show all occurrences of the subtree with this GNX.
        #[arg(long, conflicts_with_all = ["external", "position"])]
        gnx: Option<String>,
        /// Show the subtree matching this external filename.
        #[arg(long, conflicts_with_all = ["gnx", "position"])]
        external: Option<String>,
        /// Mark this occurrence and its ancestors as current.
        #[arg(long)]
        current: Option<String>,
        /// Collapse branches unless they contain the current or an expanded position.
        #[arg(long)]
        collapsed: bool,
        /// Expand a position; may be repeated.
        #[arg(long, value_name = "POSITION")]
        expand: Vec<String>,
    },
    Validate {
        file: PathBuf,
    },
    /// Import files as @auto, @edit, or @clean nodes.
    Import {
        /// Outline to modify.
        file: PathBuf,
        /// Files or directories to import.
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        #[arg(long, value_enum, default_value_t)]
        mode: CliImportMode,
        /// Import directories recursively.
        #[arg(long)]
        recursive: bool,
        /// Preserve directory structure using @path nodes.
        #[arg(long, conflicts_with = "no_paths")]
        paths: bool,
        /// Import files directly below the destination.
        #[arg(long, conflicts_with = "paths")]
        no_paths: bool,
        /// Insert below this GNX or slash-separated headline path.
        #[arg(long)]
        parent: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Apply a transactional JSON operation batch to an outline.
    #[command(after_help = r#"OPERATIONS FORMAT:
  The file is a JSON object with an "operations" array. Supported operations:

  {"op":"set-headline","node":"<gnx>","headline":"new","expected":"old"}
  {"op":"set-body","node":"<gnx>","body":"new","expected":"old"}
  {"op":"insert","parent":"<parent-gnx>","index":0,
   "node":{"id":"<new-gnx>","headline":"New node","body":""}}
  {"op":"clone","parent":"<parent-gnx>","index":0,"node":"<gnx>"}
  {"op":"remove","position":"<position>"}

  "parent" is a GNX and may be null for the outline root. Inserting below a
  cloned parent affects all its occurrences. "index" and "expected" are
  optional. "position" is an index path such as "0/2/1" and identifies one
  clone occurrence. The complete batch is committed only if every operation
  succeeds.

EXAMPLE:
  {"operations":[{"op":"set-body","node":"ekr.1","expected":"old","body":"new"}]}"#)]
    Apply {
        file: PathBuf,
        operations: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
    /// Synchronize external @file and @clean nodes into an outline.
    Sync {
        file: PathBuf,
        /// External filename to sync. Omit to sync all external nodes.
        external: Option<String>,
        /// Sync the external node with this GNX.
        #[arg(long, conflicts_with = "external")]
        gnx: Option<String>,
        /// Validate and report changes without writing the outline.
        #[arg(long)]
        dry_run: bool,
    },
    /// Parse a thin derived file and print its reconstructed outline.
    InspectDerived {
        derived: PathBuf,
        #[arg(long)]
        summary: bool,
    },
    Diff {
        before: PathBuf,
        after: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    #[cfg(feature = "tui")]
    let command = match (cli.command, cli.file) {
        (Some(command), None) => command,
        (None, Some(file)) => Command::Tui {
            file,
            no_derived: cli.no_derived,
        },
        (None, None) => {
            Cli::command().print_help()?;
            println!();
            return Ok(());
        }
        (Some(_), Some(_)) => unreachable!("clap rejects arguments alongside subcommands"),
    };
    #[cfg(not(feature = "tui"))]
    let Some(command) = cli.command else {
        Cli::command().print_help()?;
        println!();
        return Ok(());
    };
    match command {
        Command::New { file, headline } => {
            LeoDocument::new(headline)
                .save_new(&file)
                .with_context(|| format!("create {}", file.display()))?;
            println!("{}", file.display());
        }
        Command::Add { file, paths } => {
            let mut document = LeoDocument::open(&file)?;
            let report = document.outline.add_headline_paths(&paths)?;
            if report.created > 0 {
                document.save(&file)?;
            }
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::InstallSkills => install::install_skills()?,
        #[cfg(feature = "tui")]
        Command::Tui { file, no_derived } => tui::run(file, !no_derived)?,
        Command::Inspect {
            file,
            external,
            gnx,
            position,
            search,
            format,
        } => {
            let mut outline = LeoDocument::open(&file)?.outline;
            let patterns = search
                .iter()
                .map(|pattern| Regex::new(pattern))
                .collect::<Result<Vec<_>, _>>()?;
            if let Some(gnx) = gnx.as_deref() {
                load_matching_external_files(&mut outline, &file, ExternalFilter::Gnx(gnx))?;
            } else if !patterns.is_empty() {
                load_matching_external_files(
                    &mut outline,
                    &file,
                    ExternalFilter::Search(&patterns),
                )?;
            } else if let Some(external) = external.as_deref() {
                load_matching_external_files(&mut outline, &file, ExternalFilter::File(external))?;
            }
            let selected = if let Some(external) = external.as_deref() {
                select_subtrees(&outline, InspectSelector::File(external))?
            } else if let Some(gnx) = gnx.as_deref() {
                select_subtrees(&outline, InspectSelector::Gnx(gnx))?
            } else if let Some(position) = position {
                select_subtrees(&outline, InspectSelector::Position(&PositionId(position)))?
            } else {
                outline
            };
            if !search.is_empty() {
                let matches = search_outline(&selected, &patterns);
                match format {
                    InspectFormat::Compact => print!("{}", render_search_compact(&matches)),
                    InspectFormat::Json => {
                        println!("{}", serde_json::to_string_pretty(&matches)?)
                    }
                }
            } else {
                match format {
                    InspectFormat::Compact => print!("{}", render_compact(&selected)),
                    InspectFormat::Json => {
                        println!("{}", serde_json::to_string_pretty(&selected)?)
                    }
                }
            }
        }
        Command::Render {
            file,
            position,
            gnx,
            external,
            current,
            collapsed,
            expand,
        } => {
            let mut outline = LeoDocument::open(&file)?.outline;
            if let Some(gnx) = gnx.as_deref() {
                load_matching_external_files(&mut outline, &file, ExternalFilter::Gnx(gnx))?;
            } else if let Some(external) = external.as_deref() {
                load_matching_external_files(&mut outline, &file, ExternalFilter::File(external))?;
            }
            let selected = if let Some(external) = external.as_deref() {
                select_subtrees(&outline, InspectSelector::File(external))?
            } else if let Some(gnx) = gnx.as_deref() {
                select_subtrees(&outline, InspectSelector::Gnx(gnx))?
            } else if let Some(position) = position {
                select_subtrees(&outline, InspectSelector::Position(&PositionId(position)))?
            } else {
                outline
            };
            let current = current.map(PositionId);
            let expand = expand.into_iter().map(PositionId).collect::<Vec<_>>();
            print!(
                "{}",
                render_outline_with_options(&selected, current.as_ref(), collapsed, &expand)
            );
        }
        Command::Validate { file } => {
            let errors = LeoDocument::open(file)?.outline.validate();
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &errors.iter().map(ToString::to_string).collect::<Vec<_>>()
                )?
            );
            if !errors.is_empty() {
                bail!("outline is invalid")
            }
        }
        Command::Import {
            file,
            inputs,
            mode,
            recursive,
            paths,
            no_paths,
            parent,
            dry_run,
        } => {
            let mut document = LeoDocument::open(&file)?;
            let parent = parent
                .map(|selector| {
                    let id = NodeId(selector.clone());
                    if document.outline.nodes.contains_key(&id) {
                        Ok(id)
                    } else {
                        document.outline.resolve_headline_path(&selector)
                    }
                })
                .transpose()?;
            let has_directory = inputs.iter().any(|path| path.is_dir());
            let report = import_files(
                &mut document,
                &file,
                &inputs,
                &ImportOptions {
                    mode: mode.into(),
                    recursive,
                    paths: if no_paths {
                        false
                    } else {
                        paths || has_directory
                    },
                    parent,
                    dry_run,
                },
            )?;
            if !dry_run && report.imported > 0 {
                document.save(&file)?;
            }
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::Apply {
            file,
            operations,
            dry_run,
        } => {
            let mut doc = LeoDocument::open(&file)?;
            let batch: OperationBatch = serde_json::from_str(
                &fs::read_to_string(operations).context("read operations file")?,
            )?;
            let report = doc.outline.apply(&batch)?;
            if !dry_run {
                doc.save(file)?;
            }
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::Sync {
            file,
            external,
            gnx,
            dry_run,
        } => {
            let mut document = LeoDocument::open(&file)?;
            let report = sync_document(
                &mut document,
                &file,
                external.as_deref(),
                gnx.as_deref(),
                dry_run,
            )?;
            if !dry_run && report.changed > 0 {
                document.save(&file)?;
            }
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::InspectDerived { derived, summary } => {
            let source = fs::read_to_string(derived).context("read derived file")?;
            let parsed = DerivedFile::parse(&source)?;
            if summary {
                println!(
                    "{}",
                    serde_json::json!({
                        "root": parsed.root,
                        "nodes": parsed.outline.nodes.len(),
                        "positions": count_positions(&parsed.outline.roots),
                        "start_delimiter": parsed.start_delimiter,
                        "end_delimiter": parsed.end_delimiter
                    })
                );
            } else {
                println!("{}", serde_json::to_string_pretty(&parsed.outline)?);
            }
        }
        Command::Diff { before, after } => {
            let a = LeoDocument::open(before)?.outline;
            let b = LeoDocument::open(after)?.outline;
            println!(
                "{}",
                serde_json::json!({"equal": a == b, "before": a, "after": b})
            );
        }
    }
    Ok(())
}

fn count_positions(positions: &[leo::Position]) -> usize {
    positions
        .iter()
        .map(|position| 1 + count_positions(&position.children))
        .sum()
}

#[cfg(all(test, feature = "tui"))]
mod tests {
    use super::*;

    #[test]
    fn accepts_an_outline_without_the_tui_subcommand() {
        let cli = Cli::try_parse_from(["cub", "notes.leo"]).unwrap();

        assert!(cli.command.is_none());
        assert_eq!(cli.file, Some(PathBuf::from("notes.leo")));
        assert!(!cli.no_derived);
    }

    #[test]
    fn explicit_subcommands_still_parse_normally() {
        let cli = Cli::try_parse_from(["cub", "validate", "notes.leo"]).unwrap();

        assert!(matches!(
            cli.command,
            Some(Command::Validate { file }) if file == PathBuf::from("notes.leo")
        ));
        assert!(cli.file.is_none());
    }

    #[test]
    fn shorthand_accepts_tui_options() {
        let cli = Cli::try_parse_from(["cub", "notes.leo", "--no-derived"]).unwrap();

        assert_eq!(cli.file, Some(PathBuf::from("notes.leo")));
        assert!(cli.no_derived);
    }
}
