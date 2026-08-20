use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use leo::{
    DerivedFile, ExternalFilter, ImportMode, ImportOptions, InspectSelector, LeoDocument, NodeId,
    OperationBatch, PositionId, import_files, json_tree, load_matching_external_files,
    render_compact, render_outline_with_options, render_search_compact, search_outline,
    select_subtrees, sync_document,
};
use regex::Regex;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

mod install;
#[cfg(feature = "rhai")]
mod rhai_run;
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
    /// Nested JSON addressable by headline path (`get "A" | get "B"` in
    /// nu), instead of by GNX or position index. Fails if any two siblings
    /// share a headline or a headline collides with a reserved _gnx/_body
    /// key, rather than silently degrading.
    JsonTree,
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
        /// Headline for the initial root node. Ignored when --import is given.
        #[arg(long, default_value = "New Headline", conflicts_with = "import")]
        headline: String,
        /// Bootstrap the outline from files or directories instead of a blank
        /// headline; the first node is the import itself (for example @path).
        #[arg(long, value_name = "PATH")]
        import: Vec<PathBuf>,
        #[arg(long, value_enum, default_value_t, requires = "import")]
        mode: CliImportMode,
        /// Import directories non-recursively (bootstrapping defaults to recursive).
        #[arg(long, requires = "import")]
        no_recursive: bool,
        /// Import files directly without preserving directory structure.
        #[arg(long, requires = "import")]
        no_paths: bool,
        #[arg(long, requires = "import")]
        dry_run: bool,
    },
    /// Add nodes using slash-separated headline paths.
    Add {
        /// Outline to modify.
        file: PathBuf,
        /// Headline paths, for example "Project/Tasks/First task". Write
        /// "\/" for a literal slash and "\\" for a literal backslash within
        /// one headline.
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
    /// Run a Rhai test script against an outline.
    #[cfg(feature = "rhai")]
    #[command(
        after_help = r#"The script drives an outline through a small API instead of
pressing keys, so it works as a scriptable replacement for a jsonl/TUI-driven
integration test:

  let doc = open("notes.leo");     // load an outline
  let gnx = doc.add("A/B/C");      // ensure a headline path exists
  doc.set_body(gnx, "hello");
  assert_eq(doc.headline(gnx), "C");
  doc.save();                      // write back to notes.leo

Other Doc methods: gnx(path), headline(gnx), set_headline(gnx, text),
body(gnx), render(), validate(), apply(json) (a "cub apply"-style operation
batch, returns the report as JSON), count(), save_as(path). `assert(cond)`,
`assert(cond, msg)`, and `assert_eq(a, b)` abort the script with a non-zero
exit on failure; `print`/`debug` go straight to stdout/stderr."#
    )]
    Run {
        /// Rhai script to execute.
        script: PathBuf,
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
        /// Show the subtree at this slash-separated headline path (for
        /// example, "Project/Tasks/First task"), resolved the same way
        /// "cub add" resolves its paths.
        #[arg(long, conflicts_with_all = ["external", "gnx", "position"])]
        headline: Option<String>,
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
        /// Show the subtree at this slash-separated headline path (for
        /// example, "Project/Tasks/First task"), resolved the same way
        /// "cub add" resolves its paths.
        #[arg(long, conflicts_with_all = ["external", "gnx", "position"])]
        headline: Option<String>,
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
  {"op":"insert-tree","parent":"<parent-gnx>","index":0,
   "tree":{"Headline":{"_gnx":"<optional-gnx>","_body":"text",
   "Child headline":{"_body":"..."}}}}
  {"op":"clone","parent":"<parent-gnx>","index":0,"node":"<gnx>"}
  {"op":"remove","position":"<position>"}
  {"op":"replace-tree","headline":"Slash/Path/To/Node",
   "tree":{"New headline":{"_body":"..."}}}
  {"op":"merge-tree","parent":"<parent-gnx>",
   "tree":{"Existing headline":{"_body":"updated",
   "New headline":{"_body":"..."}}}}

  "parent" is a GNX and may be null for the outline root. Inserting below a
  cloned parent affects all its occurrences. "index" and "expected" are
  optional. "position" is an index path such as "0/2/1" and identifies one
  clone occurrence. The complete batch is committed only if every operation
  succeeds. Pass "-" for OPERATIONS to read the batch from stdin.

  "insert-tree" adds a whole subtree (or several sibling subtrees) in one
  operation, using the same shape "inspect --format json-tree" prints: a map
  from headline to a node with reserved "_gnx"/"_body" keys plus one key per
  child headline. Both are optional: "_body" defaults to "", and a node
  missing "_gnx" gets a fresh id from the batch's top-level "gnx-prefix"
  (default "cub"), formatted like the ids "import" generates.

  "insert-tree" and "merge-tree" may give "parent-headline" instead of
  "parent": a slash-separated headline path, resolved and created the same
  way "cub add" resolves its paths — reusing any existing prefix and adding
  only the missing segments. At most one of "parent"/"parent-headline" may
  be given; omitting both means the outline root. Write "\/" for a literal
  slash and "\\" for a literal backslash within one path component, for
  headlines that contain a "/" themselves (a branch-name-style PR title,
  say); any other backslash is kept as-is.

  "replace-tree" removes a node's defining occurrence and its subtree, then
  inserts a fresh "tree" (same shape as "insert-tree") in its place at the
  same parent/index. The target is either "node" (a GNX) or "headline" (a
  slash-separated headline path, resolved the same way as "cub add"'s
  paths) — exactly one of the two. The removed node's GNX is discarded; the
  new tree's nodes get fresh ids unless they set their own "_gnx".

  "merge-tree" merges "tree" into "parent"'s children (same shape again),
  matching each entry to an existing child by headline. A match updates
  that child's body (only if "_body" is given — omitting it, unlike
  "insert-tree", leaves the existing body alone) and merges its children
  the same way, recursively. No match inserts that entry fresh, same as
  "insert-tree". "merge-tree" never removes a node; a headline matching
  more than one sibling fails the batch.

EXAMPLE:
  {"gnx-prefix":"acme",
   "operations":[{"op":"set-body","node":"ekr.1","expected":"old","body":"new"}]}"#)]
    Apply {
        file: PathBuf,
        /// Path to the operations JSON file, or "-" to read it from stdin.
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
        Command::New {
            file,
            headline,
            import,
            mode,
            no_recursive,
            no_paths,
            dry_run,
        } => {
            if import.is_empty() {
                LeoDocument::new(headline)
                    .save_new(&file)
                    .with_context(|| format!("create {}", file.display()))?;
                println!("{}", file.display());
            } else {
                let mut document = LeoDocument::empty();
                let report = import_files(
                    &mut document,
                    &file,
                    &import,
                    &ImportOptions {
                        mode: mode.into(),
                        recursive: !no_recursive,
                        paths: !no_paths,
                        parent: None,
                        dry_run,
                    },
                )?;
                if !dry_run {
                    document
                        .save_new(&file)
                        .with_context(|| format!("create {}", file.display()))?;
                }
                println!("{}", serde_json::to_string_pretty(&report)?);
            }
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
        #[cfg(feature = "rhai")]
        Command::Run { script } => rhai_run::run(&script)?,
        Command::Inspect {
            file,
            external,
            gnx,
            position,
            headline,
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
            } else if let Some(headline) = headline.as_deref() {
                select_subtrees(&outline, InspectSelector::Headline(headline))?
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
                    InspectFormat::JsonTree => {
                        bail!("--format json-tree does not support --search")
                    }
                }
            } else {
                match format {
                    InspectFormat::Compact => print!("{}", render_compact(&selected)),
                    InspectFormat::Json => {
                        println!("{}", serde_json::to_string_pretty(&selected)?)
                    }
                    InspectFormat::JsonTree => {
                        let tree = json_tree(&selected)?;
                        println!("{}", serde_json::to_string_pretty(&tree)?);
                    }
                }
            }
        }
        Command::Render {
            file,
            position,
            gnx,
            external,
            headline,
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
            } else if let Some(headline) = headline.as_deref() {
                select_subtrees(&outline, InspectSelector::Headline(headline))?
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
            let source = if operations == Path::new("-") {
                let mut buffer = String::new();
                std::io::stdin()
                    .read_to_string(&mut buffer)
                    .context("read operations from stdin")?;
                buffer
            } else {
                fs::read_to_string(&operations).context("read operations file")?
            };
            let batch: OperationBatch = serde_json::from_str(&source)?;
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
    use std::path::Path;

    use super::*;

    #[test]
    fn accepts_an_outline_without_the_tui_subcommand() {
        let cli = Cli::try_parse_from(["cub", "notes.leo"]).unwrap();

        assert!(cli.command.is_none());
        assert_eq!(cli.file, Some(PathBuf::from("notes.leo")));
        assert!(!cli.no_derived);
    }

    #[test]
    fn apply_accepts_a_dash_as_the_operations_path() {
        let cli = Cli::try_parse_from(["cub", "apply", "notes.leo", "-"]).unwrap();

        assert!(matches!(
            cli.command,
            Some(Command::Apply { operations, .. }) if operations == Path::new("-")
        ));
    }

    #[test]
    fn explicit_subcommands_still_parse_normally() {
        let cli = Cli::try_parse_from(["cub", "validate", "notes.leo"]).unwrap();

        assert!(matches!(
            cli.command,
            Some(Command::Validate { file }) if file == Path::new("notes.leo")
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
