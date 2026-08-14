use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use leo::{
    DerivedFile, ExternalFilter, ImportMode, ImportOptions, InspectSelector, LeoDocument, NodeId,
    OperationBatch, PositionId, import_files, load_matching_external_files, render_compact,
    render_search_compact, search_outline, select_subtrees, sync_document,
};
use regex::Regex;
use std::{fs, path::PathBuf};

mod install;
#[cfg(all(feature = "tui", feature = "syntax"))]
mod syntax;
#[cfg(feature = "tui")]
mod tui;

#[derive(Parser)]
#[command(name = "cub", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
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
        /// Insert below the node with this GNX instead of at the root.
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
    match cli.command {
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
                    parent: parent.map(NodeId),
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
