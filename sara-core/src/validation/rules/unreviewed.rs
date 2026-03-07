//! Unreviewed item detection validation rule.

use crate::config::ValidationConfig;
use crate::error::ValidationError;
use crate::fingerprint::compute_item_fingerprint;
use crate::graph::KnowledgeGraph;
use crate::validation::rule::{Severity, ValidationRule};

/// Detects items whose content has changed since last review.
///
/// An item is "unreviewed" when its `reviewed` fingerprint is `None`
/// (never reviewed) or does not match the prefix of its current
/// computed fingerprint (content changed since last review).
pub struct UnreviewedItemsRule;

impl ValidationRule for UnreviewedItemsRule {
    fn validate(&self, graph: &KnowledgeGraph, _config: &ValidationConfig) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        for item in graph.items() {
            let current_fp = compute_item_fingerprint(item);
            match &item.reviewed {
                None => {
                    errors.push(ValidationError::UnreviewedItem {
                        item_id: item.id.to_string(),
                        reason: "never reviewed".to_string(),
                    });
                }
                Some(reviewed) if !current_fp.starts_with(reviewed.as_str()) => {
                    errors.push(ValidationError::UnreviewedItem {
                        item_id: item.id.to_string(),
                        reason: "content changed since last review".to_string(),
                    });
                }
                Some(_) => {}
            }
        }
        errors
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fingerprint::truncate_fingerprint;
    use crate::graph::KnowledgeGraphBuilder;
    use crate::model::ItemType;
    use crate::test_utils::create_test_item;

    fn create_item_with_reviewed(id: &str, reviewed: Option<String>) -> crate::model::Item {
        let mut item = create_test_item(id, ItemType::Entity);
        item.reviewed = reviewed;
        item
    }

    #[test]
    fn test_never_reviewed() {
        let graph = KnowledgeGraphBuilder::new()
            .add_item(create_item_with_reviewed("ITM-001", None))
            .build()
            .unwrap();

        let rule = UnreviewedItemsRule;
        let errors = rule.validate(&graph, &ValidationConfig::default());
        assert_eq!(errors.len(), 1);
        if let ValidationError::UnreviewedItem { reason, .. } = &errors[0] {
            assert!(reason.contains("never reviewed"));
        } else {
            panic!("Expected UnreviewedItem error");
        }
    }

    #[test]
    fn test_reviewed_matches() {
        let item = create_test_item("ITM-001", ItemType::Entity);
        let fp = compute_item_fingerprint(&item);
        let short = truncate_fingerprint(&fp).to_string();

        let graph = KnowledgeGraphBuilder::new()
            .add_item(create_item_with_reviewed("ITM-001", Some(short)))
            .build()
            .unwrap();

        let rule = UnreviewedItemsRule;
        let errors = rule.validate(&graph, &ValidationConfig::default());
        assert!(errors.is_empty(), "Reviewed item should not be flagged");
    }

    #[test]
    fn test_content_changed() {
        let graph = KnowledgeGraphBuilder::new()
            .add_item(create_item_with_reviewed(
                "ITM-001",
                Some("deadbeef".to_string()),
            ))
            .build()
            .unwrap();

        let rule = UnreviewedItemsRule;
        let errors = rule.validate(&graph, &ValidationConfig::default());
        assert_eq!(errors.len(), 1);
        if let ValidationError::UnreviewedItem { reason, .. } = &errors[0] {
            assert!(reason.contains("content changed"));
        } else {
            panic!("Expected UnreviewedItem error");
        }
    }

    #[test]
    fn test_full_fingerprint_matches() {
        let item = create_test_item("ITM-001", ItemType::Entity);
        let fp = compute_item_fingerprint(&item);

        let graph = KnowledgeGraphBuilder::new()
            .add_item(create_item_with_reviewed("ITM-001", Some(fp)))
            .build()
            .unwrap();

        let rule = UnreviewedItemsRule;
        let errors = rule.validate(&graph, &ValidationConfig::default());
        assert!(
            errors.is_empty(),
            "Full fingerprint match should not be flagged"
        );
    }
}
