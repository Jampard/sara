//! Field name definitions for YAML frontmatter.
//!
//! This module provides a single source of truth for all field names
//! used in YAML frontmatter serialization and deserialization.

/// All field names used in YAML frontmatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldName {
    // Required fields
    Id,
    Type,
    Name,

    // Optional metadata
    Description,

    // Upstream traceability (relationships)
    Refines,
    DerivesFrom,
    Satisfies,

    // Downstream traceability (relationships)
    IsRefinedBy,
    Derives,
    IsSatisfiedBy,

    // Peer relationships
    DependsOn,
    IsRequiredBy,

    // Requirement-specific fields
    Specification,
    Platform,
    JustifiedBy,

    // ADR-specific fields
    Status,
    Deciders,
    Justifies,
    Supersedes,
    SupersededBy,

    // Investigation upstream fields (author-declared + inverse of downstream)
    Parent,
    Cites,
    Evaluates,
    EstablishedBy,
    RaisedBy,
    PremiseOf,
    GapOf,
    HypothesisOf,
    AnalysisOf,

    // Investigation downstream fields (author-declared + inverse of upstream)
    Children,
    CitedBy,
    EvaluatedBy,
    Establishes,
    Raises,
    InvestigationPremises,
    InvestigationGaps,
    InvestigationHypotheses,
    InvestigationAnalyses,

    // Investigation peer fields
    Affects,
    AffectedBy,
}

impl FieldName {
    /// Returns all known field names.
    pub const fn all() -> &'static [FieldName] {
        &[
            Self::Id,
            Self::Type,
            Self::Name,
            Self::Description,
            Self::Refines,
            Self::DerivesFrom,
            Self::Satisfies,
            Self::IsRefinedBy,
            Self::Derives,
            Self::IsSatisfiedBy,
            Self::DependsOn,
            Self::IsRequiredBy,
            Self::Specification,
            Self::Platform,
            Self::JustifiedBy,
            Self::Status,
            Self::Deciders,
            Self::Justifies,
            Self::Supersedes,
            Self::SupersededBy,
            Self::Parent,
            Self::Cites,
            Self::Evaluates,
            Self::EstablishedBy,
            Self::RaisedBy,
            Self::PremiseOf,
            Self::GapOf,
            Self::HypothesisOf,
            Self::AnalysisOf,
            Self::Children,
            Self::CitedBy,
            Self::EvaluatedBy,
            Self::Establishes,
            Self::Raises,
            Self::InvestigationPremises,
            Self::InvestigationGaps,
            Self::InvestigationHypotheses,
            Self::InvestigationAnalyses,
            Self::Affects,
            Self::AffectedBy,
        ]
    }

    /// Returns the YAML field name (snake_case).
    ///
    /// Used for serialization, deserialization, and error messages.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Id => "id",
            Self::Type => "type",
            Self::Name => "name",
            Self::Description => "description",
            Self::Refines => "refines",
            Self::DerivesFrom => "derives_from",
            Self::Satisfies => "satisfies",
            Self::IsRefinedBy => "is_refined_by",
            Self::Derives => "derives",
            Self::IsSatisfiedBy => "is_satisfied_by",
            Self::DependsOn => "depends_on",
            Self::IsRequiredBy => "is_required_by",
            Self::Specification => "specification",
            Self::Platform => "platform",
            Self::JustifiedBy => "justified_by",
            Self::Status => "status",
            Self::Deciders => "deciders",
            Self::Justifies => "justifies",
            Self::Supersedes => "supersedes",
            Self::SupersededBy => "superseded_by",
            Self::Parent => "parent",
            Self::Cites => "cites",
            Self::Evaluates => "evaluates",
            Self::EstablishedBy => "established_by",
            Self::RaisedBy => "raised_by",
            Self::PremiseOf => "premise_of",
            Self::GapOf => "gap_of",
            Self::HypothesisOf => "hypothesis_of",
            Self::AnalysisOf => "analysis_of",
            Self::Children => "children",
            Self::CitedBy => "cited_by",
            Self::EvaluatedBy => "evaluated_by",
            Self::Establishes => "establishes",
            Self::Raises => "raises",
            Self::InvestigationPremises => "premises",
            Self::InvestigationGaps => "gaps",
            Self::InvestigationHypotheses => "hypotheses",
            Self::InvestigationAnalyses => "analyses",
            Self::Affects => "affects",
            Self::AffectedBy => "affected_by",
        }
    }

    /// Returns the human-readable display name.
    ///
    /// Used for user-facing output like change summaries.
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Id => "ID",
            Self::Type => "Type",
            Self::Name => "Name",
            Self::Description => "Description",
            Self::Refines => "Refines",
            Self::DerivesFrom => "Derives from",
            Self::Satisfies => "Satisfies",
            Self::IsRefinedBy => "Is refined by",
            Self::Derives => "Derives",
            Self::IsSatisfiedBy => "Is satisfied by",
            Self::DependsOn => "Depends on",
            Self::IsRequiredBy => "Is required by",
            Self::Specification => "Specification",
            Self::Platform => "Platform",
            Self::JustifiedBy => "Justified by",
            Self::Status => "Status",
            Self::Deciders => "Deciders",
            Self::Justifies => "Justifies",
            Self::Supersedes => "Supersedes",
            Self::SupersededBy => "Superseded by",
            Self::Parent => "Parent",
            Self::Cites => "Cites",
            Self::Evaluates => "Evaluates",
            Self::EstablishedBy => "Established by",
            Self::RaisedBy => "Raised by",
            Self::PremiseOf => "Premise of",
            Self::GapOf => "Gap of",
            Self::HypothesisOf => "Hypothesis of",
            Self::AnalysisOf => "Analysis of",
            Self::Children => "Children",
            Self::CitedBy => "Cited by",
            Self::EvaluatedBy => "Evaluated by",
            Self::Establishes => "Establishes",
            Self::Raises => "Raises",
            Self::InvestigationPremises => "Premises",
            Self::InvestigationGaps => "Gaps",
            Self::InvestigationHypotheses => "Hypotheses",
            Self::InvestigationAnalyses => "Analyses",
            Self::Affects => "Affects",
            Self::AffectedBy => "Affected by",
        }
    }

    /// Returns true if this is an upstream traceability field.
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
        )
    }

    /// Returns true if this is a downstream traceability field.
    pub const fn is_downstream(&self) -> bool {
        matches!(
            self,
            Self::IsRefinedBy
                | Self::Derives
                | Self::IsSatisfiedBy
                | Self::JustifiedBy
                | Self::Children
                | Self::CitedBy
                | Self::EvaluatedBy
                | Self::Establishes
                | Self::Raises
                | Self::InvestigationPremises
                | Self::InvestigationGaps
                | Self::InvestigationHypotheses
                | Self::InvestigationAnalyses
        )
    }

    /// Returns true if this is a peer relationship field.
    pub const fn is_peer(&self) -> bool {
        matches!(
            self,
            Self::DependsOn
                | Self::IsRequiredBy
                | Self::Supersedes
                | Self::SupersededBy
                | Self::Affects
                | Self::AffectedBy
        )
    }

    /// Returns true if this is a traceability field (upstream, downstream, or peer).
    pub const fn is_traceability(&self) -> bool {
        self.is_upstream() || self.is_downstream() || self.is_peer()
    }
}

impl std::fmt::Display for FieldName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_name_as_str() {
        assert_eq!(FieldName::Id.as_str(), "id");
        assert_eq!(FieldName::DerivesFrom.as_str(), "derives_from");
        assert_eq!(FieldName::Specification.as_str(), "specification");
    }

    #[test]
    fn test_field_name_is_upstream() {
        assert!(FieldName::Refines.is_upstream());
        assert!(FieldName::DerivesFrom.is_upstream());
        assert!(FieldName::Satisfies.is_upstream());
        assert!(!FieldName::IsRefinedBy.is_upstream());
        assert!(!FieldName::Specification.is_upstream());
    }

    #[test]
    fn test_field_name_is_downstream() {
        assert!(FieldName::IsRefinedBy.is_downstream());
        assert!(FieldName::Derives.is_downstream());
        assert!(FieldName::IsSatisfiedBy.is_downstream());
        assert!(!FieldName::Refines.is_downstream());
    }

    #[test]
    fn test_field_name_all() {
        let all = FieldName::all();
        assert!(all.contains(&FieldName::Id));
        assert!(all.contains(&FieldName::Refines));
        assert!(all.contains(&FieldName::Specification));
        assert!(all.contains(&FieldName::Status));
        assert!(all.contains(&FieldName::Justifies));
        assert_eq!(all.len(), 40);
    }

    #[test]
    fn test_field_name_display() {
        assert_eq!(format!("{}", FieldName::DerivesFrom), "derives_from");
    }

    #[test]
    fn test_investigation_field_names() {
        // Upstream investigation fields
        assert_eq!(FieldName::Parent.as_str(), "parent");
        assert_eq!(FieldName::Cites.as_str(), "cites");
        assert_eq!(FieldName::Evaluates.as_str(), "evaluates");
        assert_eq!(FieldName::EstablishedBy.as_str(), "established_by");
        assert_eq!(FieldName::RaisedBy.as_str(), "raised_by");
        assert_eq!(FieldName::PremiseOf.as_str(), "premise_of");
        assert_eq!(FieldName::GapOf.as_str(), "gap_of");
        assert_eq!(FieldName::HypothesisOf.as_str(), "hypothesis_of");
        assert_eq!(FieldName::AnalysisOf.as_str(), "analysis_of");

        // Downstream investigation fields
        assert_eq!(FieldName::Children.as_str(), "children");
        assert_eq!(FieldName::CitedBy.as_str(), "cited_by");
        assert_eq!(FieldName::EvaluatedBy.as_str(), "evaluated_by");
        assert_eq!(FieldName::Establishes.as_str(), "establishes");
        assert_eq!(FieldName::Raises.as_str(), "raises");
        assert_eq!(FieldName::InvestigationPremises.as_str(), "premises");
        assert_eq!(FieldName::InvestigationGaps.as_str(), "gaps");
        assert_eq!(FieldName::InvestigationHypotheses.as_str(), "hypotheses");
        assert_eq!(FieldName::InvestigationAnalyses.as_str(), "analyses");

        // Peer investigation fields
        assert_eq!(FieldName::Affects.as_str(), "affects");
        assert_eq!(FieldName::AffectedBy.as_str(), "affected_by");
    }

    #[test]
    fn test_investigation_field_directions() {
        assert!(FieldName::Parent.is_upstream());
        assert!(FieldName::Cites.is_upstream());
        assert!(FieldName::Evaluates.is_upstream());
        assert!(FieldName::EstablishedBy.is_upstream());
        assert!(FieldName::RaisedBy.is_upstream());
        assert!(FieldName::PremiseOf.is_upstream());
        assert!(FieldName::GapOf.is_upstream());
        assert!(FieldName::HypothesisOf.is_upstream());
        assert!(FieldName::AnalysisOf.is_upstream());

        assert!(FieldName::Children.is_downstream());
        assert!(FieldName::CitedBy.is_downstream());
        assert!(FieldName::EvaluatedBy.is_downstream());
        assert!(FieldName::Establishes.is_downstream());
        assert!(FieldName::Raises.is_downstream());
        assert!(FieldName::InvestigationPremises.is_downstream());
        assert!(FieldName::InvestigationGaps.is_downstream());
        assert!(FieldName::InvestigationHypotheses.is_downstream());
        assert!(FieldName::InvestigationAnalyses.is_downstream());

        assert!(FieldName::Affects.is_peer());
        assert!(FieldName::AffectedBy.is_peer());
    }
}
