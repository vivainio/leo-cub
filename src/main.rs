use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use leo::{LeoDocument, OperationBatch};
use std::{fs, path::PathBuf};

#[cfg(feature = "tui")]
mod tui;

#[derive(Parser)]
#[command(version, about)]
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
    Diff {
        before: PathBuf,
        after: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        #[cfg(feature = "tui")]
        Command::Tui { file } => tui::run(file)?,
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
