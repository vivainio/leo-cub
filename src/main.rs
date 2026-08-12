use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use leo::{DerivedFile, LeoDocument, OperationBatch, sync_document};
use std::{fs, path::PathBuf};

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

#[derive(Subcommand)]
enum Command {
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
    },
    Validate {
        file: PathBuf,
    },
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
        #[cfg(feature = "tui")]
        Command::Tui { file, no_derived } => tui::run(file, !no_derived)?,
        Command::Inspect { file } => println!(
            "{}",
            serde_json::to_string_pretty(&LeoDocument::open(file)?.outline)?
        ),
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
