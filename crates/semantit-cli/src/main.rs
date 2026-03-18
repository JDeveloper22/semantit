use std::fs;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use semantit_core::model::{FileSemanticIndex, MergeResult, SemanticChangeKind, SemanticDiff};
use semantit_core::{diff_indices, merge_indices, ParseInput, ParserRegistry};
use semantit_ts::TypeScriptParser;

#[derive(Debug, Parser)]
#[command(
    name = "semantit",
    version,
    about = "Semantic diff and merge for source code"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Parse(FileCommand),
    Diff(CompareCommand),
    Merge(MergeCommand),
}

#[derive(Debug, Args)]
struct FileCommand {
    file: PathBuf,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct CompareCommand {
    old: PathBuf,
    new: PathBuf,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct MergeCommand {
    base: PathBuf,
    ours: PathBuf,
    theirs: PathBuf,
    #[arg(long)]
    json: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let registry = build_registry();

    match cli.command {
        Commands::Parse(command) => {
            let index = parse_path(&registry, &command.file)?;
            if command.json {
                print_json(&index)?;
            } else {
                print_parse_human(&index);
            }
        }
        Commands::Diff(command) => {
            let old_index = parse_path(&registry, &command.old)?;
            let new_index = parse_path(&registry, &command.new)?;
            let diff = diff_indices(&old_index, &new_index);
            if command.json {
                print_json(&diff)?;
            } else {
                print_diff_human(&diff);
            }
        }
        Commands::Merge(command) => {
            let base_index = parse_path(&registry, &command.base)?;
            let ours_index = parse_path(&registry, &command.ours)?;
            let theirs_index = parse_path(&registry, &command.theirs)?;
            let merge = merge_indices(&base_index, &ours_index, &theirs_index);
            if command.json {
                print_json(&merge)?;
            } else {
                print_merge_human(&merge);
            }
        }
    }

    Ok(())
}

fn build_registry() -> ParserRegistry {
    let mut registry = ParserRegistry::default();
    registry.register(TypeScriptParser);
    registry
}

fn parse_path(
    registry: &ParserRegistry,
    path: &PathBuf,
) -> Result<FileSemanticIndex, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(path)?;
    registry
        .parse(ParseInput {
            path: path.as_path(),
            source: &source,
        })
        .map_err(Into::into)
}

fn print_json<T: serde::Serialize>(value: &T) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn print_parse_human(index: &FileSemanticIndex) {
    println!(
        "Parsed {} entities from {} ({})",
        index.entities.len(),
        index.path,
        index.language
    );
    for entity in &index.entities {
        println!(
            "- {:<10} {:<30} id={} body={} sig={}",
            entity.kind.as_str(),
            entity.path,
            short_hash(&entity.id),
            short_hash(&entity.body_hash),
            short_hash(&entity.signature_hash),
        );
    }
    if !index.exports.is_empty() {
        println!("Exports:");
        for export in &index.exports {
            println!(
                "- {} -> {}{}",
                export.exported_name,
                export.local_name,
                export
                    .entity_id
                    .as_deref()
                    .map(|value| format!(" ({})", short_hash(value)))
                    .unwrap_or_default()
            );
        }
    }
}

fn print_diff_human(diff: &SemanticDiff) {
    println!("Diff {} -> {}", diff.old_path, diff.new_path);
    println!(
        "Summary: unchanged={} moved={} renamed={} signature={} implementation={} inserted={} deleted={} split={} merged={}",
        diff.summary.unchanged,
        diff.summary.moved,
        diff.summary.renamed,
        diff.summary.signature_changed,
        diff.summary.implementation_changed,
        diff.summary.inserted,
        diff.summary.deleted,
        diff.summary.split,
        diff.summary.merged,
    );

    for change in &diff.changes {
        let kinds = change
            .kinds
            .iter()
            .map(change_kind_label)
            .collect::<Vec<_>>()
            .join(", ");
        let label = change
            .current
            .as_ref()
            .or(change.previous.as_ref())
            .map(|entity| entity.path.as_str())
            .unwrap_or("<unknown>");
        println!("- {} [{}]", label, kinds);
        println!("  {}", change.explanation);
    }
}

fn print_merge_human(merge: &MergeResult) {
    println!("Merge status: {:?}", merge.status);
    println!(
        "Resolved changes: {}, conflicts: {}",
        merge.resolved_changes.len(),
        merge.conflicts.len()
    );

    for change in &merge.resolved_changes {
        let label = change
            .current
            .as_ref()
            .or(change.previous.as_ref())
            .map(|entity| entity.path.as_str())
            .unwrap_or("<unknown>");
        println!(
            "- resolved {} [{}]",
            label,
            change
                .kinds
                .iter()
                .map(change_kind_label)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    for conflict in &merge.conflicts {
        let label = conflict
            .ours
            .as_ref()
            .or(conflict.theirs.as_ref())
            .or(conflict.base.as_ref())
            .map(|entity| entity.path.as_str())
            .unwrap_or("<unknown>");
        println!("- conflict {} [{:?}]", label, conflict.cause);
        println!("  {}", conflict.message);
    }
}

fn short_hash(value: &str) -> &str {
    value.get(..8).unwrap_or(value)
}

fn change_kind_label(kind: &SemanticChangeKind) -> &'static str {
    match kind {
        SemanticChangeKind::Unchanged => "unchanged",
        SemanticChangeKind::Moved => "moved",
        SemanticChangeKind::SignatureChanged => "signature_changed",
        SemanticChangeKind::ImplementationChanged => "implementation_changed",
        SemanticChangeKind::Renamed => "renamed",
        SemanticChangeKind::Inserted => "inserted",
        SemanticChangeKind::Deleted => "deleted",
        SemanticChangeKind::Split => "split",
        SemanticChangeKind::Merged => "merged",
    }
}
