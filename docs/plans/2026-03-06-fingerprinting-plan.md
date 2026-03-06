# Fingerprinting & Suspect Links Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add per-link fingerprint stamps, suspect link detection, and review tracking to SARA (Change Request sections 4+5).

**Architecture:** SHA-256 fingerprints computed from item UID + body + configured frontmatter fields. Stamps stored in each item's YAML frontmatter as a target→hash map. New `SuspectLinksRule` validation rule compares stamps against current fingerprints. Two new CLI commands (`sara review`, `sara clear`) for managing review state.

**Tech Stack:** Rust 2024, sha2 crate, serde/serde_yaml, clap, petgraph (existing)

**Design doc:** `docs/plans/2026-03-06-fingerprinting-design.md`

---

## Batch 1: Model layer (Tasks 0–2)

### Task 0: Add sha2 dependency

**Files:**
- Modify: `sara-core/Cargo.toml`

**Step 1: Add sha2 to dependencies**

In `sara-core/Cargo.toml`, add to `[dependencies]`:

```toml
sha2 = "0.10"
```

**Step 2: Verify it compiles**

Run: `cargo build -p sara-core`
Expected: compiles without errors

**Step 3: Commit**

```
chore: add sha2 dependency for fingerprint hashing
```

---

### Task 1: Add new fields to Item, ItemAttributes, and ItemBuilder

**Files:**
- Modify: `sara-core/src/model/item.rs`
- Test: inline `#[cfg(test)] mod tests`

**Step 1: Write failing tests**

Add to the existing test module in `item.rs`:

```rust
#[test]
fn test_item_outcome_field() {
    let source = SourceLocation::new(PathBuf::from("/test"), "test.md".to_string());
    let item = ItemBuilder::new()
        .id(ItemId::new_unchecked("EVD-001"))
        .item_type(ItemType::Evidence)
        .name("Test")
        .source(source)
        .outcome("open".to_string())
        .build()
        .unwrap();
    assert_eq!(item.outcome.as_deref(), Some("open"));
}

#[test]
fn test_item_stamps_field() {
    let source = SourceLocation::new(PathBuf::from("/test"), "test.md".to_string());
    let item = ItemBuilder::new()
        .id(ItemId::new_unchecked("ANL-001"))
        .item_type(ItemType::Analysis)
        .name("Test")
        .source(source)
        .stamps(HashMap::from([
            (ItemId::new_unchecked("EVD-001"), "abcd1234".to_string()),
        ]))
        .build()
        .unwrap();
    assert_eq!(item.stamps.len(), 1);
    assert_eq!(
        item.stamps.get(&ItemId::new_unchecked("EVD-001")).unwrap(),
        "abcd1234"
    );
}

#[test]
fn test_item_reviewed_field() {
    let source = SourceLocation::new(PathBuf::from("/test"), "test.md".to_string());
    let item = ItemBuilder::new()
        .id(ItemId::new_unchecked("ANL-001"))
        .item_type(ItemType::Analysis)
        .name("Test")
        .source(source)
        .reviewed("deadbeef".to_string())
        .build()
        .unwrap();
    assert_eq!(item.reviewed.as_deref(), Some("deadbeef"));
}

#[test]
fn test_item_body_hash_field() {
    let source = SourceLocation::new(PathBuf::from("/test"), "test.md".to_string());
    let item = ItemBuilder::new()
        .id(ItemId::new_unchecked("EVD-001"))
        .item_type(ItemType::Evidence)
        .name("Test")
        .source(source)
        .body_hash("abc123".to_string())
        .build()
        .unwrap();
    assert_eq!(item.body_hash.as_deref(), Some("abc123"));
}

#[test]
fn test_evidence_attributes_with_fields() {
    let attrs = ItemAttributes::Evidence {
        sourcing: Some("C".to_string()),
        relation: Some("hosted".to_string()),
    };
    assert_eq!(attrs.sourcing(), Some("C"));
    assert_eq!(attrs.evidence_relation(), Some("hosted"));
}

#[test]
fn test_analysis_attributes_with_assessment() {
    let attrs = ItemAttributes::Analysis {
        assessment: Some("very-likely".to_string()),
    };
    assert_eq!(attrs.assessment(), Some("very-likely"));
}

#[test]
fn test_hypothesis_attributes_with_assessment() {
    let attrs = ItemAttributes::Hypothesis {
        assessment: Some("roughly-even".to_string()),
    };
    assert_eq!(attrs.assessment(), Some("roughly-even"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p sara-core item::tests::test_item_outcome`
Expected: FAIL — no `outcome` method on `ItemBuilder`

**Step 3: Implement the changes**

On the `Item` struct (~line 940), add 4 new fields:

```rust
pub outcome: Option<String>,
pub reviewed: Option<String>,
pub stamps: HashMap<ItemId, String>,
pub body_hash: Option<String>,
```

Add `use std::collections::HashMap;` if not already imported.

On `ItemAttributes`, change the 3 unit variants to struct variants:

```rust
#[serde(rename = "evidence")]
Evidence {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sourcing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    relation: Option<String>,
},

#[serde(rename = "analysis")]
Analysis {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    assessment: Option<String>,
},

#[serde(rename = "hypothesis")]
Hypothesis {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    assessment: Option<String>,
},
```

Add accessor methods on `ItemAttributes`:

```rust
pub fn sourcing(&self) -> Option<&str> {
    match self {
        Self::Evidence { sourcing, .. } => sourcing.as_deref(),
        _ => None,
    }
}

pub fn evidence_relation(&self) -> Option<&str> {
    match self {
        Self::Evidence { relation, .. } => relation.as_deref(),
        _ => None,
    }
}

pub fn assessment(&self) -> Option<&str> {
    match self {
        Self::Analysis { assessment, .. } | Self::Hypothesis { assessment, .. } => {
            assessment.as_deref()
        }
        _ => None,
    }
}
```

On `ItemBuilder` (~line 991), add fields:

```rust
outcome: Option<String>,
reviewed: Option<String>,
stamps: Option<HashMap<ItemId, String>>,
body_hash: Option<String>,
sourcing: Option<String>,
relation: Option<String>,
assessment: Option<String>,
```

Add setter methods:

```rust
pub fn outcome(mut self, outcome: String) -> Self {
    self.outcome = Some(outcome);
    self
}
pub fn reviewed(mut self, reviewed: String) -> Self {
    self.reviewed = Some(reviewed);
    self
}
pub fn stamps(mut self, stamps: HashMap<ItemId, String>) -> Self {
    self.stamps = Some(stamps);
    self
}
pub fn body_hash(mut self, body_hash: String) -> Self {
    self.body_hash = Some(body_hash);
    self
}
pub fn sourcing(mut self, sourcing: String) -> Self {
    self.sourcing = Some(sourcing);
    self
}
pub fn relation(mut self, relation: String) -> Self {
    self.relation = Some(relation);
    self
}
pub fn assessment(mut self, assessment: String) -> Self {
    self.assessment = Some(assessment);
    self
}
```

In `build()` (~line 1233), add the new fields to `Item` construction and update `ItemAttributes` construction for Evidence/Analysis/Hypothesis:

```rust
outcome: self.outcome,
reviewed: self.reviewed,
stamps: self.stamps.unwrap_or_default(),
body_hash: self.body_hash,
```

For attributes building, update the Evidence/Analysis/Hypothesis arms in `build_attributes()`:

```rust
ItemType::Evidence => ItemAttributes::Evidence {
    sourcing: self.sourcing.clone(),
    relation: self.relation.clone(),
},
ItemType::Analysis => ItemAttributes::Analysis {
    assessment: self.assessment.clone(),
},
ItemType::Hypothesis => ItemAttributes::Hypothesis {
    assessment: self.assessment.clone(),
},
```

Fix all exhaustiveness errors from the variant changes — any match on `ItemAttributes::Evidence` / `Analysis` / `Hypothesis` that was `ItemAttributes::Evidence =>` must become `ItemAttributes::Evidence { .. } =>`.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p sara-core`
Expected: all tests pass including the 7 new ones

**Step 5: Commit**

```
feat: add outcome, stamps, reviewed, body_hash fields to Item
```

---

### Task 2: Parse new fields from frontmatter

**Files:**
- Modify: `sara-core/src/parser/markdown.rs`
- Test: inline `#[cfg(test)] mod tests`

**Step 1: Write failing tests**

```rust
#[test]
fn test_parse_evidence_with_outcome_and_sourcing() {
    let content = indoc! {r#"
        ---
        id: "EVD-001"
        type: evidence
        name: "Test Evidence"
        outcome: "open"
        sourcing: "C"
        relation: "hosted"
        parent:
          - "ITM-001"
        ---
        # Evidence body
    "#};
    let result = parse_markdown_content(content, "EVD-001.md", Path::new("/test"));
    let doc = result.unwrap();
    assert_eq!(doc.item.outcome.as_deref(), Some("open"));
    assert_eq!(doc.item.attributes.sourcing(), Some("C"));
    assert_eq!(doc.item.attributes.evidence_relation(), Some("hosted"));
}

#[test]
fn test_parse_analysis_with_assessment() {
    let content = indoc! {r#"
        ---
        id: "ANL-001"
        type: analysis
        name: "Test Analysis"
        outcome: "open"
        assessment: "very-likely"
        parent:
          - "THS-001"
        ---
        # Analysis body
    "#};
    let result = parse_markdown_content(content, "ANL-001.md", Path::new("/test"));
    let doc = result.unwrap();
    assert_eq!(doc.item.outcome.as_deref(), Some("open"));
    assert_eq!(doc.item.attributes.assessment(), Some("very-likely"));
}

#[test]
fn test_parse_item_with_stamps_and_reviewed() {
    let content = indoc! {r#"
        ---
        id: "ANL-001"
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
    "#};
    let result = parse_markdown_content(content, "ANL-001.md", Path::new("/test"));
    let doc = result.unwrap();
    assert_eq!(doc.item.reviewed.as_deref(), Some("deadbeef"));
    assert_eq!(doc.item.stamps.len(), 2);
}

#[test]
fn test_parse_body_hash_computed() {
    let content = indoc! {r#"
        ---
        id: "EVD-001"
        type: evidence
        name: "Test"
        parent:
          - "ITM-001"
        ---
        # Some body content
        With multiple lines.
    "#};
    let result = parse_markdown_content(content, "EVD-001.md", Path::new("/test"));
    let doc = result.unwrap();
    assert!(doc.item.body_hash.is_some());
    assert!(!doc.item.body_hash.as_ref().unwrap().is_empty());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p sara-core markdown::tests::test_parse_evidence_with_outcome`
Expected: FAIL

**Step 3: Implement parser changes**

Add to `RawFrontmatter` struct:

```rust
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
```

Add `use std::collections::HashMap;` at the top.

In the item-building section of `parse_markdown_content`, after constructing the builder, add:

```rust
if let Some(ref outcome) = raw.outcome {
    builder = builder.outcome(outcome.clone());
}
if let Some(ref reviewed) = raw.reviewed {
    builder = builder.reviewed(reviewed.clone());
}
if !raw.stamps.is_empty() {
    let stamps: HashMap<ItemId, String> = raw.stamps.iter()
        .map(|(k, v)| (ItemId::new_unchecked(k), v.clone()))
        .collect();
    builder = builder.stamps(stamps);
}
```

In the investigation type match arm, pass type-specific fields:

```rust
ItemType::Evidence => {
    if let Some(ref s) = raw.sourcing {
        builder = builder.sourcing(s.clone());
    }
    if let Some(ref r) = raw.relation {
        builder = builder.relation(r.clone());
    }
}
ItemType::Analysis | ItemType::Hypothesis => {
    if let Some(ref a) = raw.assessment {
        builder = builder.assessment(a.clone());
    }
}
```

Compute body_hash from the extracted body using SHA-256:

```rust
use sha2::{Sha256, Digest};

let body = extract_body(content);
let body_hash = if !body.trim().is_empty() {
    let hash = Sha256::digest(body.trim().as_bytes());
    Some(format!("{:x}", hash))
} else {
    None
};
builder = builder.body_hash(body_hash.unwrap_or_default());
```

**Step 4: Run tests**

Run: `cargo test -p sara-core`
Expected: all pass

**Step 5: Commit**

```
feat: parse outcome, assessment, sourcing, stamps from frontmatter
```

---

## Review checkpoint — Batch 1

Show test counts, `cargo clippy` output. Ready for feedback.

---

## Batch 2: Fingerprint module (Tasks 3–4)

### Task 3: Create fingerprint module

**Files:**
- Create: `sara-core/src/fingerprint/mod.rs`
- Create: `sara-core/src/fingerprint/compute.rs`
- Modify: `sara-core/src/lib.rs` (add `pub mod fingerprint;`)
- Test: inline in `compute.rs`

**Step 1: Write failing tests**

In `compute.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ItemType;

    #[test]
    fn test_compute_fingerprint_basic() {
        let fp = compute_fingerprint("EVD-001", "body text", Some("open"), &[]);
        assert!(!fp.is_empty());
        assert_eq!(fp.len(), 64); // SHA-256 hex
    }

    #[test]
    fn test_fingerprint_deterministic() {
        let fp1 = compute_fingerprint("EVD-001", "body", Some("open"), &[]);
        let fp2 = compute_fingerprint("EVD-001", "body", Some("open"), &[]);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_fingerprint_changes_with_outcome() {
        let fp1 = compute_fingerprint("EVD-001", "body", Some("open"), &[]);
        let fp2 = compute_fingerprint("EVD-001", "body", Some("verified"), &[]);
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_fingerprint_changes_with_body() {
        let fp1 = compute_fingerprint("EVD-001", "body v1", Some("open"), &[]);
        let fp2 = compute_fingerprint("EVD-001", "body v2", Some("open"), &[]);
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_fingerprint_changes_with_type_fields() {
        let fp1 = compute_fingerprint("EVD-001", "body", Some("open"), &[("sourcing", "C")]);
        let fp2 = compute_fingerprint("EVD-001", "body", Some("open"), &[("sourcing", "X")]);
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_fingerprint_none_outcome() {
        let fp = compute_fingerprint("EVD-001", "body", None, &[]);
        assert_eq!(fp.len(), 64);
    }

    #[test]
    fn test_fingerprinted_fields_evidence() {
        let fields = fingerprinted_fields(ItemType::Evidence);
        assert!(fields.contains(&"sourcing"));
        assert!(fields.contains(&"relation"));
    }

    #[test]
    fn test_fingerprinted_fields_analysis() {
        let fields = fingerprinted_fields(ItemType::Analysis);
        assert!(fields.contains(&"assessment"));
    }

    #[test]
    fn test_fingerprinted_fields_default() {
        let fields = fingerprinted_fields(ItemType::Entity);
        assert!(fields.is_empty()); // only outcome, which is handled separately
    }

    #[test]
    fn test_truncate_fingerprint() {
        let fp = compute_fingerprint("EVD-001", "body", Some("open"), &[]);
        let short = truncate_fingerprint(&fp);
        assert_eq!(short.len(), 8);
        assert!(fp.starts_with(short));
    }
}
```

**Step 2: Run to verify failure**

Run: `cargo test -p sara-core fingerprint`
Expected: FAIL — module doesn't exist

**Step 3: Implement**

`sara-core/src/lib.rs` — add `pub mod fingerprint;`

`sara-core/src/fingerprint/mod.rs`:

```rust
//! Fingerprint computation for suspect link detection.

mod compute;

pub use compute::{
    compute_fingerprint, compute_item_fingerprint, fingerprinted_fields, truncate_fingerprint,
};
```

`sara-core/src/fingerprint/compute.rs`:

```rust
//! SHA-256 fingerprint computation.

use sha2::{Digest, Sha256};

use crate::model::{Item, ItemType};

/// Computes a SHA-256 fingerprint from item components.
///
/// Fields are concatenated in deterministic order: id, body, outcome,
/// then type-specific fields sorted by name.
pub fn compute_fingerprint(
    id: &str,
    body: &str,
    outcome: Option<&str>,
    type_specific_fields: &[(&str, &str)],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(id.as_bytes());
    hasher.update(b"\0");
    hasher.update(body.trim().as_bytes());
    hasher.update(b"\0");
    hasher.update(outcome.unwrap_or("").as_bytes());
    hasher.update(b"\0");

    let mut sorted_fields: Vec<_> = type_specific_fields.to_vec();
    sorted_fields.sort_by_key(|(name, _)| *name);
    for (name, value) in &sorted_fields {
        hasher.update(name.as_bytes());
        hasher.update(b"=");
        hasher.update(value.as_bytes());
        hasher.update(b"\0");
    }

    format!("{:x}", hasher.finalize())
}

/// Computes a fingerprint for an Item using its body_hash and attributes.
///
/// Uses body_hash (pre-computed during parsing) instead of raw body text.
/// This allows fingerprint computation without file access.
pub fn compute_item_fingerprint(item: &Item) -> String {
    let body = item.body_hash.as_deref().unwrap_or("");
    let type_fields = collect_type_fields(item);
    compute_fingerprint(item.id.as_str(), body, item.outcome.as_deref(), &type_fields)
}

/// Returns the type-specific field names that contribute to fingerprinting.
pub fn fingerprinted_fields(item_type: ItemType) -> &'static [&'static str] {
    match item_type {
        ItemType::Evidence => &["sourcing", "relation"],
        ItemType::Analysis => &["assessment"],
        // Hypothesis assessment is optional and not in the Wombatt fingerprint table
        _ => &[],
    }
}

/// Truncates a fingerprint to 8 hex chars for frontmatter display.
pub fn truncate_fingerprint(fingerprint: &str) -> &str {
    &fingerprint[..8.min(fingerprint.len())]
}

/// Collects type-specific field values from an item for fingerprinting.
fn collect_type_fields(item: &Item) -> Vec<(&str, &str)> {
    let mut fields = Vec::new();
    if let Some(ref s) = item.attributes.sourcing() {
        fields.push(("sourcing", *s));
    }
    if let Some(ref r) = item.attributes.evidence_relation() {
        fields.push(("relation", *r));
    }
    if let Some(ref a) = item.attributes.assessment() {
        fields.push(("assessment", *a));
    }
    fields
}
```

Note: `collect_type_fields` uses the accessor methods, so the `*s` dereferences `&&str` to `&str`. Adjust borrows if needed during implementation.

**Step 4: Run tests**

Run: `cargo test -p sara-core fingerprint`
Expected: all 10 pass

**Step 5: Commit**

```
feat: add fingerprint module with SHA-256 computation
```

---

### Task 4: Add SuspectLinksRule validation

**Files:**
- Create: `sara-core/src/validation/rules/suspect_links.rs`
- Modify: `sara-core/src/validation/rules/mod.rs`
- Modify: `sara-core/src/validation/validator.rs`
- Test: inline in `suspect_links.rs`

**Step 1: Write failing tests**

In `suspect_links.rs`:

```rust
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
        // Build evidence with correct stamp for its parent entity
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
        // No stamps on evidence

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
```

**Step 2: Run to verify failure**

Run: `cargo test -p sara-core suspect_links`
Expected: FAIL — module doesn't exist

**Step 3: Implement**

`sara-core/src/validation/rules/suspect_links.rs`:

```rust
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
            let target_ids = item.upstream.all_ids();

            for target_id in &target_ids {
                let Some(target) = graph.get(target_id) else {
                    continue; // Broken ref handled by another rule
                };

                let current_fp = compute_item_fingerprint(target);

                match item.stamps.get(target_id) {
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
```

Add `SuspectLink` variant to `ValidationError` in `sara-core/src/error.rs`:

```rust
#[error("Suspect link: {source_id} → {target_id}: {reason}")]
SuspectLink {
    source_id: String,
    target_id: String,
    reason: String,
},
```

Register the rule in `sara-core/src/validation/rules/mod.rs`:

```rust
mod suspect_links;
pub use suspect_links::SuspectLinksRule;
```

Add to `RULES` array in `sara-core/src/validation/validator.rs`:

```rust
static RULES: &[&dyn ValidationRule] = &[
    &BrokenReferencesRule,
    &DuplicatesRule,
    &CyclesRule,
    &RelationshipsRule,
    &MetadataRule,
    &RedundantRelationshipsRule,
    &OrphansRule,
    &SuspectLinksRule,  // new
];
```

**Step 4: Run tests**

Run: `cargo test -p sara-core`
Expected: all pass

**Step 5: Commit**

```
feat: add SuspectLinksRule for fingerprint-based link validation
```

---

## Review checkpoint — Batch 2

Show test counts, `cargo clippy` output. Ready for feedback.

---

## Batch 3: CLI commands (Tasks 5–6)

### Task 5: Add `sara review` command

**Files:**
- Create: `sara-cli/src/commands/review.rs`
- Modify: `sara-cli/src/commands/mod.rs`
- Test: `sara-cli/tests/cli_tests.rs` (new module)

**Step 1: Write failing CLI test**

In `sara-cli/tests/cli_tests.rs`, add a new module:

```rust
mod review_command {
    use super::*;

    #[test]
    fn test_review_help() {
        sara()
            .arg("review")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Mark an item as reviewed"));
    }

    #[test]
    fn test_review_nonexistent_item() {
        let fixtures = fixtures_path().join("investigation");

        sara()
            .arg("review")
            .arg("-r")
            .arg(&fixtures)
            .arg("NONEXISTENT-001")
            .assert()
            .failure();
    }
}
```

**Step 2: Run to verify failure**

Run: `cargo test -p sara-cli review_command`
Expected: FAIL — no `review` subcommand

**Step 3: Implement**

`sara-cli/src/commands/review.rs`:

```rust
//! Implementation of the review command.

use std::error::Error;
use std::fs;
use std::process::ExitCode;

use clap::Args;

use sara_core::fingerprint::{compute_item_fingerprint, truncate_fingerprint};
use sara_core::graph::KnowledgeGraphBuilder;
use sara_core::model::ItemId;
use sara_core::parser::frontmatter::update_frontmatter;

use super::CommandContext;
use crate::output::{format_error, format_success};

/// Arguments for the review command.
#[derive(Args, Debug)]
pub struct ReviewArgs {
    /// Item ID to mark as reviewed
    pub item_id: String,
}

/// Runs the review command.
pub fn run(args: &ReviewArgs, ctx: &CommandContext) -> Result<ExitCode, Box<dyn Error>> {
    let items = ctx.parse_items(None)?;

    let graph = KnowledgeGraphBuilder::new()
        .add_items(items.clone())
        .build()?;

    let item_id = ItemId::new_unchecked(&args.item_id);
    let Some(item) = graph.get(&item_id) else {
        eprintln!("{}", format_error(&ctx.output, &format!(
            "Item '{}' not found", args.item_id
        )));
        return Ok(ExitCode::FAILURE);
    };

    // Compute own fingerprint
    let own_fp = compute_item_fingerprint(item);
    let own_fp_short = truncate_fingerprint(&own_fp).to_string();

    // Compute stamps for all outgoing relation targets
    let target_ids = item.upstream.all_ids();
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

    // Build updated YAML by modifying frontmatter
    let yaml = build_review_yaml(&content, &own_fp_short, &stamps);
    let updated = update_frontmatter(&content, &yaml);
    fs::write(&file_path, updated)?;

    println!("{}", format_success(&ctx.output, &format!(
        "Reviewed {} (fingerprint: {}), stamped {} link(s)",
        args.item_id, own_fp_short, stamps.len()
    )));

    Ok(ExitCode::SUCCESS)
}

/// Builds updated YAML with reviewed and stamps fields.
fn build_review_yaml(content: &str, reviewed: &str, stamps: &[(String, String)]) -> String {
    // Parse existing YAML, add/update reviewed and stamps fields
    // Implementation: read existing frontmatter as serde_yaml::Value,
    // insert reviewed and stamps, serialize back
    todo!("implement YAML update logic")
}
```

The `build_review_yaml` implementation will parse existing YAML as `serde_yaml::Value`, insert `reviewed` and `stamps` fields, and serialize back. This preserves all existing frontmatter fields.

Register in `sara-cli/src/commands/mod.rs`:

```rust
mod review;
use self::review::ReviewArgs;

// In Commands enum:
/// Mark an item as reviewed and re-stamp outgoing links
Review(ReviewArgs),

// In run() match:
Commands::Review(args) => review::run(args, &ctx),
```

**Step 4: Run tests**

Run: `cargo test --workspace --all-targets`
Expected: all pass

**Step 5: Commit**

```
feat: add sara review command for marking items reviewed
```

---

### Task 6: Add `sara clear` command

**Files:**
- Create: `sara-cli/src/commands/clear.rs`
- Modify: `sara-cli/src/commands/mod.rs`
- Test: `sara-cli/tests/cli_tests.rs` (new module)

**Step 1: Write failing CLI test**

```rust
mod clear_command {
    use super::*;

    #[test]
    fn test_clear_help() {
        sara()
            .arg("clear")
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("Clear a suspect link"));
    }
}
```

**Step 2: Run to verify failure**

Run: `cargo test -p sara-cli clear_command`
Expected: FAIL

**Step 3: Implement**

`sara-cli/src/commands/clear.rs`:

```rust
//! Implementation of the clear command.

use std::error::Error;
use std::fs;
use std::process::ExitCode;

use clap::Args;

use sara_core::fingerprint::{compute_item_fingerprint, truncate_fingerprint};
use sara_core::graph::KnowledgeGraphBuilder;
use sara_core::model::ItemId;
use sara_core::parser::frontmatter::update_frontmatter;

use super::CommandContext;
use crate::output::{format_error, format_success};

/// Arguments for the clear command.
#[derive(Args, Debug)]
pub struct ClearArgs {
    /// Source item ID
    pub item_id: String,
    /// Target item ID to re-stamp
    pub target_id: String,
}

/// Runs the clear command.
pub fn run(args: &ClearArgs, ctx: &CommandContext) -> Result<ExitCode, Box<dyn Error>> {
    let items = ctx.parse_items(None)?;

    let graph = KnowledgeGraphBuilder::new()
        .add_items(items.clone())
        .build()?;

    let item_id = ItemId::new_unchecked(&args.item_id);
    let target_id = ItemId::new_unchecked(&args.target_id);

    let Some(item) = graph.get(&item_id) else {
        eprintln!("{}", format_error(&ctx.output, &format!(
            "Item '{}' not found", args.item_id
        )));
        return Ok(ExitCode::FAILURE);
    };

    let Some(target) = graph.get(&target_id) else {
        eprintln!("{}", format_error(&ctx.output, &format!(
            "Target '{}' not found", args.target_id
        )));
        return Ok(ExitCode::FAILURE);
    };

    let target_fp = compute_item_fingerprint(target);
    let target_fp_short = truncate_fingerprint(&target_fp).to_string();

    // Read file, update stamps for this specific target
    let file_path = item.source.full_path();
    let content = fs::read_to_string(&file_path)?;
    let yaml = build_clear_yaml(&content, &args.target_id, &target_fp_short);
    let updated = update_frontmatter(&content, &yaml);
    fs::write(&file_path, updated)?;

    println!("{}", format_success(&ctx.output, &format!(
        "Cleared suspect link {} → {} (stamp: {})",
        args.item_id, args.target_id, target_fp_short
    )));

    Ok(ExitCode::SUCCESS)
}

/// Builds updated YAML with a single stamp updated.
fn build_clear_yaml(content: &str, target_id: &str, stamp: &str) -> String {
    // Parse existing YAML, update stamps[target_id], serialize back
    todo!("implement YAML update logic")
}
```

Register in `mod.rs` same pattern as review.

**Step 4: Run tests**

Run: `cargo test --workspace --all-targets`
Expected: all pass

**Step 5: Commit**

```
feat: add sara clear command for re-stamping individual links
```

---

## Review checkpoint — Batch 3

Show test counts, `cargo clippy` output. Ready for feedback.

---

## Batch 4: Integration and polish (Tasks 7–8)

### Task 7: Update investigation fixtures with outcome/stamps

**Files:**
- Modify: `tests/fixtures/investigation/*.md` (add outcome fields)
- Create: integration test for suspect link detection

Update the 8 investigation fixture files to include `outcome` fields.
Add a fixture variant with stamps for testing the "clean" path.
Write an integration test that parses the fixtures, builds the graph,
runs validation, and verifies suspect link warnings appear.

**Step 1: Update fixture files**

Add `outcome: "open"` to each investigation fixture's frontmatter.
Add `assessment: "roughly-even"` to `ANL-001.md`.
Add `sourcing: "C"` to `EVD-001.md`.

**Step 2: Write integration test**

In `sara-core/src/validation/validator.rs` tests:

```rust
#[test]
fn test_suspect_links_detected_for_unstamped_items() {
    // Build investigation hierarchy — no stamps
    let items = create_investigation_hierarchy();
    let graph = KnowledgeGraphBuilder::new()
        .add_items(items)
        .build()
        .unwrap();
    let report = validate(&graph);
    // Should have suspect link warnings for all unstamped relations
    assert!(report.warning_count() > 0);
}
```

**Step 3: Run tests**

Run: `cargo test --workspace --all-targets`

**Step 4: Commit**

```
test: add outcome fields to investigation fixtures
```

---

### Task 8: Final verification

**Step 1:** `cargo fmt --all -- --check`
**Step 2:** `cargo clippy --workspace --all-targets -- -D warnings`
**Step 3:** `cargo test --workspace --all-targets`

All must pass. Report exact counts.

**Step 4: Commit any fixes**

```
chore: fix formatting and clippy warnings
```

---

## Summary

| Task | What | New tests |
|------|------|-----------|
| 0 | sha2 dependency | 0 |
| 1 | Item/ItemAttributes/ItemBuilder fields | ~7 |
| 2 | Parser: outcome, assessment, sourcing, stamps | ~4 |
| 3 | Fingerprint module | ~10 |
| 4 | SuspectLinksRule | ~3 |
| 5 | `sara review` command | ~2 |
| 6 | `sara clear` command | ~1 |
| 7 | Fixture updates + integration test | ~1 |
| 8 | Final verification | 0 |

**Estimated new tests:** ~28
**Batch checkpoints:** After tasks 2, 4, 6, 8
