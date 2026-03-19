//! Envelope validation rule for evidence items.

use std::collections::HashSet;

use crate::config::ValidationConfig;
use crate::error::ValidationError;
use crate::graph::KnowledgeGraph;
use crate::model::ItemType;
use crate::validation::rule::{Severity, ValidationRule};

/// Validates envelope data on evidence items.
///
/// Checks:
/// 1. All entity UIDs in envelopes must appear in participants
/// 2. At most one envelope type per evidence item (mutual exclusivity)
/// 3. Envelope IDs are unique within their array
pub struct EnvelopeRule;

impl ValidationRule for EnvelopeRule {
    fn validate(&self, graph: &KnowledgeGraph, _config: &ValidationConfig) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        for item in graph.items().filter(|i| i.item_type == ItemType::Evidence) {
            let file = item.source.file_path.display().to_string();

            // Collect participant entity IDs for cross-validation
            let participant_entities: HashSet<&str> = item
                .participants
                .iter()
                .map(|p| p.entity.as_str())
                .collect();

            // 1. Mutual exclusivity: at most one envelope type
            let mut envelope_count = 0;
            if !item.attributes.messages().is_empty() {
                envelope_count += 1;
            }
            if item.attributes.deposition().is_some() {
                envelope_count += 1;
            }
            if !item.attributes.flights().is_empty() {
                envelope_count += 1;
            }
            if !item.attributes.transactions().is_empty() {
                envelope_count += 1;
            }
            if envelope_count > 1 {
                errors.push(ValidationError::InvalidMetadata {
                    file: file.clone(),
                    reason: format!(
                        "Evidence {} has {} envelope types; at most one is allowed",
                        item.id, envelope_count
                    ),
                });
            }

            // Skip remaining checks if no envelopes
            if envelope_count == 0 {
                continue;
            }

            // 2. Participant cross-validation: every envelope entity must be in participants
            let envelope_ids = item.attributes.envelope_entity_ids();
            for entity_id in &envelope_ids {
                if !participant_entities.contains(entity_id.as_str()) {
                    errors.push(ValidationError::InvalidMetadata {
                        file: file.clone(),
                        reason: format!(
                            "Envelope entity '{}' in {} not listed in participants",
                            entity_id, item.id
                        ),
                    });
                }
            }

            // 3. Unique envelope IDs within each array
            let mut seen_ids = HashSet::new();
            for msg in item.attributes.messages() {
                if !seen_ids.insert(("message", msg.id)) {
                    errors.push(ValidationError::InvalidMetadata {
                        file: file.clone(),
                        reason: format!("Duplicate message id {} in {}", msg.id, item.id),
                    });
                }
            }

            seen_ids.clear();
            if let Some(depo) = item.attributes.deposition() {
                for ex in &depo.exchanges {
                    if !seen_ids.insert(("exchange", ex.id)) {
                        errors.push(ValidationError::InvalidMetadata {
                            file: file.clone(),
                            reason: format!(
                                "Duplicate deposition exchange id {} in {}",
                                ex.id, item.id
                            ),
                        });
                    }
                }
            }

            seen_ids.clear();
            for flight in item.attributes.flights() {
                if !seen_ids.insert(("flight", flight.id)) {
                    errors.push(ValidationError::InvalidMetadata {
                        file: file.clone(),
                        reason: format!("Duplicate flight id {} in {}", flight.id, item.id),
                    });
                }
            }

            seen_ids.clear();
            for txn in item.attributes.transactions() {
                if !seen_ids.insert(("transaction", txn.id)) {
                    errors.push(ValidationError::InvalidMetadata {
                        file: file.clone(),
                        reason: format!("Duplicate transaction id {} in {}", txn.id, item.id),
                    });
                }
            }
        }

        errors
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::KnowledgeGraphBuilder;
    use crate::model::{
        EnvelopeFlight, EnvelopeMessage, ItemBuilder, ItemId, Participant, SourceLocation,
        UpstreamRefs,
    };
    use std::path::PathBuf;

    fn make_evidence_with_messages(
        id: &str,
        participants: &[&str],
        messages: Vec<EnvelopeMessage>,
    ) -> crate::model::Item {
        let source = SourceLocation::new(PathBuf::from("/test"), format!("{id}.md"));
        let parts = participants
            .iter()
            .map(|e| Participant {
                entity: ItemId::new_unchecked(*e),
                role: "subject".to_string(),
            })
            .collect();
        ItemBuilder::new()
            .id(ItemId::new_unchecked(id))
            .item_type(ItemType::Evidence)
            .name(format!("Test {id}"))
            .source(source)
            .upstream(UpstreamRefs {
                parent: vec![ItemId::new_unchecked("ITM-alice")],
                ..Default::default()
            })
            .participants(parts)
            .envelope_messages(messages)
            .build()
            .unwrap()
    }

    #[test]
    fn test_envelope_participant_cross_validation() {
        // Message references ITM-bob but ITM-bob is not in participants
        let msg = EnvelopeMessage {
            id: 1,
            from: ItemId::new_unchecked("ITM-alice"),
            to: vec![ItemId::new_unchecked("ITM-bob")],
            date: None,
            subject: None,
            cc: None,
            bcc: None,
            forward: None,
            removed: None,
        };
        let evd = make_evidence_with_messages("EVD-001", &["ITM-alice"], vec![msg]);
        let entity_a = crate::test_utils::create_test_item("ITM-alice", ItemType::Entity);
        let entity_b = crate::test_utils::create_test_item("ITM-bob", ItemType::Entity);

        let graph = KnowledgeGraphBuilder::new()
            .add_item(entity_a)
            .add_item(entity_b)
            .add_item(evd)
            .build()
            .unwrap();

        let errors = EnvelopeRule.validate(&graph, &ValidationConfig::default());
        assert!(
            errors.iter().any(|e| matches!(
                e,
                ValidationError::InvalidMetadata { reason, .. }
                    if reason.contains("ITM-bob") && reason.contains("not listed in participants")
            )),
            "Should detect missing participant: {errors:?}"
        );
    }

    #[test]
    fn test_envelope_valid_participants() {
        let msg = EnvelopeMessage {
            id: 1,
            from: ItemId::new_unchecked("ITM-alice"),
            to: vec![ItemId::new_unchecked("ITM-bob")],
            date: None,
            subject: None,
            cc: None,
            bcc: None,
            forward: None,
            removed: None,
        };
        let evd = make_evidence_with_messages("EVD-001", &["ITM-alice", "ITM-bob"], vec![msg]);
        let entity_a = crate::test_utils::create_test_item("ITM-alice", ItemType::Entity);
        let entity_b = crate::test_utils::create_test_item("ITM-bob", ItemType::Entity);

        let graph = KnowledgeGraphBuilder::new()
            .add_item(entity_a)
            .add_item(entity_b)
            .add_item(evd)
            .build()
            .unwrap();

        let errors = EnvelopeRule.validate(&graph, &ValidationConfig::default());
        assert!(errors.is_empty(), "No errors expected: {errors:?}");
    }

    #[test]
    fn test_envelope_mutual_exclusivity() {
        // Evidence with both messages AND flights (should fail)
        let source = SourceLocation::new(PathBuf::from("/test"), "EVD-bad.md".to_string());
        let msg = EnvelopeMessage {
            id: 1,
            from: ItemId::new_unchecked("ITM-alice"),
            to: vec![ItemId::new_unchecked("ITM-bob")],
            date: None,
            subject: None,
            cc: None,
            bcc: None,
            forward: None,
            removed: None,
        };
        let flight = EnvelopeFlight {
            id: 1,
            date: "2024-01-01".to_string(),
            origin: ItemId::new_unchecked("ITM-loc-a"),
            destination: ItemId::new_unchecked("ITM-loc-b"),
            aircraft: None,
            passengers: vec![
                ItemId::new_unchecked("ITM-alice"),
                ItemId::new_unchecked("ITM-bob"),
            ],
        };
        let evd = ItemBuilder::new()
            .id(ItemId::new_unchecked("EVD-bad"))
            .item_type(ItemType::Evidence)
            .name("Bad Evidence")
            .source(source)
            .upstream(UpstreamRefs {
                parent: vec![ItemId::new_unchecked("ITM-alice")],
                ..Default::default()
            })
            .participants(vec![
                Participant {
                    entity: ItemId::new_unchecked("ITM-alice"),
                    role: "subject".to_string(),
                },
                Participant {
                    entity: ItemId::new_unchecked("ITM-bob"),
                    role: "subject".to_string(),
                },
                Participant {
                    entity: ItemId::new_unchecked("ITM-loc-a"),
                    role: "location".to_string(),
                },
                Participant {
                    entity: ItemId::new_unchecked("ITM-loc-b"),
                    role: "location".to_string(),
                },
            ])
            .envelope_messages(vec![msg])
            .envelope_flights(vec![flight])
            .build()
            .unwrap();

        let entity_a = crate::test_utils::create_test_item("ITM-alice", ItemType::Entity);
        let entity_b = crate::test_utils::create_test_item("ITM-bob", ItemType::Entity);
        let loc_a = crate::test_utils::create_test_item("ITM-loc-a", ItemType::Entity);
        let loc_b = crate::test_utils::create_test_item("ITM-loc-b", ItemType::Entity);

        let graph = KnowledgeGraphBuilder::new()
            .add_item(entity_a)
            .add_item(entity_b)
            .add_item(loc_a)
            .add_item(loc_b)
            .add_item(evd)
            .build()
            .unwrap();

        let errors = EnvelopeRule.validate(&graph, &ValidationConfig::default());
        assert!(
            errors.iter().any(|e| matches!(
                e,
                ValidationError::InvalidMetadata { reason, .. }
                    if reason.contains("2 envelope types")
            )),
            "Should detect multiple envelope types: {errors:?}"
        );
    }

    #[test]
    fn test_envelope_duplicate_ids() {
        let msg1 = EnvelopeMessage {
            id: 1,
            from: ItemId::new_unchecked("ITM-alice"),
            to: vec![ItemId::new_unchecked("ITM-bob")],
            date: None,
            subject: None,
            cc: None,
            bcc: None,
            forward: None,
            removed: None,
        };
        let msg2 = EnvelopeMessage {
            id: 1, // duplicate
            from: ItemId::new_unchecked("ITM-bob"),
            to: vec![ItemId::new_unchecked("ITM-alice")],
            date: None,
            subject: None,
            cc: None,
            bcc: None,
            forward: None,
            removed: None,
        };
        let evd =
            make_evidence_with_messages("EVD-dup", &["ITM-alice", "ITM-bob"], vec![msg1, msg2]);
        let entity_a = crate::test_utils::create_test_item("ITM-alice", ItemType::Entity);
        let entity_b = crate::test_utils::create_test_item("ITM-bob", ItemType::Entity);

        let graph = KnowledgeGraphBuilder::new()
            .add_item(entity_a)
            .add_item(entity_b)
            .add_item(evd)
            .build()
            .unwrap();

        let errors = EnvelopeRule.validate(&graph, &ValidationConfig::default());
        assert!(
            errors.iter().any(|e| matches!(
                e,
                ValidationError::InvalidMetadata { reason, .. }
                    if reason.contains("Duplicate message id")
            )),
            "Should detect duplicate message IDs: {errors:?}"
        );
    }
}
