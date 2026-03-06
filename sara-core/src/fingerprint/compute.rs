//! SHA-256 fingerprint computation.

use sha2::{Digest, Sha256};

use crate::model::{Item, ItemType};

/// Computes a SHA-256 fingerprint from item components.
///
/// Fields are concatenated in deterministic order: id, body, outcome,
/// then type-specific fields sorted by name. Null-byte separators
/// prevent field value collisions.
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
        ItemType::Evidence => &["relation", "sourcing"],
        ItemType::Analysis => &["assessment"],
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
    if let Some(s) = item.attributes.sourcing() {
        fields.push(("sourcing", s));
    }
    if let Some(r) = item.attributes.evidence_relation() {
        fields.push(("relation", r));
    }
    if let Some(a) = item.attributes.assessment() {
        fields.push(("assessment", a));
    }
    fields
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(fields.is_empty());
    }

    #[test]
    fn test_truncate_fingerprint() {
        let fp = compute_fingerprint("EVD-001", "body", Some("open"), &[]);
        let short = truncate_fingerprint(&fp);
        assert_eq!(short.len(), 8);
        assert!(fp.starts_with(short));
    }
}
