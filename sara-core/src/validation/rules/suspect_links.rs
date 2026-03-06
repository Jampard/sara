//! Suspect link detection via fingerprint stamp comparison.

use crate::config::ValidationConfig;
use crate::error::ValidationError;
use crate::fingerprint::compute_item_fingerprint;
use crate::graph::KnowledgeGraph;
use crate::validation::rule::{Severity, ValidationRule};

/// Detects suspect links by comparing stored stamps against current fingerprints.
pub struct SuspectLinksRule;

impl ValidationRule for SuspectLinksRule {
    fn validate(
        &self,
        graph: &KnowledgeGraph,
        _config: &ValidationConfig,
    ) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        for item in graph.items() {
            // Collect all outgoing relation target IDs
            let target_ids: Vec<_> = item.upstream.all_ids().collect();

            for target_id in &target_ids {
                let Some(target) = graph.get(target_id) else {
                    continue; // Broken ref handled by another rule
                };

                let current_fp = compute_item_fingerprint(target);

                match item.stamps.get(*target_id) {
                    None => {
                        errors.push(ValidationError::SuspectLink {
                            source_id: item.id.to_string(),
                            target_id: target_id.to_string(),
                            reason: "never reviewed (no stamp)".to_string(),
                        });
                    }
                    Some(stamp) if !current_fp.starts_with(stamp.as_str()) => {
                        errors.push(ValidationError::SuspectLink {
                            source_id: item.id.to_string(),
                            target_id: target_id.to_string(),
                            reason: "target changed since last review".to_string(),
                        });
                    }
                    Some(_) => {} // Match — link is clean
                }
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
    use crate::config::ValidationConfig;
    use crate::fingerprint::compute_item_fingerprint;
    use crate::graph::KnowledgeGraphBuilder;
    use crate::model::{ItemId, ItemType, UpstreamRefs};
    use crate::test_utils::{create_test_item, create_test_item_with_upstream};
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_no_suspect_links_when_stamps_match() {
        let entity = create_test_item("ITM-001", ItemType::Entity);
        let entity_fp = compute_item_fingerprint(&entity);

        let source = crate::model::SourceLocation::new(
            PathBuf::from("/test"),
            "EVD-001.md".to_string(),
        );
        let evidence = crate::model::ItemBuilder::new()
            .id(ItemId::new_unchecked("EVD-001"))
            .item_type(ItemType::Evidence)
            .name("Test Evidence")
            .source(source)
            .upstream(UpstreamRefs {
                parent: vec![ItemId::new_unchecked("ITM-001")],
                ..Default::default()
            })
            .stamps(HashMap::from([(
                ItemId::new_unchecked("ITM-001"),
                entity_fp,
            )]))
            .build()
            .unwrap();

        let graph = KnowledgeGraphBuilder::new()
            .add_item(entity)
            .add_item(evidence)
            .build()
            .unwrap();

        let rule = SuspectLinksRule;
        let config = ValidationConfig::default();
        let errors = rule.validate(&graph, &config);
        assert!(errors.is_empty(), "Expected no suspect links");
    }

    #[test]
    fn test_suspect_link_when_stamp_missing() {
        let entity = create_test_item("ITM-001", ItemType::Entity);
        let evidence = create_test_item_with_upstream(
            "EVD-001",
            ItemType::Evidence,
            UpstreamRefs {
                parent: vec![ItemId::new_unchecked("ITM-001")],
                ..Default::default()
            },
        );

        let graph = KnowledgeGraphBuilder::new()
            .add_item(entity)
            .add_item(evidence)
            .build()
            .unwrap();

        let rule = SuspectLinksRule;
        let config = ValidationConfig::default();
        let errors = rule.validate(&graph, &config);
        assert!(!errors.is_empty(), "Expected suspect link warning");
    }

    #[test]
    fn test_suspect_link_when_stamp_stale() {
        let entity = create_test_item("ITM-001", ItemType::Entity);

        let source = crate::model::SourceLocation::new(
            PathBuf::from("/test"),
            "EVD-001.md".to_string(),
        );
        let evidence = crate::model::ItemBuilder::new()
            .id(ItemId::new_unchecked("EVD-001"))
            .item_type(ItemType::Evidence)
            .name("Test Evidence")
            .source(source)
            .upstream(UpstreamRefs {
                parent: vec![ItemId::new_unchecked("ITM-001")],
                ..Default::default()
            })
            .stamps(HashMap::from([(
                ItemId::new_unchecked("ITM-001"),
                "stale_fingerprint".to_string(),
            )]))
            .build()
            .unwrap();

        let graph = KnowledgeGraphBuilder::new()
            .add_item(entity)
            .add_item(evidence)
            .build()
            .unwrap();

        let rule = SuspectLinksRule;
        let config = ValidationConfig::default();
        let errors = rule.validate(&graph, &config);
        assert!(!errors.is_empty(), "Expected stale suspect link");
    }
}
