//! Implementation of the clear command.

use std::error::Error;
use std::fs;
use std::process::ExitCode;

use clap::Args;

use sara_core::fingerprint::review::apply_stamp;
use sara_core::fingerprint::{compute_item_fingerprint, truncate_fingerprint};
use sara_core::graph::KnowledgeGraphBuilder;
use sara_core::model::ItemId;

use super::CommandContext;
use crate::output::{print_error, print_success};

/// Clear a suspect link by re-stamping a single target
#[derive(Args, Debug)]
pub struct ClearArgs {
    /// Source item ID
    pub item_id: String,
    /// Target item ID to re-stamp
    pub target_id: String,
}

/// Runs the clear command.
pub fn run(args: &ClearArgs, ctx: &CommandContext) -> Result<ExitCode, Box<dyn Error>> {
    let items = ctx.parse_items(None)?;

    let graph = KnowledgeGraphBuilder::new()
        .add_items(items)
        .build()?;

    let item_id = ItemId::new_unchecked(&args.item_id);
    let target_id = ItemId::new_unchecked(&args.target_id);

    let Some(item) = graph.get(&item_id) else {
        print_error(&ctx.output, &format!("Item '{}' not found", args.item_id));
        return Ok(ExitCode::FAILURE);
    };

    let Some(target) = graph.get(&target_id) else {
        print_error(
            &ctx.output,
            &format!("Target '{}' not found", args.target_id),
        );
        return Ok(ExitCode::FAILURE);
    };

    let target_fp = compute_item_fingerprint(target);
    let target_fp_short = truncate_fingerprint(&target_fp).to_string();

    // Read file, update stamp for this specific target
    let file_path = item.source.full_path();
    let content = fs::read_to_string(&file_path)?;

    let updated = apply_stamp(&content, &args.target_id, &target_fp_short)
        .map_err(|e| format!("Failed to update frontmatter: {e}"))?;
    fs::write(&file_path, updated)?;

    print_success(
        &ctx.output,
        &format!(
            "Cleared suspect link {} -> {} (stamp: {})",
            args.item_id, args.target_id, target_fp_short
        ),
    );

    Ok(ExitCode::SUCCESS)
}
