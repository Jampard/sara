//! Markdown file parsing and document extraction.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::error::ParseError;
use crate::model::{
    AdrStatus, DepositionExchange, DownstreamRefs, EnvelopeDeposition, EnvelopeFlight,
    EnvelopeMessage, EnvelopeTransaction, Item, ItemBuilder, ItemId, ItemType, Participant,
    SourceLocation, UpstreamRefs,
};
use crate::parser::frontmatter::extract_frontmatter;

/// Raw frontmatter structure for deserialization.
///
/// This represents the YAML frontmatter as it appears in Markdown files.
/// All relationship fields accept both single values and arrays for flexibility.
#[derive(Debug, Clone, Deserialize)]
pub struct RawFrontmatter {
    /// Unique identifier (required).
    pub id: String,

    /// Item type (required).
    #[serde(rename = "type")]
    pub item_type: ItemType,

    /// Human-readable name (required).
    pub name: String,

    /// Description (optional).
    #[serde(default)]
    pub description: Option<String>,

    // Upstream references (toward Solution)
    /// Items this item refines (for UseCase, Scenario).
    #[serde(default)]
    pub refines: Vec<String>,

    /// Items this item derives from (for SystemRequirement, HW/SW Requirement).
    #[serde(default)]
    pub derives_from: Vec<String>,

    /// Items this item satisfies (for SystemArchitecture, HW/SW DetailedDesign).
    #[serde(default)]
    pub satisfies: Vec<String>,

    // Downstream references (toward Detailed Designs)
    /// Items that refine this item (for Solution, UseCase).
    #[serde(default)]
    pub is_refined_by: Vec<String>,

    /// Items derived from this item (for Scenario, SystemArchitecture).
    #[serde(default)]
    pub derives: Vec<String>,

    /// Items that satisfy this item (for SystemRequirement, HW/SW Requirement).
    #[serde(default)]
    pub is_satisfied_by: Vec<String>,

    // Type-specific attributes
    /// Specification statement (required for requirement types).
    #[serde(default)]
    pub specification: Option<String>,

    /// Peer dependencies (for requirement types).
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// Target platform (for SystemArchitecture).
    #[serde(default)]
    pub platform: Option<String>,

    /// ADR links (for SystemArchitecture, HW/SW DetailedDesign).
    #[serde(default)]
    pub justified_by: Option<Vec<String>>,

    /// ADR lifecycle status (required for ADR items).
    #[serde(default)]
    pub status: Option<AdrStatus>,

    /// ADR deciders (required for ADR items).
    #[serde(default)]
    pub deciders: Vec<String>,

    /// Design artifacts this ADR justifies (for ADR items).
    #[serde(default)]
    pub justifies: Vec<String>,

    /// Older ADRs this decision supersedes (for ADR items).
    #[serde(default)]
    pub supersedes: Vec<String>,

    // Investigation upstream fields (author-declared)
    #[serde(default)]
    pub parent: Vec<String>,
    #[serde(default)]
    pub cites: Vec<CiteEntry>,
    #[serde(default)]
    pub evaluates: Vec<String>,
    #[serde(default)]
    pub established_by: Vec<String>,
    #[serde(default)]
    pub raised_by: Vec<String>,

    // Investigation downstream fields (author-declared)
    #[serde(default)]
    pub premises: Vec<String>,
    #[serde(default)]
    pub gaps: Vec<String>,
    #[serde(default)]
    pub hypotheses: Vec<String>,
    #[serde(default)]
    pub analyses: Vec<String>,

    // Investigation downstream fields (auto-inferred inverses, author can also declare)
    #[serde(default)]
    pub children: Vec<String>,
    #[serde(default)]
    pub cited_by: Vec<String>,
    #[serde(default)]
    pub evaluated_by: Vec<String>,
    #[serde(default)]
    pub establishes: Vec<String>,
    #[serde(default)]
    pub raises: Vec<String>,

    // Investigation peer fields
    #[serde(default)]
    pub affects: Vec<String>,
    #[serde(default)]
    pub affected_by: Vec<String>,

    // Fingerprint/review fields
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub sourcing: Option<String>,
    #[serde(default)]
    pub relation: Option<String>,
    #[serde(default)]
    pub assessment: Option<String>,
    #[serde(default)]
    pub reviewed: Option<String>,
    #[serde(default)]
    pub stamps: HashMap<String, String>,

    // Per-item flags
    #[serde(default)]
    pub derived: bool,
    #[serde(default = "default_normative")]
    pub normative: bool,

    // N-ary participants
    #[serde(default)]
    pub participants: Vec<RawParticipant>,

    // Envelope fields (Evidence type)
    #[serde(default)]
    messages: Vec<RawMessage>,
    #[serde(default)]
    deposition: Option<RawDeposition>,
    #[serde(default)]
    flights: Vec<RawFlight>,
    #[serde(default)]
    transactions: Vec<RawTransaction>,
}

/// Accepts both old flat format (`"EVD-uid"`) and new structured format (`{evd: "EVD-uid", weight: "diagnostic"}`).
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum CiteEntry {
    Structured { evd: String, weight: String },
    Flat(String),
}

impl CiteEntry {
    pub fn uid(&self) -> &str {
        match self {
            CiteEntry::Structured { evd, .. } => evd,
            CiteEntry::Flat(s) => s,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawParticipant {
    pub entity: String,
    pub role: String,
}

/// Raw message for YAML deserialization (string UIDs before conversion to ItemId).
#[derive(Debug, Clone, Deserialize)]
struct RawMessage {
    id: i64,
    from: String,
    to: Vec<String>,
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    subject: Option<String>,
    #[serde(default)]
    cc: Option<Vec<String>>,
    #[serde(default)]
    bcc: Option<Vec<String>>,
    #[serde(default)]
    forward: Option<bool>,
    #[serde(default)]
    removed: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawDepoExchange {
    id: i64,
    speaker: String,
    #[serde(default)]
    page: Option<i64>,
    #[serde(default)]
    objection: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawDeposition {
    witness: String,
    date: String,
    proceeding: String,
    exchanges: Vec<RawDepoExchange>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawFlight {
    id: i64,
    date: String,
    origin: String,
    destination: String,
    #[serde(default)]
    aircraft: Option<String>,
    passengers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawTransaction {
    id: i64,
    date: String,
    from: String,
    to: String,
    amount: f64,
    currency: String,
    #[serde(default)]
    method: Option<String>,
}

fn default_normative() -> bool {
    true
}

impl RawFrontmatter {
    /// Converts string IDs to ItemIds for upstream refs.
    pub fn upstream_refs(&self) -> Result<UpstreamRefs, ParseError> {
        Ok(UpstreamRefs {
            refines: self.refines.iter().map(ItemId::new_unchecked).collect(),
            derives_from: self
                .derives_from
                .iter()
                .map(ItemId::new_unchecked)
                .collect(),
            satisfies: self.satisfies.iter().map(ItemId::new_unchecked).collect(),
            justifies: self.justifies.iter().map(ItemId::new_unchecked).collect(),
            parent: self.parent.iter().map(ItemId::new_unchecked).collect(),
            cites: self
                .cites
                .iter()
                .map(|c| ItemId::new_unchecked(c.uid()))
                .collect(),
            evaluates: self.evaluates.iter().map(ItemId::new_unchecked).collect(),
            established_by: self
                .established_by
                .iter()
                .map(ItemId::new_unchecked)
                .collect(),
            raised_by: self.raised_by.iter().map(ItemId::new_unchecked).collect(),
            premise_of: Vec::new(),
            gap_of: Vec::new(),
            hypothesis_of: Vec::new(),
            analysis_of: Vec::new(),
            affects: self.affects.iter().map(ItemId::new_unchecked).collect(),
        })
    }

    /// Converts string IDs to ItemIds for downstream refs.
    pub fn downstream_refs(&self) -> Result<DownstreamRefs, ParseError> {
        Ok(DownstreamRefs {
            is_refined_by: self
                .is_refined_by
                .iter()
                .map(ItemId::new_unchecked)
                .collect(),
            derives: self.derives.iter().map(ItemId::new_unchecked).collect(),
            is_satisfied_by: self
                .is_satisfied_by
                .iter()
                .map(ItemId::new_unchecked)
                .collect(),
            justified_by: self
                .justified_by
                .as_ref()
                .map(|ids| ids.iter().map(ItemId::new_unchecked).collect())
                .unwrap_or_default(),
            children: self.children.iter().map(ItemId::new_unchecked).collect(),
            cited_by: self.cited_by.iter().map(ItemId::new_unchecked).collect(),
            evaluated_by: self
                .evaluated_by
                .iter()
                .map(ItemId::new_unchecked)
                .collect(),
            establishes: self.establishes.iter().map(ItemId::new_unchecked).collect(),
            raises: self.raises.iter().map(ItemId::new_unchecked).collect(),
            premises: self.premises.iter().map(ItemId::new_unchecked).collect(),
            gaps: self.gaps.iter().map(ItemId::new_unchecked).collect(),
            hypotheses: self.hypotheses.iter().map(ItemId::new_unchecked).collect(),
            analyses: self.analyses.iter().map(ItemId::new_unchecked).collect(),
            affected_by: self.affected_by.iter().map(ItemId::new_unchecked).collect(),
        })
    }
}

/// Parses a Markdown file and extracts the item.
///
/// # Arguments
/// * `content` - The raw file content.
/// * `file_path` - Relative path within the repository.
/// * `repository` - Absolute path to the repository root.
///
/// # Returns
/// The parsed Item, or a ParseError if parsing fails.
pub fn parse_markdown_file(
    content: &str,
    file_path: &Path,
    repository: &Path,
) -> Result<Item, ParseError> {
    let extracted = extract_frontmatter(content, file_path)?;

    let frontmatter: RawFrontmatter =
        serde_yaml::from_str(&extracted.yaml).map_err(|e| ParseError::InvalidYaml {
            file: file_path.to_path_buf(),
            reason: e.to_string(),
        })?;

    // Validate item ID format
    let item_id = ItemId::new(&frontmatter.id).map_err(|e| ParseError::InvalidFrontmatter {
        file: file_path.to_path_buf(),
        reason: format!("Invalid item ID: {}", e),
    })?;

    // Create source location
    let source = SourceLocation::new(repository, file_path);

    // Build the item
    let mut builder = ItemBuilder::new()
        .id(item_id)
        .item_type(frontmatter.item_type)
        .name(&frontmatter.name)
        .source(source)
        .upstream(frontmatter.upstream_refs()?)
        .downstream(frontmatter.downstream_refs()?);

    if let Some(desc) = &frontmatter.description {
        builder = builder.description(desc);
    }

    // Set type-specific attributes based on item type
    match frontmatter.item_type {
        ItemType::Solution | ItemType::UseCase | ItemType::Scenario => {
            // No type-specific attributes
        }
        ItemType::SystemRequirement
        | ItemType::SoftwareRequirement
        | ItemType::HardwareRequirement => {
            if let Some(spec) = &frontmatter.specification {
                builder = builder.specification(spec);
            }
            for id in &frontmatter.depends_on {
                builder = builder.depends_on(ItemId::new_unchecked(id));
            }
        }
        ItemType::SystemArchitecture => {
            if let Some(platform) = &frontmatter.platform {
                builder = builder.platform(platform);
            }
            // justified_by is now handled via downstream_refs()
        }
        ItemType::SoftwareDetailedDesign | ItemType::HardwareDetailedDesign => {
            // justified_by is now handled via downstream_refs()
        }
        ItemType::ArchitectureDecisionRecord => {
            if let Some(status) = frontmatter.status {
                builder = builder.status(status);
            }
            builder = builder.deciders(frontmatter.deciders.clone());
            // justifies is now handled via upstream_refs()
            builder = builder.supersedes_all(
                frontmatter
                    .supersedes
                    .iter()
                    .map(ItemId::new_unchecked)
                    .collect(),
            );
        }
        // Investigation types — type-specific attribute fields
        ItemType::Entity
        | ItemType::Thesis
        | ItemType::Premise
        | ItemType::Question
        | ItemType::Block => {}
        ItemType::Evidence => {
            if let Some(ref s) = frontmatter.sourcing {
                builder = builder.sourcing(s.clone());
            }
            if let Some(ref r) = frontmatter.relation {
                builder = builder.relation(r.clone());
            }
            // Convert raw envelope types to domain types
            if !frontmatter.messages.is_empty() {
                let messages = frontmatter
                    .messages
                    .iter()
                    .map(|m| EnvelopeMessage {
                        id: m.id,
                        from: ItemId::new_unchecked(&m.from),
                        to: m.to.iter().map(ItemId::new_unchecked).collect(),
                        date: m.date.clone(),
                        subject: m.subject.clone(),
                        cc: m
                            .cc
                            .as_ref()
                            .map(|v| v.iter().map(ItemId::new_unchecked).collect()),
                        bcc: m
                            .bcc
                            .as_ref()
                            .map(|v| v.iter().map(ItemId::new_unchecked).collect()),
                        forward: m.forward,
                        removed: m
                            .removed
                            .as_ref()
                            .map(|v| v.iter().map(ItemId::new_unchecked).collect()),
                    })
                    .collect();
                builder = builder.envelope_messages(messages);
            }
            if let Some(ref d) = frontmatter.deposition {
                let depo = EnvelopeDeposition {
                    witness: ItemId::new_unchecked(&d.witness),
                    date: d.date.clone(),
                    proceeding: d.proceeding.clone(),
                    exchanges: d
                        .exchanges
                        .iter()
                        .map(|ex| DepositionExchange {
                            id: ex.id,
                            speaker: ItemId::new_unchecked(&ex.speaker),
                            page: ex.page,
                            objection: ex.objection,
                        })
                        .collect(),
                };
                builder = builder.envelope_deposition(depo);
            }
            if !frontmatter.flights.is_empty() {
                let flights = frontmatter
                    .flights
                    .iter()
                    .map(|f| EnvelopeFlight {
                        id: f.id,
                        date: f.date.clone(),
                        origin: ItemId::new_unchecked(&f.origin),
                        destination: ItemId::new_unchecked(&f.destination),
                        aircraft: f.aircraft.as_ref().map(ItemId::new_unchecked),
                        passengers: f.passengers.iter().map(ItemId::new_unchecked).collect(),
                    })
                    .collect();
                builder = builder.envelope_flights(flights);
            }
            if !frontmatter.transactions.is_empty() {
                let transactions = frontmatter
                    .transactions
                    .iter()
                    .map(|t| EnvelopeTransaction {
                        id: t.id,
                        date: t.date.clone(),
                        from: ItemId::new_unchecked(&t.from),
                        to: ItemId::new_unchecked(&t.to),
                        amount: t.amount,
                        currency: t.currency.clone(),
                        method: t.method.clone(),
                    })
                    .collect();
                builder = builder.envelope_transactions(transactions);
            }
        }
        ItemType::Analysis | ItemType::Hypothesis => {
            if let Some(ref a) = frontmatter.assessment {
                builder = builder.assessment(a.clone());
            }
        }
    }

    // Set fingerprint/review fields
    if let Some(ref outcome) = frontmatter.outcome {
        builder = builder.outcome(outcome.clone());
    }
    if let Some(ref reviewed) = frontmatter.reviewed {
        builder = builder.reviewed(reviewed.clone());
    }
    if !frontmatter.stamps.is_empty() {
        let stamps: HashMap<ItemId, String> = frontmatter
            .stamps
            .iter()
            .map(|(k, v)| (ItemId::new_unchecked(k), v.clone()))
            .collect();
        builder = builder.stamps(stamps);
    }

    // Set per-item flags
    if frontmatter.derived {
        builder = builder.derived(true);
    }
    if !frontmatter.normative {
        builder = builder.normative(false);
    }

    // Set participants
    if !frontmatter.participants.is_empty() {
        let participants = frontmatter
            .participants
            .iter()
            .map(|p| Participant {
                entity: ItemId::new_unchecked(&p.entity),
                role: p.role.clone(),
            })
            .collect();
        builder = builder.participants(participants);
    }

    // Extract raw YAML field keys for deprecated field detection
    if let Ok(serde_yaml::Value::Mapping(mapping)) = serde_yaml::from_str(&extracted.yaml) {
        let keys: Vec<String> = mapping
            .keys()
            .filter_map(|k| k.as_str().map(String::from))
            .collect();
        builder = builder.raw_field_keys(keys);
    }

    // Compute body_hash from body content
    let body = &extracted.body;
    let trimmed_body = body.trim();
    if !trimmed_body.is_empty() {
        let hash = Sha256::digest(trimmed_body.as_bytes());
        builder = builder.body_hash(format!("{:x}", hash));
    }

    builder.build().map_err(|e| ParseError::InvalidFrontmatter {
        file: file_path.to_path_buf(),
        reason: e.to_string(),
    })
}

/// Represents a parsed document with its item and body content.
#[derive(Debug)]
pub struct ParsedDocument {
    /// The extracted item.
    pub item: Item,
    /// The Markdown body content after frontmatter.
    pub body: String,
}

/// Parses a Markdown file and returns the item and body.
pub fn parse_document(
    content: &str,
    file_path: &Path,
    repository: &Path,
) -> Result<ParsedDocument, ParseError> {
    let extracted = extract_frontmatter(content, file_path)?;
    let item = parse_markdown_file(content, file_path, repository)?;

    Ok(ParsedDocument {
        item,
        body: extracted.body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const SOLUTION_MD: &str = r#"---
id: "SOL-001"
type: solution
name: "Test Solution"
description: "A test solution"
is_refined_by:
  - "UC-001"
---
# Test Solution

This is the body content.
"#;

    const REQUIREMENT_MD: &str = r#"---
id: "SYSREQ-001"
type: system_requirement
name: "Performance Requirement"
specification: "The system SHALL respond within 100ms."
derives_from:
  - "SCEN-001"
is_satisfied_by:
  - "SYSARCH-001"
---
# Requirement
"#;

    #[test]
    fn test_parse_solution() {
        let item = parse_markdown_file(
            SOLUTION_MD,
            &PathBuf::from("SOL-001.md"),
            &PathBuf::from("/repo"),
        )
        .unwrap();

        assert_eq!(item.id.as_str(), "SOL-001");
        assert_eq!(item.item_type, ItemType::Solution);
        assert_eq!(item.name, "Test Solution");
        assert_eq!(item.description, Some("A test solution".to_string()));
        assert_eq!(item.downstream.is_refined_by.len(), 1);
        assert_eq!(item.downstream.is_refined_by[0].as_str(), "UC-001");
    }

    #[test]
    fn test_parse_requirement() {
        let item = parse_markdown_file(
            REQUIREMENT_MD,
            &PathBuf::from("SYSREQ-001.md"),
            &PathBuf::from("/repo"),
        )
        .unwrap();

        assert_eq!(item.id.as_str(), "SYSREQ-001");
        assert_eq!(item.item_type, ItemType::SystemRequirement);
        assert_eq!(
            item.attributes.specification().map(String::as_str),
            Some("The system SHALL respond within 100ms.")
        );
        assert_eq!(item.upstream.derives_from.len(), 1);
        assert_eq!(item.downstream.is_satisfied_by.len(), 1);
    }

    #[test]
    fn test_parse_document() {
        let doc = parse_document(
            SOLUTION_MD,
            &PathBuf::from("SOL-001.md"),
            &PathBuf::from("/repo"),
        )
        .unwrap();

        assert_eq!(doc.item.id.as_str(), "SOL-001");
        assert!(doc.body.contains("# Test Solution"));
    }

    #[test]
    fn test_parse_invalid_id() {
        let content = r#"---
id: "invalid id with spaces"
type: solution
name: "Test"
---
"#;
        let result =
            parse_markdown_file(content, &PathBuf::from("test.md"), &PathBuf::from("/repo"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_type() {
        let content = r#"---
id: "SOL-001"
name: "Test"
---
"#;
        let result =
            parse_markdown_file(content, &PathBuf::from("test.md"), &PathBuf::from("/repo"));
        assert!(result.is_err());
    }

    const ADR_MD: &str = r#"---
id: "ADR-001"
type: architecture_decision_record
name: "Use Microservices Architecture"
description: "Decision to adopt microservices"
status: proposed
deciders:
  - "Alice Smith"
  - "Bob Jones"
justifies:
  - "SYSARCH-001"
  - "SWDD-001"
supersedes: []
superseded_by: null
---
# Architecture Decision: Use Microservices Architecture

## Context and problem statement

We need to choose an architecture pattern for our system.

## Decision Outcome

Chosen option: Microservices, because it provides better scalability.
"#;

    #[test]
    fn test_parse_adr() {
        let item = parse_markdown_file(
            ADR_MD,
            &PathBuf::from("ADR-001.md"),
            &PathBuf::from("/repo"),
        )
        .unwrap();

        assert_eq!(item.id.as_str(), "ADR-001");
        assert_eq!(item.item_type, ItemType::ArchitectureDecisionRecord);
        assert_eq!(item.name, "Use Microservices Architecture");
        assert_eq!(
            item.description,
            Some("Decision to adopt microservices".to_string())
        );

        // Check ADR-specific attributes
        assert_eq!(item.attributes.status(), Some(AdrStatus::Proposed));
        assert_eq!(item.attributes.deciders().len(), 2);
        assert!(
            item.attributes
                .deciders()
                .contains(&"Alice Smith".to_string())
        );
        assert!(
            item.attributes
                .deciders()
                .contains(&"Bob Jones".to_string())
        );
        // justifies is now in upstream refs
        assert_eq!(item.upstream.justifies.len(), 2);
        assert_eq!(item.upstream.justifies[0].as_str(), "SYSARCH-001");
        assert_eq!(item.upstream.justifies[1].as_str(), "SWDD-001");
        assert!(item.attributes.supersedes().is_empty());
    }

    #[test]
    fn test_parse_adr_missing_deciders() {
        let content = r#"---
id: "ADR-002"
type: architecture_decision_record
name: "Test Decision"
status: proposed
---
"#;
        let result =
            parse_markdown_file(content, &PathBuf::from("test.md"), &PathBuf::from("/repo"));
        // Should fail because deciders is required for ADRs
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_adr_missing_status() {
        let content = r#"---
id: "ADR-003"
type: architecture_decision_record
name: "Test Decision"
deciders:
  - "Alice"
---
"#;
        let result =
            parse_markdown_file(content, &PathBuf::from("test.md"), &PathBuf::from("/repo"));
        // Should fail because status is required for ADRs
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_adr_with_supersession() {
        let content = r#"---
id: "ADR-005"
type: architecture_decision_record
name: "Updated Architecture Decision"
status: accepted
deciders:
  - "Alice Smith"
justifies:
  - "SYSARCH-001"
supersedes:
  - "ADR-001"
  - "ADR-002"
---
"#;
        let item = parse_markdown_file(
            content,
            &PathBuf::from("ADR-005.md"),
            &PathBuf::from("/repo"),
        )
        .unwrap();

        // justifies is now in upstream refs
        assert_eq!(item.upstream.justifies.len(), 1);
        assert_eq!(item.upstream.justifies[0].as_str(), "SYSARCH-001");
        // supersedes is still in attributes
        assert_eq!(item.attributes.supersedes().len(), 2);
        assert_eq!(item.attributes.supersedes()[0].as_str(), "ADR-001");
        assert_eq!(item.attributes.supersedes()[1].as_str(), "ADR-002");
    }

    #[test]
    fn test_parse_evidence_item() {
        let content = r#"---
id: "EVD-001"
type: evidence
name: "Test Evidence"
description: "An evidence claim"
parent:
  - "ITM-001"
---

Evidence body text.
"#;
        let item = parse_markdown_file(
            content,
            &PathBuf::from("EVD-001.md"),
            &PathBuf::from("/repo"),
        )
        .unwrap();

        assert_eq!(item.item_type, ItemType::Evidence);
        assert_eq!(item.id.as_str(), "EVD-001");
        assert_eq!(item.upstream.parent.len(), 1);
        assert_eq!(item.upstream.parent[0].as_str(), "ITM-001");
    }

    #[test]
    fn test_parse_analysis_with_multiple_relations() {
        let content = r#"---
id: "ANL-001"
type: analysis
name: "Test Analysis"
parent:
  - "THS-001"
cites:
  - "EVD-001"
  - "EVD-002"
evaluates:
  - "HYP-001"
premises:
  - "PRM-001"
gaps:
  - "QST-001"
---

Analysis body.
"#;
        let item = parse_markdown_file(
            content,
            &PathBuf::from("ANL-001.md"),
            &PathBuf::from("/repo"),
        )
        .unwrap();

        assert_eq!(item.item_type, ItemType::Analysis);
        assert_eq!(item.upstream.parent.len(), 1);
        assert_eq!(item.upstream.cites.len(), 2);
        assert_eq!(item.upstream.evaluates.len(), 1);
        assert_eq!(item.downstream.premises.len(), 1);
        assert_eq!(item.downstream.gaps.len(), 1);
    }

    #[test]
    fn test_parse_structured_cites() {
        let content = r#"---
id: "ANL-002"
type: analysis
name: "Structured Cites Analysis"
parent:
  - "THS-001"
cites:
  - evd: "EVD-001"
    weight: "diagnostic"
  - evd: "EVD-002"
    weight: "consistent"
evaluates:
  - "HYP-001"
---

Analysis with structured cites.
"#;
        let item = parse_markdown_file(
            content,
            &PathBuf::from("ANL-002.md"),
            &PathBuf::from("/repo"),
        )
        .unwrap();

        assert_eq!(item.item_type, ItemType::Analysis);
        assert_eq!(item.upstream.cites.len(), 2);
        assert_eq!(item.upstream.cites[0].as_str(), "EVD-001");
        assert_eq!(item.upstream.cites[1].as_str(), "EVD-002");
    }

    #[test]
    fn test_parse_mixed_cites_formats() {
        let content = r#"---
id: "ANL-003"
type: analysis
name: "Mixed Cites Analysis"
parent:
  - "THS-001"
cites:
  - "EVD-001"
  - evd: "EVD-002"
    weight: "diagnostic"
evaluates:
  - "HYP-001"
---

Analysis with mixed cites formats.
"#;
        let item = parse_markdown_file(
            content,
            &PathBuf::from("ANL-003.md"),
            &PathBuf::from("/repo"),
        )
        .unwrap();

        assert_eq!(item.upstream.cites.len(), 2);
        assert_eq!(item.upstream.cites[0].as_str(), "EVD-001");
        assert_eq!(item.upstream.cites[1].as_str(), "EVD-002");
    }

    #[test]
    fn test_parse_evidence_with_outcome_and_sourcing() {
        let content = r#"---
id: "EVD-010"
type: evidence
name: "Test Evidence"
outcome: "open"
sourcing: "C"
relation: "hosted"
parent:
  - "ITM-001"
---
# Evidence body
"#;
        let item = parse_markdown_file(
            content,
            &PathBuf::from("EVD-010.md"),
            &PathBuf::from("/repo"),
        )
        .unwrap();

        assert_eq!(item.outcome.as_deref(), Some("open"));
        assert_eq!(item.attributes.sourcing(), Some("C"));
        assert_eq!(item.attributes.evidence_relation(), Some("hosted"));
    }

    #[test]
    fn test_parse_analysis_with_assessment() {
        let content = r#"---
id: "ANL-010"
type: analysis
name: "Test Analysis"
outcome: "open"
assessment: "very-likely"
parent:
  - "THS-001"
---
# Analysis body
"#;
        let item = parse_markdown_file(
            content,
            &PathBuf::from("ANL-010.md"),
            &PathBuf::from("/repo"),
        )
        .unwrap();

        assert_eq!(item.outcome.as_deref(), Some("open"));
        assert_eq!(item.attributes.assessment(), Some("very-likely"));
    }

    #[test]
    fn test_parse_item_with_stamps_and_reviewed() {
        let content = r#"---
id: "ANL-011"
type: analysis
name: "Test Analysis"
reviewed: "deadbeef"
stamps:
  EVD-001: "abcd1234"
  HYP-001: "ef567890"
parent:
  - "THS-001"
---
# Analysis body
"#;
        let item = parse_markdown_file(
            content,
            &PathBuf::from("ANL-011.md"),
            &PathBuf::from("/repo"),
        )
        .unwrap();

        assert_eq!(item.reviewed.as_deref(), Some("deadbeef"));
        assert_eq!(item.stamps.len(), 2);
    }

    #[test]
    fn test_parse_body_hash_computed() {
        let content = r#"---
id: "EVD-020"
type: evidence
name: "Test"
parent:
  - "ITM-001"
---
# Some body content
With multiple lines.
"#;
        let item = parse_markdown_file(
            content,
            &PathBuf::from("EVD-020.md"),
            &PathBuf::from("/repo"),
        )
        .unwrap();

        assert!(item.body_hash.is_some());
        assert!(!item.body_hash.as_ref().unwrap().is_empty());
        assert_eq!(item.body_hash.as_ref().unwrap().len(), 64); // SHA-256 hex
    }

    #[test]
    fn test_parse_block_with_affects() {
        let content = r#"---
id: "BLK-001"
type: block
name: "Test Block"
affects:
  - "EVD-001"
  - "ANL-001"
---
"#;
        let item = parse_markdown_file(
            content,
            &PathBuf::from("BLK-001.md"),
            &PathBuf::from("/repo"),
        )
        .unwrap();

        assert_eq!(item.item_type, ItemType::Block);
        assert_eq!(item.upstream.affects.len(), 2);
    }

    #[test]
    fn test_parse_evidence_with_messages_envelope() {
        let content = r#"---
id: "EVD-msg"
type: evidence
name: "Email Thread"
sourcing: "forensic"
participants:
  - entity: "ITM-alice"
    role: "sender"
  - entity: "ITM-bob"
    role: "recipient"
messages:
  - id: 1
    from: "ITM-alice"
    to:
      - "ITM-bob"
    date: "2024-01-15"
    subject: "Meeting"
    cc:
      - "ITM-carol"
  - id: 2
    from: "ITM-bob"
    to:
      - "ITM-alice"
---
Email body.
"#;
        let item = parse_markdown_file(
            content,
            &PathBuf::from("EVD-msg.mdx"),
            &PathBuf::from("/repo"),
        )
        .unwrap();

        assert_eq!(item.item_type, ItemType::Evidence);
        let msgs = item.attributes.messages();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].from.as_str(), "ITM-alice");
        assert_eq!(msgs[0].to[0].as_str(), "ITM-bob");
        assert_eq!(msgs[0].subject.as_deref(), Some("Meeting"));
        assert_eq!(msgs[0].cc.as_ref().unwrap()[0].as_str(), "ITM-carol");
        assert_eq!(msgs[1].from.as_str(), "ITM-bob");
    }

    #[test]
    fn test_parse_evidence_with_flights_envelope() {
        let content = r#"---
id: "EVD-flt"
type: evidence
name: "Flight Records"
participants:
  - entity: "ITM-alice"
    role: "passenger"
  - entity: "ITM-loc-a"
    role: "location"
flights:
  - id: 1
    date: "2024-02-10"
    origin: "ITM-loc-a"
    destination: "ITM-loc-b"
    aircraft: "ITM-plane"
    passengers:
      - "ITM-alice"
---
"#;
        let item = parse_markdown_file(
            content,
            &PathBuf::from("EVD-flt.md"),
            &PathBuf::from("/repo"),
        )
        .unwrap();

        let flights = item.attributes.flights();
        assert_eq!(flights.len(), 1);
        assert_eq!(flights[0].origin.as_str(), "ITM-loc-a");
        assert_eq!(flights[0].destination.as_str(), "ITM-loc-b");
        assert_eq!(flights[0].aircraft.as_ref().unwrap().as_str(), "ITM-plane");
        assert_eq!(flights[0].passengers.len(), 1);
    }

    #[test]
    fn test_parse_evidence_with_transactions_envelope() {
        let content = r#"---
id: "EVD-txn"
type: evidence
name: "Financial Records"
participants:
  - entity: "ITM-alice"
    role: "payer"
  - entity: "ITM-bob"
    role: "payee"
transactions:
  - id: 1
    date: "2024-03-01"
    from: "ITM-alice"
    to: "ITM-bob"
    amount: 5000.00
    currency: "USD"
    method: "wire"
---
"#;
        let item = parse_markdown_file(
            content,
            &PathBuf::from("EVD-txn.md"),
            &PathBuf::from("/repo"),
        )
        .unwrap();

        let txns = item.attributes.transactions();
        assert_eq!(txns.len(), 1);
        assert_eq!(txns[0].from.as_str(), "ITM-alice");
        assert_eq!(txns[0].to.as_str(), "ITM-bob");
        assert!((txns[0].amount - 5000.0).abs() < f64::EPSILON);
        assert_eq!(txns[0].currency, "USD");
        assert_eq!(txns[0].method.as_deref(), Some("wire"));
    }

    #[test]
    fn test_parse_evidence_with_deposition_envelope() {
        let content = r#"---
id: "EVD-dep"
type: evidence
name: "Deposition Transcript"
participants:
  - entity: "ITM-alice"
    role: "witness"
  - entity: "ITM-bob"
    role: "examiner"
deposition:
  witness: "ITM-alice"
  date: "2024-04-01"
  proceeding: "Case No. 2024-001"
  exchanges:
    - id: 1
      speaker: "ITM-bob"
      page: 5
    - id: 2
      speaker: "ITM-alice"
      page: 5
      objection: true
---
"#;
        let item = parse_markdown_file(
            content,
            &PathBuf::from("EVD-dep.md"),
            &PathBuf::from("/repo"),
        )
        .unwrap();

        let depo = item.attributes.deposition().unwrap();
        assert_eq!(depo.witness.as_str(), "ITM-alice");
        assert_eq!(depo.proceeding, "Case No. 2024-001");
        assert_eq!(depo.exchanges.len(), 2);
        assert_eq!(depo.exchanges[0].speaker.as_str(), "ITM-bob");
        assert_eq!(depo.exchanges[0].page, Some(5));
        assert_eq!(depo.exchanges[1].objection, Some(true));
    }
}
