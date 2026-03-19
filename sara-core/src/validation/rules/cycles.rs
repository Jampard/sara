//! Circular reference detection validation rule.

use petgraph::algo::tarjan_scc;
use petgraph::visit::EdgeFiltered;

use crate::config::ValidationConfig;
use crate::error::ValidationError;
use crate::graph::KnowledgeGraph;
use crate::validation::rule::ValidationRule;

/// Circular reference detection rule.
///
/// Uses Tarjan's strongly connected components algorithm to find cycles.
/// Only considers primary relationships (not inverse edges) when detecting
/// cycles, since inverse edges are just for graph traversal and don't
/// represent logical cycles.
pub struct CyclesRule;

impl ValidationRule for CyclesRule {
    fn validate(&self, graph: &KnowledgeGraph, _config: &ValidationConfig) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let inner = graph.inner();

        // Filter the graph to only include primary hierarchical relationships for cycle detection.
        // Inverse relationships (IsRefinedBy, Derives, IsSatisfiedBy, etc.) are excluded
        // because they're just for traversal and would cause false positives.
        // Peer relationships (CommunicatedWith, TraveledWith, PaidTo, etc.) are excluded
        // because bidirectional entity-entity edges are not hierarchical cycles.
        let filtered = EdgeFiltered::from_fn(inner, |edge| {
            edge.weight().is_primary() && !edge.weight().is_peer()
        });

        // Find strongly connected components on the filtered graph
        let sccs = tarjan_scc(&filtered);

        for scc in sccs {
            if scc.len() >= 2 {
                // SCC with 2+ nodes indicates a cycle
                let cycle_ids: Vec<String> = scc
                    .iter()
                    .filter_map(|idx| inner.node_weight(*idx))
                    .map(|item| item.id.as_str().to_string())
                    .collect();

                let cycle_str = cycle_ids.join(" -> ");

                errors.push(ValidationError::CircularReference { cycle: cycle_str });
            } else if scc.len() == 1 {
                // Check for self-loop (only with primary relationships)
                let idx = scc[0];
                let has_self_loop = inner
                    .edges_connecting(idx, idx)
                    .any(|e| e.weight().is_primary() && !e.weight().is_peer());

                if has_self_loop && let Some(item) = inner.node_weight(idx) {
                    errors.push(ValidationError::CircularReference {
                        cycle: format!("{} -> {}", item.id.as_str(), item.id.as_str()),
                    });
                }
            }
        }

        errors
    }
}

/// Checks if adding an edge would create a cycle.
#[cfg(test)]
fn would_create_cycle(
    graph: &KnowledgeGraph,
    from: &crate::model::ItemId,
    to: &crate::model::ItemId,
) -> bool {
    // If to can reach from, adding from->to would create a cycle
    // This is a simple reachability check
    let inner = graph.inner();

    let from_idx = match graph.node_index(from) {
        Some(idx) => idx,
        None => return false,
    };

    let to_idx = match graph.node_index(to) {
        Some(idx) => idx,
        None => return false,
    };

    // Check if there's a path from 'to' back to 'from'
    petgraph::algo::has_path_connecting(inner, to_idx, from_idx, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::KnowledgeGraphBuilder;
    use crate::model::{ItemBuilder, ItemId, ItemType, UpstreamRefs};
    use crate::test_utils::{create_test_item, create_test_item_with_upstream};

    #[test]
    fn test_no_cycles() {
        let graph = KnowledgeGraphBuilder::new()
            .add_item(create_test_item("SOL-001", ItemType::Solution))
            .add_item(create_test_item_with_upstream(
                "UC-001",
                ItemType::UseCase,
                UpstreamRefs {
                    refines: vec![ItemId::new_unchecked("SOL-001")],
                    ..Default::default()
                },
            ))
            .build()
            .unwrap();

        let rule = CyclesRule;
        let errors = rule.validate(&graph, &ValidationConfig::default());
        assert!(errors.is_empty());
    }

    #[test]
    fn test_cycle_detected() {
        // Create a cycle: SCEN-001 -> SCEN-002 -> SCEN-001
        let scen1 = create_test_item_with_upstream(
            "SCEN-001",
            ItemType::Scenario,
            UpstreamRefs {
                refines: vec![ItemId::new_unchecked("SCEN-002")],
                ..Default::default()
            },
        );
        let scen2 = create_test_item_with_upstream(
            "SCEN-002",
            ItemType::Scenario,
            UpstreamRefs {
                refines: vec![ItemId::new_unchecked("SCEN-001")],
                ..Default::default()
            },
        );

        let graph = KnowledgeGraphBuilder::new()
            .add_item(scen1)
            .add_item(scen2)
            .build()
            .unwrap();

        let rule = CyclesRule;
        let errors = rule.validate(&graph, &ValidationConfig::default());
        assert!(!errors.is_empty(), "Cycle should be detected");
    }

    #[test]
    fn test_would_create_cycle() {
        let sol = create_test_item("SOL-001", ItemType::Solution);
        let uc = create_test_item_with_upstream(
            "UC-001",
            ItemType::UseCase,
            UpstreamRefs {
                refines: vec![ItemId::new_unchecked("SOL-001")],
                ..Default::default()
            },
        );

        let graph = KnowledgeGraphBuilder::new()
            .add_item(sol)
            .add_item(uc)
            .build()
            .unwrap();

        // Adding SOL-001 -> UC-001 would create a cycle
        assert!(would_create_cycle(
            &graph,
            &ItemId::new_unchecked("SOL-001"),
            &ItemId::new_unchecked("UC-001"),
        ));

        // Adding UC-001 -> new item would not create a cycle
        assert!(!would_create_cycle(
            &graph,
            &ItemId::new_unchecked("UC-001"),
            &ItemId::new_unchecked("SCEN-001"),
        ));
    }

    #[test]
    fn test_peer_edges_do_not_cause_false_cycle() {
        use crate::model::{EnvelopeMessage, Participant};

        // Build two entities connected by bidirectional CommunicatedWith edges
        // via a messages envelope. This should NOT be detected as a cycle.
        let entity_a = create_test_item("ITM-alice", ItemType::Entity);
        let entity_b = create_test_item("ITM-bob", ItemType::Entity);

        let evidence = ItemBuilder::new()
            .id(ItemId::new_unchecked("EVD-001"))
            .item_type(ItemType::Evidence)
            .name("Email Thread")
            .source(crate::model::SourceLocation::new(
                std::path::PathBuf::from("/test"),
                "EVD-001.md",
            ))
            .participants(vec![
                Participant {
                    entity: ItemId::new_unchecked("ITM-alice"),
                    role: "sender".into(),
                },
                Participant {
                    entity: ItemId::new_unchecked("ITM-bob"),
                    role: "recipient".into(),
                },
            ])
            .envelope_messages(vec![
                EnvelopeMessage {
                    id: 1,
                    from: ItemId::new_unchecked("ITM-alice"),
                    to: vec![ItemId::new_unchecked("ITM-bob")],
                    date: None,
                    subject: None,
                    cc: None,
                    bcc: None,
                    forward: None,
                    removed: None,
                },
                EnvelopeMessage {
                    id: 2,
                    from: ItemId::new_unchecked("ITM-bob"),
                    to: vec![ItemId::new_unchecked("ITM-alice")],
                    date: None,
                    subject: None,
                    cc: None,
                    bcc: None,
                    forward: None,
                    removed: None,
                },
            ])
            .build()
            .unwrap();

        let graph = KnowledgeGraphBuilder::new()
            .add_item(entity_a)
            .add_item(entity_b)
            .add_item(evidence)
            .build()
            .unwrap();

        let rule = CyclesRule;
        let errors = rule.validate(&graph, &ValidationConfig::default());
        assert!(
            errors.is_empty(),
            "Bidirectional peer edges should not be detected as cycles, got: {errors:?}"
        );
    }

    #[test]
    fn test_real_cycle_still_detected_with_peer_edges() {
        use crate::model::{EnvelopeMessage, Participant};

        // Create a real hierarchical cycle AND peer edges.
        // The cycle should still be detected.
        let scen1 = create_test_item_with_upstream(
            "SCEN-001",
            ItemType::Scenario,
            UpstreamRefs {
                refines: vec![ItemId::new_unchecked("SCEN-002")],
                ..Default::default()
            },
        );
        let scen2 = create_test_item_with_upstream(
            "SCEN-002",
            ItemType::Scenario,
            UpstreamRefs {
                refines: vec![ItemId::new_unchecked("SCEN-001")],
                ..Default::default()
            },
        );

        // Also add entities with peer edges (should be ignored for cycle detection)
        let entity_a = create_test_item("ITM-a", ItemType::Entity);
        let entity_b = create_test_item("ITM-b", ItemType::Entity);
        let evidence = ItemBuilder::new()
            .id(ItemId::new_unchecked("EVD-001"))
            .item_type(ItemType::Evidence)
            .name("Emails")
            .source(crate::model::SourceLocation::new(
                std::path::PathBuf::from("/test"),
                "EVD-001.md",
            ))
            .participants(vec![
                Participant {
                    entity: ItemId::new_unchecked("ITM-a"),
                    role: "sender".into(),
                },
                Participant {
                    entity: ItemId::new_unchecked("ITM-b"),
                    role: "recipient".into(),
                },
            ])
            .envelope_messages(vec![EnvelopeMessage {
                id: 1,
                from: ItemId::new_unchecked("ITM-a"),
                to: vec![ItemId::new_unchecked("ITM-b")],
                date: None,
                subject: None,
                cc: None,
                bcc: None,
                forward: None,
                removed: None,
            }])
            .build()
            .unwrap();

        let graph = KnowledgeGraphBuilder::new()
            .add_item(scen1)
            .add_item(scen2)
            .add_item(entity_a)
            .add_item(entity_b)
            .add_item(evidence)
            .build()
            .unwrap();

        let rule = CyclesRule;
        let errors = rule.validate(&graph, &ValidationConfig::default());
        assert!(
            !errors.is_empty(),
            "Real hierarchical cycle should still be detected despite peer edges"
        );
    }
}
