//! Knowledge graph implementation using petgraph.

use petgraph::Direction;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::SaraError;
use crate::model::{Item, ItemId, ItemType, RelationshipType};

/// The main knowledge graph container.
#[derive(Debug)]
pub struct KnowledgeGraph {
    /// The underlying directed graph.
    graph: DiGraph<Item, RelationshipType>,

    /// Index for O(1) lookup by ItemId.
    index: HashMap<ItemId, NodeIndex>,
}

impl KnowledgeGraph {
    /// Creates a new empty knowledge graph.
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            index: HashMap::new(),
        }
    }

    /// Returns the number of items in the graph.
    pub fn item_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Returns the number of relationships in the graph.
    pub fn relationship_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Adds an item to the graph.
    fn add_item(&mut self, item: Item) -> NodeIndex {
        let id = item.id.clone();
        let idx = self.graph.add_node(item);
        self.index.insert(id, idx);
        idx
    }

    /// Adds a relationship between two items.
    fn add_relationship(&mut self, from: &ItemId, to: &ItemId, rel_type: RelationshipType) {
        if let (Some(from_idx), Some(to_idx)) = (self.index.get(from), self.index.get(to)) {
            self.graph.add_edge(*from_idx, *to_idx, rel_type);
        }
    }

    /// Gets an item by ID.
    pub fn get(&self, id: &ItemId) -> Option<&Item> {
        let idx = self.index.get(id)?;
        self.graph.node_weight(*idx)
    }

    /// Gets a mutable reference to an item by ID.
    pub fn get_mut(&mut self, id: &ItemId) -> Option<&mut Item> {
        let idx = self.index.get(id)?;
        self.graph.node_weight_mut(*idx)
    }

    /// Checks if an item exists in the graph.
    pub fn contains(&self, id: &ItemId) -> bool {
        self.index.contains_key(id)
    }

    /// Returns all items in the graph.
    pub fn items(&self) -> impl Iterator<Item = &Item> {
        self.graph.node_weights()
    }

    /// Returns all item IDs in the graph.
    pub fn item_ids(&self) -> impl Iterator<Item = &ItemId> {
        self.index.keys()
    }

    /// Returns all items of a specific type.
    pub fn items_by_type(&self, item_type: ItemType) -> Vec<&Item> {
        self.graph
            .node_weights()
            .filter(|item| item.item_type == item_type)
            .collect()
    }

    /// Returns the count of items by type.
    pub fn count_by_type(&self) -> HashMap<ItemType, usize> {
        let mut counts = HashMap::new();
        for item in self.graph.node_weights() {
            *counts.entry(item.item_type).or_insert(0) += 1;
        }
        counts
    }

    /// Returns direct parents of an item (items that this item relates to upstream).
    pub fn parents(&self, id: &ItemId) -> Vec<&Item> {
        let Some(idx) = self.index.get(id) else {
            return Vec::new();
        };

        self.graph
            .edges_directed(*idx, Direction::Outgoing)
            .filter(|edge| edge.weight().is_upstream())
            .filter_map(|edge| self.graph.node_weight(edge.target()))
            .collect()
    }

    /// Returns direct children of an item (items that relate to this item downstream).
    pub fn children(&self, id: &ItemId) -> Vec<&Item> {
        let Some(idx) = self.index.get(id) else {
            return Vec::new();
        };

        self.graph
            .edges_directed(*idx, Direction::Incoming)
            .filter(|edge| edge.weight().is_upstream())
            .filter_map(|edge| self.graph.node_weight(edge.source()))
            .collect()
    }

    /// Returns all items with no upstream parents (potential orphans).
    pub fn orphans(&self) -> Vec<&Item> {
        self.graph
            .node_weights()
            .filter(|item| {
                // Solutions are allowed to have no parents (root of hierarchy)
                // Derived items are auto-generated; suppress orphan warnings
                if item.item_type.is_root() || item.derived {
                    return false;
                }
                // Check if item has any upstream references
                item.upstream.is_empty()
            })
            .collect()
    }

    /// Returns the underlying petgraph for advanced operations.
    pub fn inner(&self) -> &DiGraph<Item, RelationshipType> {
        &self.graph
    }

    /// Returns a mutable reference to the underlying petgraph.
    pub fn inner_mut(&mut self) -> &mut DiGraph<Item, RelationshipType> {
        &mut self.graph
    }

    /// Returns the node index for an item ID.
    pub fn node_index(&self, id: &ItemId) -> Option<NodeIndex> {
        self.index.get(id).copied()
    }

    /// Checks if the graph has cycles.
    pub fn has_cycles(&self) -> bool {
        petgraph::algo::is_cyclic_directed(&self.graph)
    }

    /// Returns all relationships in the graph.
    pub fn relationships(&self) -> Vec<(ItemId, ItemId, RelationshipType)> {
        self.graph
            .edge_references()
            .filter_map(|edge| {
                let from = self.graph.node_weight(edge.source())?;
                let to = self.graph.node_weight(edge.target())?;
                Some((from.id.clone(), to.id.clone(), *edge.weight()))
            })
            .collect()
    }
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing knowledge graphs.
#[derive(Debug, Default)]
pub struct KnowledgeGraphBuilder {
    items: Vec<Item>,
    repositories: Vec<PathBuf>,
}

impl KnowledgeGraphBuilder {
    /// Creates a new graph builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a repository path.
    pub fn add_repository(mut self, path: impl Into<PathBuf>) -> Self {
        self.repositories.push(path.into());
        self
    }

    /// Adds an item to the graph.
    pub fn add_item(mut self, item: Item) -> Self {
        self.items.push(item);
        self
    }

    /// Adds multiple items to the graph.
    pub fn add_items(mut self, items: impl IntoIterator<Item = Item>) -> Self {
        self.items.extend(items);
        self
    }

    /// Builds the knowledge graph.
    pub fn build(self) -> Result<KnowledgeGraph, SaraError> {
        let mut graph = KnowledgeGraph::new();

        // First pass: add all items
        for item in &self.items {
            graph.add_item(item.clone());
        }

        // Second pass: add relationships based on item references
        for item in &self.items {
            self.add_relationships_for_item(&mut graph, item);
        }

        Ok(graph)
    }

    /// Adds relationships for an item based on its references.
    fn add_relationships_for_item(&self, graph: &mut KnowledgeGraph, item: &Item) {
        // Add upstream relationships
        for target_id in &item.upstream.refines {
            graph.add_relationship(&item.id, target_id, RelationshipType::Refines);
        }
        for target_id in &item.upstream.derives_from {
            graph.add_relationship(&item.id, target_id, RelationshipType::DerivesFrom);
        }
        for target_id in &item.upstream.satisfies {
            graph.add_relationship(&item.id, target_id, RelationshipType::Satisfies);
        }
        // ADR justifies design artifacts (standard upstream relationship)
        for target_id in &item.upstream.justifies {
            graph.add_relationship(&item.id, target_id, RelationshipType::Justifies);
            // Add inverse: target is justified by this ADR
            graph.add_relationship(target_id, &item.id, RelationshipType::IsJustifiedBy);
        }

        // Add downstream relationships (and their inverse for bidirectional graph queries)
        for target_id in &item.downstream.is_refined_by {
            graph.add_relationship(&item.id, target_id, RelationshipType::IsRefinedBy);
            // Add inverse: target refines this item
            graph.add_relationship(target_id, &item.id, RelationshipType::Refines);
        }
        for target_id in &item.downstream.derives {
            graph.add_relationship(&item.id, target_id, RelationshipType::Derives);
            // Add inverse: target derives_from this item
            graph.add_relationship(target_id, &item.id, RelationshipType::DerivesFrom);
        }
        for target_id in &item.downstream.is_satisfied_by {
            graph.add_relationship(&item.id, target_id, RelationshipType::IsSatisfiedBy);
            // Add inverse: target satisfies this item
            graph.add_relationship(target_id, &item.id, RelationshipType::Satisfies);
        }
        // Design artifact is justified by ADRs (standard downstream relationship)
        for adr_id in &item.downstream.justified_by {
            graph.add_relationship(&item.id, adr_id, RelationshipType::IsJustifiedBy);
            // Add inverse: ADR justifies this item
            graph.add_relationship(adr_id, &item.id, RelationshipType::Justifies);
        }

        // Add peer dependencies (for requirement types)
        for target_id in item.attributes.depends_on() {
            graph.add_relationship(&item.id, target_id, RelationshipType::DependsOn);
        }

        // ADR supersession (peer relationships between ADRs, stored in attributes)
        for target_id in item.attributes.supersedes() {
            graph.add_relationship(&item.id, target_id, RelationshipType::Supersedes);
            // Add inverse: target is superseded by this ADR
            graph.add_relationship(target_id, &item.id, RelationshipType::IsSupersededBy);
        }

        // Investigation upstream relationships
        for target_id in &item.upstream.parent {
            graph.add_relationship(&item.id, target_id, RelationshipType::Parent);
            graph.add_relationship(target_id, &item.id, RelationshipType::Children);
        }
        for target_id in &item.upstream.cites {
            graph.add_relationship(&item.id, target_id, RelationshipType::Cites);
            graph.add_relationship(target_id, &item.id, RelationshipType::CitedBy);
        }
        for target_id in &item.upstream.evaluates {
            graph.add_relationship(&item.id, target_id, RelationshipType::Evaluates);
            graph.add_relationship(target_id, &item.id, RelationshipType::EvaluatedBy);
        }
        for target_id in &item.upstream.established_by {
            graph.add_relationship(&item.id, target_id, RelationshipType::EstablishedBy);
            graph.add_relationship(target_id, &item.id, RelationshipType::Establishes);
        }
        for target_id in &item.upstream.raised_by {
            graph.add_relationship(&item.id, target_id, RelationshipType::RaisedBy);
            graph.add_relationship(target_id, &item.id, RelationshipType::Raises);
        }

        // Investigation downstream relationships
        for target_id in &item.downstream.children {
            graph.add_relationship(&item.id, target_id, RelationshipType::Children);
            graph.add_relationship(target_id, &item.id, RelationshipType::Parent);
        }
        for target_id in &item.downstream.premises {
            graph.add_relationship(&item.id, target_id, RelationshipType::InvestigationPremises);
            graph.add_relationship(target_id, &item.id, RelationshipType::PremiseOf);
        }
        for target_id in &item.downstream.gaps {
            graph.add_relationship(&item.id, target_id, RelationshipType::InvestigationGaps);
            graph.add_relationship(target_id, &item.id, RelationshipType::GapOf);
        }
        for target_id in &item.downstream.hypotheses {
            graph.add_relationship(
                &item.id,
                target_id,
                RelationshipType::InvestigationHypotheses,
            );
            graph.add_relationship(target_id, &item.id, RelationshipType::HypothesisOf);
        }
        for target_id in &item.downstream.analyses {
            graph.add_relationship(&item.id, target_id, RelationshipType::InvestigationAnalyses);
            graph.add_relationship(target_id, &item.id, RelationshipType::AnalysisOf);
        }

        // Investigation peer relationships
        for target_id in &item.upstream.affects {
            graph.add_relationship(&item.id, target_id, RelationshipType::Affects);
            graph.add_relationship(target_id, &item.id, RelationshipType::AffectedBy);
        }

        // N-ary participant relationships
        for participant in &item.participants {
            graph.add_relationship(&item.id, &participant.entity, RelationshipType::Participant);
            graph.add_relationship(
                &participant.entity,
                &item.id,
                RelationshipType::ParticipantOf,
            );
        }

        // Entity-entity edges from evidence envelopes
        if item.item_type == ItemType::Evidence {
            // Messages: from → each to (deduplicated per entity pair)
            let mut seen_comm = std::collections::HashSet::new();
            for msg in item.attributes.messages() {
                for to_id in &msg.to {
                    if seen_comm.insert((&msg.from, to_id)) {
                        graph.add_relationship(
                            &msg.from,
                            to_id,
                            RelationshipType::CommunicatedWith,
                        );
                        graph.add_relationship(
                            to_id,
                            &msg.from,
                            RelationshipType::ReceivedCommunicationFrom,
                        );
                    }
                }
            }
            // Flights: passenger co-occurrence (deduplicated)
            let mut seen_travel = std::collections::HashSet::new();
            for flight in item.attributes.flights() {
                for i in 0..flight.passengers.len() {
                    for j in (i + 1)..flight.passengers.len() {
                        let (a, b) = (&flight.passengers[i], &flight.passengers[j]);
                        let key = if a < b { (a, b) } else { (b, a) };
                        if seen_travel.insert(key) {
                            graph.add_relationship(a, b, RelationshipType::TraveledWith);
                            graph.add_relationship(b, a, RelationshipType::TraveledWith);
                        }
                    }
                }
            }
            // Transactions: from → to
            for txn in item.attributes.transactions() {
                graph.add_relationship(&txn.from, &txn.to, RelationshipType::PaidTo);
                graph.add_relationship(&txn.to, &txn.from, RelationshipType::ReceivedPaymentFrom);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::UpstreamRefs;
    use crate::test_utils::{
        create_test_adr, create_test_investigation_item, create_test_item,
        create_test_item_with_refs, create_test_item_with_upstream,
    };

    #[test]
    fn test_add_and_get_item() {
        let graph = KnowledgeGraphBuilder::new()
            .add_item(create_test_item("SOL-001", ItemType::Solution))
            .build()
            .unwrap();

        let id = ItemId::new_unchecked("SOL-001");
        assert!(graph.contains(&id));
        assert_eq!(graph.get(&id).unwrap().name, "Test SOL-001");
    }

    #[test]
    fn test_items_by_type() {
        let graph = KnowledgeGraphBuilder::new()
            .add_item(create_test_item("SOL-001", ItemType::Solution))
            .add_item(create_test_item("UC-001", ItemType::UseCase))
            .add_item(create_test_item("UC-002", ItemType::UseCase))
            .build()
            .unwrap();

        let solutions = graph.items_by_type(ItemType::Solution);
        assert_eq!(solutions.len(), 1);

        let use_cases = graph.items_by_type(ItemType::UseCase);
        assert_eq!(use_cases.len(), 2);
    }

    #[test]
    fn test_item_count() {
        let graph = KnowledgeGraphBuilder::new().build().unwrap();
        assert_eq!(graph.item_count(), 0);

        let graph = KnowledgeGraphBuilder::new()
            .add_item(create_test_item("SOL-001", ItemType::Solution))
            .build()
            .unwrap();
        assert_eq!(graph.item_count(), 1);

        let graph = KnowledgeGraphBuilder::new()
            .add_item(create_test_item("SOL-001", ItemType::Solution))
            .add_item(create_test_item("UC-001", ItemType::UseCase))
            .build()
            .unwrap();
        assert_eq!(graph.item_count(), 2);
    }

    #[test]
    fn test_build_simple_graph() {
        let graph = KnowledgeGraphBuilder::new()
            .add_item(create_test_item("SOL-001", ItemType::Solution))
            .build()
            .unwrap();

        assert_eq!(graph.item_count(), 1);
    }

    #[test]
    fn test_build_graph_with_relationships() {
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

        assert_eq!(graph.item_count(), 2);
        assert_eq!(graph.relationship_count(), 1);
    }

    #[test]
    fn test_adr_justifies_relationship() {
        // Create a system architecture item
        let sysarch = create_test_item("SYSARCH-001", ItemType::SystemArchitecture);
        // Create an ADR that justifies it
        let adr = create_test_adr("ADR-001", &["SYSARCH-001"], &[]);

        let graph = KnowledgeGraphBuilder::new()
            .add_item(sysarch)
            .add_item(adr)
            .build()
            .unwrap();

        assert_eq!(graph.item_count(), 2);
        // ADR-001 -> Justifies -> SYSARCH-001
        // SYSARCH-001 -> IsJustifiedBy -> ADR-001
        assert_eq!(graph.relationship_count(), 2);
    }

    #[test]
    fn test_adr_supersession_relationship() {
        // Create two ADRs where the newer one supersedes the older
        let adr_old = create_test_adr("ADR-001", &[], &[]);
        let adr_new = create_test_adr("ADR-002", &[], &["ADR-001"]);

        let graph = KnowledgeGraphBuilder::new()
            .add_item(adr_old)
            .add_item(adr_new)
            .build()
            .unwrap();

        assert_eq!(graph.item_count(), 2);
        // ADR-002 -> Supersedes -> ADR-001
        // ADR-001 -> IsSupersededBy -> ADR-002
        assert_eq!(graph.relationship_count(), 2);
    }

    #[test]
    fn test_investigation_graph_construction() {
        use crate::model::DownstreamRefs;

        // Build a minimal investigation graph:
        // ITM-001 (entity, root)
        // THS-001 (thesis, root, downstream: hypotheses→HYP-001, analyses→ANL-001)
        // EVD-001 (evidence, parent→ITM-001)
        // HYP-001 (hypothesis, parent→THS-001)
        // ANL-001 (analysis, parent→THS-001, cites→EVD-001, evaluates→HYP-001,
        //          downstream: premises→PRM-001, gaps→QST-001)
        // PRM-001 (premise, established_by→THS-001)
        // QST-001 (question, raised_by→ANL-001)
        // BLK-001 (block, affects→EVD-001)

        let entity = create_test_item("ITM-001", ItemType::Entity);
        let thesis = create_test_item_with_refs(
            "THS-001",
            ItemType::Thesis,
            UpstreamRefs::default(),
            DownstreamRefs {
                hypotheses: vec![ItemId::new_unchecked("HYP-001")],
                analyses: vec![ItemId::new_unchecked("ANL-001")],
                ..Default::default()
            },
        );
        let evidence = create_test_investigation_item("EVD-001", ItemType::Evidence, &["ITM-001"]);
        let hypothesis =
            create_test_investigation_item("HYP-001", ItemType::Hypothesis, &["THS-001"]);
        let analysis_upstream = UpstreamRefs {
            parent: vec![ItemId::new_unchecked("THS-001")],
            cites: vec![ItemId::new_unchecked("EVD-001")],
            evaluates: vec![ItemId::new_unchecked("HYP-001")],
            ..Default::default()
        };
        let analysis_downstream = DownstreamRefs {
            premises: vec![ItemId::new_unchecked("PRM-001")],
            gaps: vec![ItemId::new_unchecked("QST-001")],
            ..Default::default()
        };
        let analysis = create_test_item_with_refs(
            "ANL-001",
            ItemType::Analysis,
            analysis_upstream,
            analysis_downstream,
        );
        let premise = create_test_item_with_upstream(
            "PRM-001",
            ItemType::Premise,
            UpstreamRefs {
                established_by: vec![ItemId::new_unchecked("THS-001")],
                ..Default::default()
            },
        );
        let question = create_test_item_with_upstream(
            "QST-001",
            ItemType::Question,
            UpstreamRefs {
                raised_by: vec![ItemId::new_unchecked("ANL-001")],
                ..Default::default()
            },
        );
        let block = create_test_item_with_upstream(
            "BLK-001",
            ItemType::Block,
            UpstreamRefs {
                affects: vec![ItemId::new_unchecked("EVD-001")],
                ..Default::default()
            },
        );

        let graph = KnowledgeGraphBuilder::new()
            .add_item(entity)
            .add_item(thesis)
            .add_item(evidence)
            .add_item(hypothesis)
            .add_item(analysis)
            .add_item(premise)
            .add_item(question)
            .add_item(block)
            .build()
            .unwrap();

        assert_eq!(graph.item_count(), 8);

        // Verify nodes exist
        assert!(graph.contains(&ItemId::new_unchecked("ITM-001")));
        assert!(graph.contains(&ItemId::new_unchecked("ANL-001")));
        assert!(graph.contains(&ItemId::new_unchecked("BLK-001")));

        // Edges are bidirectional, so each declared relationship creates 2 edges.
        // Declared relations:
        //   EVD-001→ITM-001 (parent)        = 2
        //   THS-001→HYP-001 (hypotheses)    = 2
        //   THS-001→ANL-001 (analyses)      = 2
        //   HYP-001→THS-001 (parent)        = 2
        //   ANL-001→THS-001 (parent)        = 2
        //   ANL-001→EVD-001 (cites)         = 2
        //   ANL-001→HYP-001 (evaluates)     = 2
        //   ANL-001→PRM-001 (premises)      = 2
        //   ANL-001→QST-001 (gaps)          = 2
        //   PRM-001→THS-001 (established_by)= 2
        //   QST-001→ANL-001 (raised_by)     = 2
        //   BLK-001→EVD-001 (affects)       = 2
        // Total: 12 * 2 = 24 edges
        assert_eq!(graph.relationship_count(), 24);
    }

    #[test]
    fn test_envelope_message_edges() {
        use crate::model::{EnvelopeMessage, ItemBuilder, Participant};

        // Evidence with two messages: Alice→Bob, Bob→Alice
        // Should create CommunicatedWith + ReceivedCommunicationFrom edges,
        // deduplicated per directed entity pair.
        let entity_a = create_test_item("ITM-alice", ItemType::Entity);
        let entity_b = create_test_item("ITM-bob", ItemType::Entity);
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
                // Second message same pair — should NOT create duplicate edges
                EnvelopeMessage {
                    id: 2,
                    from: ItemId::new_unchecked("ITM-alice"),
                    to: vec![ItemId::new_unchecked("ITM-bob")],
                    date: None,
                    subject: None,
                    cc: None,
                    bcc: None,
                    forward: None,
                    removed: None,
                },
                // Reverse direction — distinct pair, should create new edges
                EnvelopeMessage {
                    id: 3,
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

        // Participant edges: 2 participants × 2 (bidirectional) = 4
        // Message edges:
        //   Alice→Bob (CommunicatedWith) + Bob→Alice (ReceivedCommunicationFrom) = 2
        //   Bob→Alice (CommunicatedWith) + Alice→Bob (ReceivedCommunicationFrom) = 2
        //   (second Alice→Bob message is deduplicated)
        // Total: 4 + 4 = 8
        assert_eq!(graph.relationship_count(), 8);

        // Verify the edge types exist
        let inner = graph.inner();
        let alice_idx = graph
            .node_index(&ItemId::new_unchecked("ITM-alice"))
            .unwrap();
        let bob_idx = graph.node_index(&ItemId::new_unchecked("ITM-bob")).unwrap();

        let alice_to_bob: Vec<_> = inner
            .edges_connecting(alice_idx, bob_idx)
            .map(|e| *e.weight())
            .collect();
        assert!(
            alice_to_bob.contains(&RelationshipType::CommunicatedWith),
            "Expected CommunicatedWith edge Alice→Bob"
        );
    }

    #[test]
    fn test_envelope_flight_edges() {
        use crate::model::{EnvelopeFlight, ItemBuilder, Participant};

        // Flight with 3 passengers → should create TraveledWith for all pairs
        let entity_a = create_test_item("ITM-a", ItemType::Entity);
        let entity_b = create_test_item("ITM-b", ItemType::Entity);
        let entity_c = create_test_item("ITM-c", ItemType::Entity);
        let loc = create_test_item("ITM-loc", ItemType::Entity);

        let evidence = ItemBuilder::new()
            .id(ItemId::new_unchecked("EVD-001"))
            .item_type(ItemType::Evidence)
            .name("Flights")
            .source(crate::model::SourceLocation::new(
                std::path::PathBuf::from("/test"),
                "EVD-001.md",
            ))
            .participants(vec![
                Participant {
                    entity: ItemId::new_unchecked("ITM-a"),
                    role: "passenger".into(),
                },
                Participant {
                    entity: ItemId::new_unchecked("ITM-b"),
                    role: "passenger".into(),
                },
                Participant {
                    entity: ItemId::new_unchecked("ITM-c"),
                    role: "passenger".into(),
                },
                Participant {
                    entity: ItemId::new_unchecked("ITM-loc"),
                    role: "location".into(),
                },
            ])
            .envelope_flights(vec![EnvelopeFlight {
                id: 1,
                date: "2024-01-01".into(),
                origin: ItemId::new_unchecked("ITM-loc"),
                destination: ItemId::new_unchecked("ITM-loc"),
                aircraft: None,
                passengers: vec![
                    ItemId::new_unchecked("ITM-a"),
                    ItemId::new_unchecked("ITM-b"),
                    ItemId::new_unchecked("ITM-c"),
                ],
            }])
            .build()
            .unwrap();

        let graph = KnowledgeGraphBuilder::new()
            .add_item(entity_a)
            .add_item(entity_b)
            .add_item(entity_c)
            .add_item(loc)
            .add_item(evidence)
            .build()
            .unwrap();

        // Participant edges: 4 participants × 2 = 8
        // Flight co-occurrence: 3 pairs (a-b, a-c, b-c) × 2 (symmetric) = 6
        // Total: 8 + 6 = 14
        assert_eq!(graph.relationship_count(), 14);
    }

    #[test]
    fn test_envelope_transaction_edges() {
        use crate::model::{EnvelopeTransaction, ItemBuilder, Participant};

        let entity_a = create_test_item("ITM-a", ItemType::Entity);
        let entity_b = create_test_item("ITM-b", ItemType::Entity);

        let evidence = ItemBuilder::new()
            .id(ItemId::new_unchecked("EVD-001"))
            .item_type(ItemType::Evidence)
            .name("Transactions")
            .source(crate::model::SourceLocation::new(
                std::path::PathBuf::from("/test"),
                "EVD-001.md",
            ))
            .participants(vec![
                Participant {
                    entity: ItemId::new_unchecked("ITM-a"),
                    role: "payer".into(),
                },
                Participant {
                    entity: ItemId::new_unchecked("ITM-b"),
                    role: "payee".into(),
                },
            ])
            .envelope_transactions(vec![EnvelopeTransaction {
                id: 1,
                date: "2024-01-01".into(),
                from: ItemId::new_unchecked("ITM-a"),
                to: ItemId::new_unchecked("ITM-b"),
                amount: 1000.0,
                currency: "USD".into(),
                method: None,
            }])
            .build()
            .unwrap();

        let graph = KnowledgeGraphBuilder::new()
            .add_item(entity_a)
            .add_item(entity_b)
            .add_item(evidence)
            .build()
            .unwrap();

        // Participant edges: 2 × 2 = 4
        // Transaction edges: PaidTo + ReceivedPaymentFrom = 2
        // Total: 6
        assert_eq!(graph.relationship_count(), 6);

        let inner = graph.inner();
        let a_idx = graph.node_index(&ItemId::new_unchecked("ITM-a")).unwrap();
        let b_idx = graph.node_index(&ItemId::new_unchecked("ITM-b")).unwrap();

        let a_to_b: Vec<_> = inner
            .edges_connecting(a_idx, b_idx)
            .map(|e| *e.weight())
            .collect();
        assert!(a_to_b.contains(&RelationshipType::PaidTo));

        let b_to_a: Vec<_> = inner
            .edges_connecting(b_idx, a_idx)
            .map(|e| *e.weight())
            .collect();
        assert!(b_to_a.contains(&RelationshipType::ReceivedPaymentFrom));
    }
}
