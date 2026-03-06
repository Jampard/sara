//! Implementation of the review command.

use std::error::Error;
use std::fs;
use std::process::ExitCode;

use clap::Args;

use sara_core::fingerprint::review::apply_review;
use sara_core::fingerprint::{compute_item_fingerprint, truncate_fingerprint};
use sara_core::graph::KnowledgeGraphBuilder;
use sara_core::model::ItemId;

use super::CommandContext;
use crate::output::{print_error, print_success};

/// Mark an item as reviewed and re-stamp outgoing links
#[derive(Args, Debug)]
pub struct ReviewArgs {
    /// Item ID to mark as reviewed
    pub item_id: String,
}

/// Runs the review command.
pub fn run(args: &ReviewArgs, ctx: &CommandContext) -> Result<ExitCode, Box<dyn Error>> {
    let items = ctx.parse_items(None)?;

    let graph = KnowledgeGraphBuilder::new().add_items(items).build()?;

    let item_id = ItemId::new_unchecked(&args.item_id);
    let Some(item) = graph.get(&item_id) else {
        print_error(&ctx.output, &format!("Item '{}' not found", args.item_id));
        return Ok(ExitCode::FAILURE);
    };

    // Compute own fingerprint
    let own_fp = compute_item_fingerprint(item);
    let own_fp_short = truncate_fingerprint(&own_fp).to_string();

    // Compute stamps for all upstream relation targets
    let target_ids: Vec<_> = item.upstream.all_ids().collect();
    let mut stamps: Vec<(String, String)> = Vec::new();

    for target_id in &target_ids {
        if let Some(target) = graph.get(target_id) {
            let target_fp = compute_item_fingerprint(target);
            stamps.push((
                target_id.to_string(),
                truncate_fingerprint(&target_fp).to_string(),
            ));
        }
    }

    // Read file, update frontmatter with reviewed + stamps
    let file_path = item.source.full_path();
    let content = fs::read_to_string(&file_path)?;

    let updated = apply_review(&content, &own_fp_short, &stamps)
        .map_err(|e| format!("Failed to update frontmatter: {e}"))?;
    fs::write(&file_path, updated)?;

    print_success(
        &ctx.output,
        &format!(
            "Reviewed {} (fingerprint: {}), stamped {} link(s)",
            args.item_id,
            own_fp_short,
            stamps.len()
        ),
    );

    Ok(ExitCode::SUCCESS)
}
