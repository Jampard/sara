//! Redundant relationship detection validation rule.

use std::collections::HashSet;

use crate::config::ValidationConfig;
use crate::error::ValidationError;
use crate::graph::KnowledgeGraph;
use crate::model::{Item, ItemId, RelationshipType};
use crate::validation::rule::{Severity, ValidationRule};

/// Redundant relationship detection rule (warning).
///
/// Detects redundant relationships where both items declare the same link.
/// For example, if SARCH-001 has `satisfies: [SYSREQ-00001]` and SYSREQ-00001
/// has `is_satisfied_by: [SARCH-001]`, this is redundant - only one declaration
/// is needed since the inverse is automatically inferred.
pub struct RedundantRelationshipsRule;

impl ValidationRule for RedundantRelationshipsRule {
    fn validate(&self, graph: &KnowledgeGraph, _config: &ValidationConfig) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let mut seen_pairs: HashSet<(String, String)> = HashSet::new();

        for item in graph.items() {
            // Check downstream declarations against upstream declarations in target items

            // is_refined_by <-> refines
            check_redundant_pair(
                item,
                graph,
                &item.downstream.is_refined_by,
                |target| target.upstream.refines.contains(&item.id),
                &RelationshipPair {
                    from_rel: RelationshipType::IsRefinedBy,
                    to_rel: RelationshipType::Refines,
                },
                &mut seen_pairs,
                &mut errors,
            );

            // derives <-> derives_from
            check_redundant_pair(
                item,
                graph,
                &item.downstream.derives,
                |target| target.upstream.derives_from.contains(&item.id),
                &RelationshipPair {
                    from_rel: RelationshipType::Derives,
                    to_rel: RelationshipType::DerivesFrom,
                },
                &mut seen_pairs,
                &mut errors,
            );

            // is_satisfied_by <-> satisfies
            check_redundant_pair(
                item,
                graph,
                &item.downstream.is_satisfied_by,
                |target| target.upstream.satisfies.contains(&item.id),
                &RelationshipPair {
                    from_rel: RelationshipType::IsSatisfiedBy,
                    to_rel: RelationshipType::Satisfies,
                },
                &mut seen_pairs,
                &mut errors,
            );

            // Investigation: children <-> parent
            check_redundant_pair(
                item,
                graph,
                &item.downstream.children,
                |target| target.upstream.parent.contains(&item.id),
                &RelationshipPair {
                    from_rel: RelationshipType::Children,
                    to_rel: RelationshipType::Parent,
                },
                &mut seen_pairs,
                &mut errors,
            );

            // Investigation: cited_by <-> cites
            check_redundant_pair(
                item,
                graph,
                &item.downstream.cited_by,
                |target| target.upstream.cites.contains(&item.id),
                &RelationshipPair {
                    from_rel: RelationshipType::CitedBy,
                    to_rel: RelationshipType::Cites,
                },
                &mut seen_pairs,
                &mut errors,
            );

            // Investigation: evaluated_by <-> evaluates
            check_redundant_pair(
                item,
                graph,
                &item.downstream.evaluated_by,
                |target| target.upstream.evaluates.contains(&item.id),
                &RelationshipPair {
                    from_rel: RelationshipType::EvaluatedBy,
                    to_rel: RelationshipType::Evaluates,
                },
                &mut seen_pairs,
                &mut errors,
            );

            // Investigation: establishes <-> established_by
            check_redundant_pair(
                item,
                graph,
                &item.downstream.establishes,
                |target| target.upstream.established_by.contains(&item.id),
                &RelationshipPair {
                    from_rel: RelationshipType::Establishes,
                    to_rel: RelationshipType::EstablishedBy,
                },
                &mut seen_pairs,
                &mut errors,
            );

            // Investigation: raises <-> raised_by
            check_redundant_pair(
                item,
                graph,
                &item.downstream.raises,
                |target| target.upstream.raised_by.contains(&item.id),
                &RelationshipPair {
                    from_rel: RelationshipType::Raises,
                    to_rel: RelationshipType::RaisedBy,
                },
                &mut seen_pairs,
                &mut errors,
            );

            // Investigation: premises <-> premise_of
            check_redundant_pair(
                item,
                graph,
                &item.downstream.premises,
                |target| target.upstream.premise_of.contains(&item.id),
                &RelationshipPair {
                    from_rel: RelationshipType::InvestigationPremises,
                    to_rel: RelationshipType::PremiseOf,
                },
                &mut seen_pairs,
                &mut errors,
            );

            // Investigation: gaps <-> gap_of
            check_redundant_pair(
                item,
                graph,
                &item.downstream.gaps,
                |target| target.upstream.gap_of.contains(&item.id),
                &RelationshipPair {
                    from_rel: RelationshipType::InvestigationGaps,
                    to_rel: RelationshipType::GapOf,
                },
                &mut seen_pairs,
                &mut errors,
            );

            // Investigation: hypotheses <-> hypothesis_of
            check_redundant_pair(
                item,
                graph,
                &item.downstream.hypotheses,
                |target| target.upstream.hypothesis_of.contains(&item.id),
                &RelationshipPair {
                    from_rel: RelationshipType::InvestigationHypotheses,
                    to_rel: RelationshipType::HypothesisOf,
                },
                &mut seen_pairs,
                &mut errors,
            );

            // Investigation: analyses <-> analysis_of
            check_redundant_pair(
                item,
                graph,
                &item.downstream.analyses,
                |target| target.upstream.analysis_of.contains(&item.id),
                &RelationshipPair {
                    from_rel: RelationshipType::InvestigationAnalyses,
                    to_rel: RelationshipType::AnalysisOf,
                },
                &mut seen_pairs,
                &mut errors,
            );

            // Investigation: affected_by <-> affects
            check_redundant_pair(
                item,
                graph,
                &item.downstream.affected_by,
                |target| target.upstream.affects.contains(&item.id),
                &RelationshipPair {
                    from_rel: RelationshipType::AffectedBy,
                    to_rel: RelationshipType::Affects,
                },
                &mut seen_pairs,
                &mut errors,
            );
        }

        errors
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }
}

/// Relationship pair configuration for redundancy checking.
struct RelationshipPair {
    from_rel: RelationshipType,
    to_rel: RelationshipType,
}

/// Checks for redundant declarations in a specific relationship pair.
fn check_redundant_pair<F>(
    item: &Item,
    graph: &KnowledgeGraph,
    downstream_refs: &[ItemId],
    has_inverse: F,
    pair: &RelationshipPair,
    seen_pairs: &mut HashSet<(String, String)>,
    errors: &mut Vec<ValidationError>,
) where
    F: Fn(&Item) -> bool,
{
    for target_id in downstream_refs {
        if let Some(target) = graph.get(target_id)
            && has_inverse(target)
        {
            let pair_key = make_pair_key(&item.id, target_id);
            if seen_pairs.insert(pair_key) {
                errors.push(ValidationError::RedundantRelationship {
                    from_id: item.id.clone(),
                    to_id: target_id.clone(),
                    from_rel: pair.from_rel,
                    to_rel: pair.to_rel,
                });
            }
        }
    }
}

/// Creates a canonical pair key for deduplication (smaller ID first).
fn make_pair_key(id1: &ItemId, id2: &ItemId) -> (String, String) {
    let s1 = id1.as_str();
    let s2 = id2.as_str();
    if s1 < s2 {
        (s1.to_string(), s2.to_string())
    } else {
        (s2.to_string(), s1.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::KnowledgeGraphBuilder;
    use crate::model::{DownstreamRefs, ItemType, UpstreamRefs};
    use crate::test_utils::{
        create_test_item, create_test_item_with_refs, create_test_item_with_upstream,
    };

    #[test]
    fn test_no_redundancy() {
        // SARCH satisfies SYSREQ, but SYSREQ doesn't declare is_satisfied_by
        let sysreq = create_test_item("SYSREQ-001", ItemType::SystemRequirement);
        let sarch = create_test_item_with_upstream(
            "SARCH-001",
            ItemType::SystemArchitecture,
            UpstreamRefs {
                satisfies: vec![ItemId::new_unchecked("SYSREQ-001")],
                ..Default::default()
            },
        );

        let graph = KnowledgeGraphBuilder::new()
            .add_item(sysreq)
            .add_item(sarch)
            .build()
            .unwrap();

        let rule = RedundantRelationshipsRule;
        let warnings = rule.validate(&graph, &ValidationConfig::default());
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_no_investigation_redundancy() {
        // THS-001 declares hypotheses: [HYP-001] (downstream)
        // HYP-001 declares parent: [THS-001] (upstream, different rel type)
        // This is NOT redundant — children↔parent would be, but hypotheses↔parent is not
        let thesis = create_test_item_with_refs(
            "THS-001",
            ItemType::Thesis,
            UpstreamRefs::default(),
            DownstreamRefs {
                hypotheses: vec![ItemId::new_unchecked("HYP-001")],
                ..Default::default()
            },
        );
        let hypothesis = create_test_item_with_upstream(
            "HYP-001",
            ItemType::Hypothesis,
            UpstreamRefs {
                parent: vec![ItemId::new_unchecked("THS-001")],
                ..Default::default()
            },
        );

        let graph = KnowledgeGraphBuilder::new()
            .add_item(thesis)
            .add_item(hypothesis)
            .build()
            .unwrap();

        let rule = RedundantRelationshipsRule;
        let warnings = rule.validate(&graph, &ValidationConfig::default());
        assert!(
            warnings.is_empty(),
            "Different relationship types should not be flagged as redundant"
        );
    }

    #[test]
    fn test_redundant_children_parent() {
        // Entity declares children: [EVD-001] AND Evidence declares parent: [ITM-001]
        // This IS redundant — both sides declare the same relationship
        let entity = create_test_item_with_refs(
            "ITM-001",
            ItemType::Entity,
            UpstreamRefs::default(),
            DownstreamRefs {
                children: vec![ItemId::new_unchecked("EVD-001")],
                ..Default::default()
            },
        );
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

        let rule = RedundantRelationshipsRule;
        let warnings = rule.validate(&graph, &ValidationConfig::default());
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            &warnings[0],
            ValidationError::RedundantRelationship { from_rel, to_rel, .. }
            if *from_rel == RelationshipType::Children && *to_rel == RelationshipType::Parent
        ));
    }

    #[test]
    fn test_redundant_satisfies() {
        // Both declare the relationship - this is redundant
        let sysreq = create_test_item_with_refs(
            "SYSREQ-001",
            ItemType::SystemRequirement,
            UpstreamRefs::default(),
            DownstreamRefs {
                is_satisfied_by: vec![ItemId::new_unchecked("SARCH-001")],
                ..Default::default()
            },
        );
        let sarch = create_test_item_with_upstream(
            "SARCH-001",
            ItemType::SystemArchitecture,
            UpstreamRefs {
                satisfies: vec![ItemId::new_unchecked("SYSREQ-001")],
                ..Default::default()
            },
        );

        let graph = KnowledgeGraphBuilder::new()
            .add_item(sysreq)
            .add_item(sarch)
            .build()
            .unwrap();

        let rule = RedundantRelationshipsRule;
        let warnings = rule.validate(&graph, &ValidationConfig::default());
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            &warnings[0],
            ValidationError::RedundantRelationship { from_id, to_id, .. }
            if from_id.as_str() == "SYSREQ-001" && to_id.as_str() == "SARCH-001"
        ));
    }
}
