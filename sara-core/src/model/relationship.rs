//! Relationship types and structures for the knowledge graph.

use serde::{Deserialize, Serialize};

use super::field::FieldName;
use super::item::{ItemId, ItemType};

/// Represents the type of relationship between items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    /// Refinement: child refines parent (Scenario refines Use Case).
    Refines,
    /// Inverse of Refines: parent is refined by child.
    IsRefinedBy,
    /// Derivation: parent derives child (Scenario derives System Requirement).
    Derives,
    /// Inverse of Derives: child derives from parent.
    DerivesFrom,
    /// Satisfaction: child satisfies parent (System Architecture satisfies System Requirement).
    Satisfies,
    /// Inverse of Satisfies: parent is satisfied by child.
    IsSatisfiedBy,
    /// Dependency: Requirement depends on another Requirement of the same type.
    DependsOn,
    /// Inverse of DependsOn: Requirement is required by another.
    IsRequiredBy,
    /// Justification: ADR justifies a design artifact (SYSARCH, SWDD, HWDD).
    Justifies,
    /// Inverse of Justifies: design artifact is justified by an ADR.
    IsJustifiedBy,
    /// Supersession: newer ADR supersedes older ADR.
    Supersedes,
    /// Inverse of Supersedes: older ADR is superseded by newer ADR.
    IsSupersededBy,

    // Investigation relationship types
    /// Hierarchical parent (evidence/hypothesis/analysis/question → entity/thesis).
    Parent,
    /// Inverse of Parent.
    Children,
    /// Citation (analysis → evidence).
    Cites,
    /// Inverse of Cites.
    CitedBy,
    /// Evaluation (analysis → hypothesis).
    Evaluates,
    /// Inverse of Evaluates.
    EvaluatedBy,
    /// Downstream: analysis → premise.
    InvestigationPremises,
    /// Inverse of Premises.
    PremiseOf,
    /// Downstream: analysis → question (gaps).
    InvestigationGaps,
    /// Inverse of Gaps.
    GapOf,
    /// Upstream: premise → thesis.
    EstablishedBy,
    /// Inverse of EstablishedBy.
    Establishes,
    /// Upstream: question → analysis/evidence.
    RaisedBy,
    /// Inverse of RaisedBy.
    Raises,
    /// Peer constraint: block → evidence/analysis/question.
    Affects,
    /// Inverse of Affects.
    AffectedBy,
    /// Downstream: thesis → hypothesis.
    InvestigationHypotheses,
    /// Inverse of Hypotheses.
    HypothesisOf,
    /// Downstream: thesis → analysis.
    InvestigationAnalyses,
    /// Inverse of Analyses.
    AnalysisOf,

    /// N-ary participant reference (item → entity).
    Participant,
    /// Inverse of Participant.
    ParticipantOf,
}

impl RelationshipType {
    /// Get the inverse relationship type.
    #[must_use]
    pub const fn inverse(&self) -> Self {
        match self {
            Self::Refines => Self::IsRefinedBy,
            Self::IsRefinedBy => Self::Refines,
            Self::Derives => Self::DerivesFrom,
            Self::DerivesFrom => Self::Derives,
            Self::Satisfies => Self::IsSatisfiedBy,
            Self::IsSatisfiedBy => Self::Satisfies,
            Self::DependsOn => Self::IsRequiredBy,
            Self::IsRequiredBy => Self::DependsOn,
            Self::Justifies => Self::IsJustifiedBy,
            Self::IsJustifiedBy => Self::Justifies,
            Self::Supersedes => Self::IsSupersededBy,
            Self::IsSupersededBy => Self::Supersedes,
            Self::Parent => Self::Children,
            Self::Children => Self::Parent,
            Self::Cites => Self::CitedBy,
            Self::CitedBy => Self::Cites,
            Self::Evaluates => Self::EvaluatedBy,
            Self::EvaluatedBy => Self::Evaluates,
            Self::InvestigationPremises => Self::PremiseOf,
            Self::PremiseOf => Self::InvestigationPremises,
            Self::InvestigationGaps => Self::GapOf,
            Self::GapOf => Self::InvestigationGaps,
            Self::EstablishedBy => Self::Establishes,
            Self::Establishes => Self::EstablishedBy,
            Self::RaisedBy => Self::Raises,
            Self::Raises => Self::RaisedBy,
            Self::Affects => Self::AffectedBy,
            Self::AffectedBy => Self::Affects,
            Self::InvestigationHypotheses => Self::HypothesisOf,
            Self::HypothesisOf => Self::InvestigationHypotheses,
            Self::InvestigationAnalyses => Self::AnalysisOf,
            Self::AnalysisOf => Self::InvestigationAnalyses,
            Self::Participant => Self::ParticipantOf,
            Self::ParticipantOf => Self::Participant,
        }
    }

    /// Check if this is an upstream relationship (toward Solution).
    /// For ADRs, Justifies is considered upstream as it links ADR to design artifacts.
    #[must_use]
    pub const fn is_upstream(&self) -> bool {
        matches!(
            self,
            Self::Refines
                | Self::DerivesFrom
                | Self::Satisfies
                | Self::Justifies
                | Self::Parent
                | Self::Cites
                | Self::Evaluates
                | Self::EstablishedBy
                | Self::RaisedBy
                | Self::PremiseOf
                | Self::GapOf
                | Self::HypothesisOf
                | Self::AnalysisOf
                | Self::Participant
        )
    }

    /// Check if this is a downstream relationship (toward Detailed Designs).
    #[must_use]
    pub const fn is_downstream(&self) -> bool {
        matches!(
            self,
            Self::IsRefinedBy
                | Self::Derives
                | Self::IsSatisfiedBy
                | Self::IsJustifiedBy
                | Self::Children
                | Self::CitedBy
                | Self::EvaluatedBy
                | Self::Establishes
                | Self::Raises
                | Self::InvestigationPremises
                | Self::InvestigationGaps
                | Self::InvestigationHypotheses
                | Self::InvestigationAnalyses
                | Self::ParticipantOf
        )
    }

    /// Check if this is a peer relationship (between items of the same type).
    #[must_use]
    pub const fn is_peer(&self) -> bool {
        matches!(
            self,
            Self::DependsOn
                | Self::IsRequiredBy
                | Self::Supersedes
                | Self::IsSupersededBy
                | Self::Affects
                | Self::AffectedBy
        )
    }

    /// Check if this is a primary relationship (not an inverse).
    ///
    /// Primary relationships are the declared direction:
    /// - Refines, DerivesFrom, Satisfies, Justifies (upstream)
    /// - DependsOn, Supersedes (peer, primary)
    ///
    /// Inverse relationships exist only for graph traversal and should not
    /// be considered when checking for cycles.
    #[must_use]
    pub const fn is_primary(&self) -> bool {
        matches!(
            self,
            Self::Refines
                | Self::DerivesFrom
                | Self::Satisfies
                | Self::Justifies
                | Self::DependsOn
                | Self::Supersedes
                | Self::Parent
                | Self::Cites
                | Self::Evaluates
                | Self::EstablishedBy
                | Self::RaisedBy
                | Self::InvestigationPremises
                | Self::InvestigationGaps
                | Self::InvestigationHypotheses
                | Self::InvestigationAnalyses
                | Self::Affects
                | Self::Participant
        )
    }

    /// Returns the corresponding FieldName for this relationship type.
    #[must_use]
    pub const fn field_name(&self) -> FieldName {
        match self {
            Self::Refines => FieldName::Refines,
            Self::IsRefinedBy => FieldName::IsRefinedBy,
            Self::Derives => FieldName::Derives,
            Self::DerivesFrom => FieldName::DerivesFrom,
            Self::Satisfies => FieldName::Satisfies,
            Self::IsSatisfiedBy => FieldName::IsSatisfiedBy,
            Self::DependsOn => FieldName::DependsOn,
            Self::IsRequiredBy => FieldName::IsRequiredBy,
            Self::Justifies => FieldName::Justifies,
            Self::IsJustifiedBy => FieldName::JustifiedBy,
            Self::Supersedes => FieldName::Supersedes,
            Self::IsSupersededBy => FieldName::SupersededBy,
            Self::Parent => FieldName::Parent,
            Self::Children => FieldName::Children,
            Self::Cites => FieldName::Cites,
            Self::CitedBy => FieldName::CitedBy,
            Self::Evaluates => FieldName::Evaluates,
            Self::EvaluatedBy => FieldName::EvaluatedBy,
            Self::InvestigationPremises => FieldName::InvestigationPremises,
            Self::PremiseOf => FieldName::PremiseOf,
            Self::InvestigationGaps => FieldName::InvestigationGaps,
            Self::GapOf => FieldName::GapOf,
            Self::EstablishedBy => FieldName::EstablishedBy,
            Self::Establishes => FieldName::Establishes,
            Self::RaisedBy => FieldName::RaisedBy,
            Self::Raises => FieldName::Raises,
            Self::Affects => FieldName::Affects,
            Self::AffectedBy => FieldName::AffectedBy,
            Self::InvestigationHypotheses => FieldName::InvestigationHypotheses,
            Self::HypothesisOf => FieldName::HypothesisOf,
            Self::InvestigationAnalyses => FieldName::InvestigationAnalyses,
            Self::AnalysisOf => FieldName::AnalysisOf,
            Self::Participant => FieldName::Participants,
            Self::ParticipantOf => FieldName::ParticipantOf,
        }
    }
}

impl std::fmt::Display for RelationshipType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.field_name().as_str())
    }
}

/// Represents a link between two items in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Source item ID.
    pub from: ItemId,
    /// Target item ID.
    pub to: ItemId,
    /// Type of relationship.
    pub relationship_type: RelationshipType,
}

impl Relationship {
    /// Creates a new relationship.
    #[must_use]
    pub const fn new(from: ItemId, to: ItemId, relationship_type: RelationshipType) -> Self {
        Self {
            from,
            to,
            relationship_type,
        }
    }

    /// Returns the inverse relationship.
    #[must_use]
    pub fn inverse(&self) -> Self {
        Self {
            from: self.to.clone(),
            to: self.from.clone(),
            relationship_type: self.relationship_type.inverse(),
        }
    }
}

/// Valid relationship rules based on item types.
pub struct RelationshipRules;

impl RelationshipRules {
    /// Returns the valid upstream relationship types for a given item type.
    #[must_use]
    pub fn valid_upstream_for(item_type: ItemType) -> Option<(RelationshipType, Vec<ItemType>)> {
        match item_type {
            ItemType::Solution => None,
            ItemType::UseCase => Some((RelationshipType::Refines, vec![ItemType::Solution])),
            ItemType::Scenario => Some((RelationshipType::Refines, vec![ItemType::UseCase])),
            ItemType::SystemRequirement => {
                Some((RelationshipType::DerivesFrom, vec![ItemType::Scenario]))
            }
            ItemType::SystemArchitecture => Some((
                RelationshipType::Satisfies,
                vec![ItemType::SystemRequirement],
            )),
            ItemType::HardwareRequirement => Some((
                RelationshipType::DerivesFrom,
                vec![ItemType::SystemArchitecture],
            )),
            ItemType::SoftwareRequirement => Some((
                RelationshipType::DerivesFrom,
                vec![ItemType::SystemArchitecture],
            )),
            ItemType::HardwareDetailedDesign => Some((
                RelationshipType::Satisfies,
                vec![ItemType::HardwareRequirement],
            )),
            ItemType::SoftwareDetailedDesign => Some((
                RelationshipType::Satisfies,
                vec![ItemType::SoftwareRequirement],
            )),
            ItemType::ArchitectureDecisionRecord => Some((
                RelationshipType::Justifies,
                vec![
                    ItemType::SystemArchitecture,
                    ItemType::SoftwareDetailedDesign,
                    ItemType::HardwareDetailedDesign,
                ],
            )),
            // Investigation types
            ItemType::Entity | ItemType::Thesis | ItemType::Block => None,
            ItemType::Evidence | ItemType::Hypothesis | ItemType::Question => Some((
                RelationshipType::Parent,
                vec![ItemType::Entity, ItemType::Thesis],
            )),
            ItemType::Analysis => Some((
                RelationshipType::Parent,
                vec![ItemType::Entity, ItemType::Thesis],
            )),
            ItemType::Premise => Some((RelationshipType::EstablishedBy, vec![ItemType::Thesis])),
        }
    }

    /// Returns the valid downstream relationship types for a given item type.
    #[must_use]
    pub fn valid_downstream_for(item_type: ItemType) -> Option<(RelationshipType, Vec<ItemType>)> {
        match item_type {
            ItemType::Solution => Some((RelationshipType::IsRefinedBy, vec![ItemType::UseCase])),
            ItemType::UseCase => Some((RelationshipType::IsRefinedBy, vec![ItemType::Scenario])),
            ItemType::Scenario => {
                Some((RelationshipType::Derives, vec![ItemType::SystemRequirement]))
            }
            ItemType::SystemRequirement => Some((
                RelationshipType::IsSatisfiedBy,
                vec![ItemType::SystemArchitecture],
            )),
            ItemType::SystemArchitecture => Some((
                RelationshipType::Derives,
                vec![ItemType::HardwareRequirement, ItemType::SoftwareRequirement],
            )),
            ItemType::HardwareRequirement => Some((
                RelationshipType::IsSatisfiedBy,
                vec![ItemType::HardwareDetailedDesign],
            )),
            ItemType::SoftwareRequirement => Some((
                RelationshipType::IsSatisfiedBy,
                vec![ItemType::SoftwareDetailedDesign],
            )),
            ItemType::HardwareDetailedDesign | ItemType::SoftwareDetailedDesign => Some((
                RelationshipType::IsJustifiedBy,
                vec![ItemType::ArchitectureDecisionRecord],
            )),
            ItemType::ArchitectureDecisionRecord => None,
            // Investigation types
            ItemType::Entity => Some((
                RelationshipType::Children,
                vec![
                    ItemType::Evidence,
                    ItemType::Hypothesis,
                    ItemType::Analysis,
                    ItemType::Question,
                ],
            )),
            ItemType::Thesis => Some((
                RelationshipType::Children,
                vec![
                    ItemType::Evidence,
                    ItemType::Hypothesis,
                    ItemType::Analysis,
                    ItemType::Question,
                ],
            )),
            ItemType::Evidence
            | ItemType::Hypothesis
            | ItemType::Premise
            | ItemType::Question
            | ItemType::Block => None,
            ItemType::Analysis => Some((
                RelationshipType::InvestigationPremises,
                vec![ItemType::Premise],
            )),
        }
    }

    /// Returns the valid peer dependency types for a given item type.
    #[must_use]
    pub const fn valid_peer_for(item_type: ItemType) -> Option<ItemType> {
        match item_type {
            ItemType::SystemRequirement => Some(ItemType::SystemRequirement),
            ItemType::HardwareRequirement => Some(ItemType::HardwareRequirement),
            ItemType::SoftwareRequirement => Some(ItemType::SoftwareRequirement),
            ItemType::ArchitectureDecisionRecord => Some(ItemType::ArchitectureDecisionRecord),
            _ => None,
        }
    }

    /// Returns the valid justification targets for ADRs.
    #[must_use]
    pub fn valid_justification_targets() -> Vec<ItemType> {
        vec![
            ItemType::SystemArchitecture,
            ItemType::SoftwareDetailedDesign,
            ItemType::HardwareDetailedDesign,
        ]
    }

    /// Checks if a justification relationship is valid (ADR -> design artifact).
    #[must_use]
    pub fn is_valid_justification(from_type: ItemType, to_type: ItemType) -> bool {
        from_type == ItemType::ArchitectureDecisionRecord
            && Self::valid_justification_targets().contains(&to_type)
    }

    /// Checks if a supersession relationship is valid (ADR -> ADR).
    #[must_use]
    pub const fn is_valid_supersession(from_type: ItemType, to_type: ItemType) -> bool {
        matches!(from_type, ItemType::ArchitectureDecisionRecord)
            && matches!(to_type, ItemType::ArchitectureDecisionRecord)
    }

    /// Checks if a relationship is valid between two item types.
    #[must_use]
    pub fn is_valid_relationship(
        from_type: ItemType,
        to_type: ItemType,
        rel_type: RelationshipType,
    ) -> bool {
        match rel_type {
            // Upstream relationships
            RelationshipType::Refines
            | RelationshipType::DerivesFrom
            | RelationshipType::Satisfies
            | RelationshipType::Justifies => {
                if let Some((expected_rel, valid_targets)) = Self::valid_upstream_for(from_type) {
                    expected_rel == rel_type && valid_targets.contains(&to_type)
                } else {
                    false
                }
            }
            // Downstream relationships
            RelationshipType::IsRefinedBy
            | RelationshipType::Derives
            | RelationshipType::IsSatisfiedBy => {
                if let Some((expected_rel, valid_targets)) = Self::valid_downstream_for(from_type) {
                    expected_rel == rel_type && valid_targets.contains(&to_type)
                } else {
                    false
                }
            }
            // IsJustifiedBy needs special handling since design artifacts have multiple downstream types
            RelationshipType::IsJustifiedBy => Self::is_valid_justification(to_type, from_type),
            // Peer relationships (including ADR supersession)
            RelationshipType::DependsOn
            | RelationshipType::IsRequiredBy
            | RelationshipType::Supersedes
            | RelationshipType::IsSupersededBy => Self::valid_peer_for(from_type) == Some(to_type),
            // Investigation relationships — validated via traceability_configs, allow all for now
            RelationshipType::Parent
            | RelationshipType::Children
            | RelationshipType::Cites
            | RelationshipType::CitedBy
            | RelationshipType::Evaluates
            | RelationshipType::EvaluatedBy
            | RelationshipType::InvestigationPremises
            | RelationshipType::PremiseOf
            | RelationshipType::InvestigationGaps
            | RelationshipType::GapOf
            | RelationshipType::EstablishedBy
            | RelationshipType::Establishes
            | RelationshipType::RaisedBy
            | RelationshipType::Raises
            | RelationshipType::Affects
            | RelationshipType::AffectedBy
            | RelationshipType::InvestigationHypotheses
            | RelationshipType::HypothesisOf
            | RelationshipType::InvestigationAnalyses
            | RelationshipType::AnalysisOf
            | RelationshipType::Participant
            | RelationshipType::ParticipantOf => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relationship_type_inverse() {
        assert_eq!(
            RelationshipType::Refines.inverse(),
            RelationshipType::IsRefinedBy
        );
        assert_eq!(
            RelationshipType::Derives.inverse(),
            RelationshipType::DerivesFrom
        );
        assert_eq!(
            RelationshipType::Satisfies.inverse(),
            RelationshipType::IsSatisfiedBy
        );
        assert_eq!(
            RelationshipType::DependsOn.inverse(),
            RelationshipType::IsRequiredBy
        );
    }

    #[test]
    fn test_relationship_type_direction() {
        assert!(RelationshipType::Refines.is_upstream());
        assert!(RelationshipType::DerivesFrom.is_upstream());
        assert!(RelationshipType::Satisfies.is_upstream());

        assert!(RelationshipType::IsRefinedBy.is_downstream());
        assert!(RelationshipType::Derives.is_downstream());
        assert!(RelationshipType::IsSatisfiedBy.is_downstream());

        assert!(RelationshipType::DependsOn.is_peer());
        assert!(RelationshipType::IsRequiredBy.is_peer());
    }

    #[test]
    fn test_valid_relationships() {
        // UseCase refines Solution
        assert!(RelationshipRules::is_valid_relationship(
            ItemType::UseCase,
            ItemType::Solution,
            RelationshipType::Refines
        ));

        // Scenario refines UseCase
        assert!(RelationshipRules::is_valid_relationship(
            ItemType::Scenario,
            ItemType::UseCase,
            RelationshipType::Refines
        ));

        // SystemRequirement derives_from Scenario
        assert!(RelationshipRules::is_valid_relationship(
            ItemType::SystemRequirement,
            ItemType::Scenario,
            RelationshipType::DerivesFrom
        ));

        // Invalid: Solution refines nothing
        assert!(!RelationshipRules::is_valid_relationship(
            ItemType::Solution,
            ItemType::UseCase,
            RelationshipType::Refines
        ));
    }

    #[test]
    fn test_peer_dependencies() {
        assert!(RelationshipRules::is_valid_relationship(
            ItemType::SystemRequirement,
            ItemType::SystemRequirement,
            RelationshipType::DependsOn
        ));

        assert!(!RelationshipRules::is_valid_relationship(
            ItemType::Solution,
            ItemType::Solution,
            RelationshipType::DependsOn
        ));
    }

    #[test]
    fn test_adr_justifies_relationship() {
        // ADR can justify design artifacts
        assert!(RelationshipRules::is_valid_relationship(
            ItemType::ArchitectureDecisionRecord,
            ItemType::SystemArchitecture,
            RelationshipType::Justifies
        ));
        assert!(RelationshipRules::is_valid_relationship(
            ItemType::ArchitectureDecisionRecord,
            ItemType::SoftwareDetailedDesign,
            RelationshipType::Justifies
        ));
        assert!(RelationshipRules::is_valid_relationship(
            ItemType::ArchitectureDecisionRecord,
            ItemType::HardwareDetailedDesign,
            RelationshipType::Justifies
        ));

        // ADR cannot justify non-design artifacts
        assert!(!RelationshipRules::is_valid_relationship(
            ItemType::ArchitectureDecisionRecord,
            ItemType::SystemRequirement,
            RelationshipType::Justifies
        ));
    }

    #[test]
    fn test_adr_supersession_relationship() {
        // ADR can supersede other ADRs (peer relationship)
        assert!(RelationshipRules::is_valid_relationship(
            ItemType::ArchitectureDecisionRecord,
            ItemType::ArchitectureDecisionRecord,
            RelationshipType::Supersedes
        ));
        assert!(RelationshipRules::is_valid_relationship(
            ItemType::ArchitectureDecisionRecord,
            ItemType::ArchitectureDecisionRecord,
            RelationshipType::IsSupersededBy
        ));

        // ADR cannot supersede non-ADR items
        assert!(!RelationshipRules::is_valid_relationship(
            ItemType::ArchitectureDecisionRecord,
            ItemType::SystemArchitecture,
            RelationshipType::Supersedes
        ));
    }

    #[test]
    fn test_adr_relationship_direction() {
        // Justifies is upstream
        assert!(RelationshipType::Justifies.is_upstream());
        // IsJustifiedBy is downstream
        assert!(RelationshipType::IsJustifiedBy.is_downstream());
        // Supersedes/IsSupersededBy are peer
        assert!(RelationshipType::Supersedes.is_peer());
        assert!(RelationshipType::IsSupersededBy.is_peer());
    }

    #[test]
    fn test_investigation_relationship_types() {
        assert_eq!(
            RelationshipType::Parent.inverse(),
            RelationshipType::Children
        );
        assert_eq!(RelationshipType::Cites.inverse(), RelationshipType::CitedBy);
        assert_eq!(
            RelationshipType::Evaluates.inverse(),
            RelationshipType::EvaluatedBy
        );
        assert_eq!(
            RelationshipType::InvestigationPremises.inverse(),
            RelationshipType::PremiseOf
        );
        assert_eq!(
            RelationshipType::InvestigationGaps.inverse(),
            RelationshipType::GapOf
        );
        assert_eq!(
            RelationshipType::EstablishedBy.inverse(),
            RelationshipType::Establishes
        );
        assert_eq!(
            RelationshipType::RaisedBy.inverse(),
            RelationshipType::Raises
        );
        assert_eq!(
            RelationshipType::Affects.inverse(),
            RelationshipType::AffectedBy
        );
        assert_eq!(
            RelationshipType::InvestigationHypotheses.inverse(),
            RelationshipType::HypothesisOf
        );
        assert_eq!(
            RelationshipType::InvestigationAnalyses.inverse(),
            RelationshipType::AnalysisOf
        );
    }

    #[test]
    fn test_investigation_relationship_directions() {
        // Upstream
        assert!(RelationshipType::Parent.is_upstream());
        assert!(RelationshipType::Cites.is_upstream());
        assert!(RelationshipType::Evaluates.is_upstream());
        assert!(RelationshipType::EstablishedBy.is_upstream());
        assert!(RelationshipType::RaisedBy.is_upstream());

        // Downstream
        assert!(RelationshipType::Children.is_downstream());
        assert!(RelationshipType::CitedBy.is_downstream());
        assert!(RelationshipType::EvaluatedBy.is_downstream());
        assert!(RelationshipType::Establishes.is_downstream());
        assert!(RelationshipType::Raises.is_downstream());
        assert!(RelationshipType::InvestigationPremises.is_downstream());
        assert!(RelationshipType::InvestigationGaps.is_downstream());
        assert!(RelationshipType::InvestigationHypotheses.is_downstream());
        assert!(RelationshipType::InvestigationAnalyses.is_downstream());
        assert!(RelationshipType::PremiseOf.is_upstream());
        assert!(RelationshipType::GapOf.is_upstream());
        assert!(RelationshipType::HypothesisOf.is_upstream());
        assert!(RelationshipType::AnalysisOf.is_upstream());

        // Peer
        assert!(RelationshipType::Affects.is_peer());
        assert!(RelationshipType::AffectedBy.is_peer());
    }

    #[test]
    fn test_investigation_relationship_primary() {
        assert!(RelationshipType::Parent.is_primary());
        assert!(RelationshipType::Cites.is_primary());
        assert!(RelationshipType::Evaluates.is_primary());
        assert!(RelationshipType::EstablishedBy.is_primary());
        assert!(RelationshipType::RaisedBy.is_primary());
        assert!(RelationshipType::InvestigationPremises.is_primary());
        assert!(RelationshipType::InvestigationGaps.is_primary());
        assert!(RelationshipType::InvestigationHypotheses.is_primary());
        assert!(RelationshipType::InvestigationAnalyses.is_primary());
        assert!(RelationshipType::Affects.is_primary());

        assert!(!RelationshipType::Children.is_primary());
        assert!(!RelationshipType::AffectedBy.is_primary());
    }

    #[test]
    fn test_investigation_relationship_field_names() {
        assert_eq!(RelationshipType::Parent.field_name(), FieldName::Parent);
        assert_eq!(RelationshipType::Cites.field_name(), FieldName::Cites);
        assert_eq!(RelationshipType::Affects.field_name(), FieldName::Affects);
        assert_eq!(RelationshipType::Children.field_name(), FieldName::Children);
        assert_eq!(
            RelationshipType::InvestigationPremises.field_name(),
            FieldName::InvestigationPremises
        );
    }

    #[test]
    fn test_investigation_relationship_rules() {
        // Evidence → Entity/Thesis via Parent
        let result = RelationshipRules::valid_upstream_for(ItemType::Evidence);
        assert!(result.is_some());
        let (rel, targets) = result.unwrap();
        assert_eq!(rel, RelationshipType::Parent);
        assert!(targets.contains(&ItemType::Entity));
        assert!(targets.contains(&ItemType::Thesis));

        // Analysis → Entity/Thesis via Parent (primary upstream)
        let (rel, _) = RelationshipRules::valid_upstream_for(ItemType::Analysis).unwrap();
        assert_eq!(rel, RelationshipType::Parent);

        // Entity is root — no upstream
        assert!(RelationshipRules::valid_upstream_for(ItemType::Entity).is_none());
        assert!(RelationshipRules::valid_upstream_for(ItemType::Thesis).is_none());
        assert!(RelationshipRules::valid_upstream_for(ItemType::Block).is_none());

        // Premise → Thesis via EstablishedBy
        let (rel, targets) = RelationshipRules::valid_upstream_for(ItemType::Premise).unwrap();
        assert_eq!(rel, RelationshipType::EstablishedBy);
        assert!(targets.contains(&ItemType::Thesis));
    }
}
