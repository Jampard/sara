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
    let mut type_fields = collect_type_fields(item);

    // Include participants (sorted for determinism)
    if !item.participants.is_empty() {
        let mut parts: Vec<String> = item
            .participants
            .iter()
            .map(|p| format!("{}:{}", p.entity, p.role))
            .collect();
        parts.sort();
        type_fields.push(("participants", parts.join(",")));
    }

    // Convert owned strings to borrowed for compute_fingerprint
    let borrowed: Vec<(&str, &str)> = type_fields.iter().map(|(k, v)| (*k, v.as_str())).collect();

    compute_fingerprint(item.id.as_str(), body, item.outcome.as_deref(), &borrowed)
}

/// Returns the type-specific field names that contribute to fingerprinting.
pub fn fingerprinted_fields(item_type: ItemType) -> &'static [&'static str] {
    match item_type {
        ItemType::Evidence => &[
            "relation",
            "sourcing",
            "messages",
            "deposition",
            "flights",
            "transactions",
        ],
        ItemType::Analysis | ItemType::Hypothesis => &["assessment"],
        _ => &[],
    }
}

/// Truncates a fingerprint to 8 hex chars for frontmatter display.
pub fn truncate_fingerprint(fingerprint: &str) -> &str {
    &fingerprint[..8.min(fingerprint.len())]
}

/// Collects type-specific field values from an item for fingerprinting.
///
/// For envelope fields, only structural entity references are fingerprinted
/// (from, to, passengers, etc.), not content fields (subject, proceeding).
fn collect_type_fields(item: &Item) -> Vec<(&str, String)> {
    let mut fields: Vec<(&str, String)> = Vec::new();
    if let Some(s) = item.attributes.sourcing() {
        fields.push(("sourcing", s.to_string()));
    }
    if let Some(r) = item.attributes.evidence_relation() {
        fields.push(("relation", r.to_string()));
    }
    if let Some(a) = item.attributes.assessment() {
        fields.push(("assessment", a.to_string()));
    }

    // Envelope structural fields (deterministic sorted serialization)
    let messages = item.attributes.messages();
    if !messages.is_empty() {
        let mut parts: Vec<String> = messages
            .iter()
            .map(|m| {
                let mut to_sorted: Vec<&str> = m.to.iter().map(|id| id.as_str()).collect();
                to_sorted.sort();
                format!("{}>{}", m.from, to_sorted.join(","))
            })
            .collect();
        parts.sort();
        fields.push(("messages", parts.join(";")));
    }

    if let Some(depo) = item.attributes.deposition() {
        let mut speakers: Vec<&str> = depo
            .exchanges
            .iter()
            .map(|ex| ex.speaker.as_str())
            .collect();
        speakers.sort();
        fields.push((
            "deposition",
            format!("{}:{}", depo.witness, speakers.join(",")),
        ));
    }

    let flights = item.attributes.flights();
    if !flights.is_empty() {
        let mut parts: Vec<String> = flights
            .iter()
            .map(|f| {
                let mut pax: Vec<&str> = f.passengers.iter().map(|id| id.as_str()).collect();
                pax.sort();
                format!("{}>{}:{}", f.origin, f.destination, pax.join(","))
            })
            .collect();
        parts.sort();
        fields.push(("flights", parts.join(";")));
    }

    let txns = item.attributes.transactions();
    if !txns.is_empty() {
        let mut parts: Vec<String> = txns
            .iter()
            .map(|t| format!("{}>{}:{}", t.from, t.to, t.amount))
            .collect();
        parts.sort();
        fields.push(("transactions", parts.join(";")));
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
    fn test_fingerprinted_fields_hypothesis() {
        let fields = fingerprinted_fields(ItemType::Hypothesis);
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

    #[test]
    fn test_envelope_messages_change_fingerprint() {
        use crate::model::{
            EnvelopeMessage, ItemBuilder, ItemId, ItemType, Participant, SourceLocation,
        };
        use std::path::PathBuf;

        let source = || SourceLocation::new(PathBuf::from("/test"), "EVD-001.md");

        // Evidence without messages
        let item_no_msg = ItemBuilder::new()
            .id(ItemId::new_unchecked("EVD-001"))
            .item_type(ItemType::Evidence)
            .name("Test")
            .source(source())
            .build()
            .unwrap();

        // Evidence with messages
        let item_with_msg = ItemBuilder::new()
            .id(ItemId::new_unchecked("EVD-001"))
            .item_type(ItemType::Evidence)
            .name("Test")
            .source(source())
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

        let fp1 = compute_item_fingerprint(&item_no_msg);
        let fp2 = compute_item_fingerprint(&item_with_msg);
        assert_ne!(fp1, fp2, "Adding messages should change the fingerprint");
    }

    #[test]
    fn test_envelope_fingerprint_deterministic_regardless_of_order() {
        use crate::model::{
            EnvelopeMessage, ItemBuilder, ItemId, ItemType, Participant, SourceLocation,
        };
        use std::path::PathBuf;

        let source = || SourceLocation::new(PathBuf::from("/test"), "EVD-001.md");
        let participants = || {
            vec![
                Participant {
                    entity: ItemId::new_unchecked("ITM-a"),
                    role: "sender".into(),
                },
                Participant {
                    entity: ItemId::new_unchecked("ITM-b"),
                    role: "recipient".into(),
                },
            ]
        };

        let msg_a_to_b = EnvelopeMessage {
            id: 1,
            from: ItemId::new_unchecked("ITM-a"),
            to: vec![ItemId::new_unchecked("ITM-b")],
            date: None,
            subject: None,
            cc: None,
            bcc: None,
            forward: None,
            removed: None,
        };
        let msg_b_to_a = EnvelopeMessage {
            id: 2,
            from: ItemId::new_unchecked("ITM-b"),
            to: vec![ItemId::new_unchecked("ITM-a")],
            date: None,
            subject: None,
            cc: None,
            bcc: None,
            forward: None,
            removed: None,
        };

        // Order 1: a→b then b→a
        let item1 = ItemBuilder::new()
            .id(ItemId::new_unchecked("EVD-001"))
            .item_type(ItemType::Evidence)
            .name("Test")
            .source(source())
            .participants(participants())
            .envelope_messages(vec![msg_a_to_b.clone(), msg_b_to_a.clone()])
            .build()
            .unwrap();

        // Order 2: b→a then a→b
        let item2 = ItemBuilder::new()
            .id(ItemId::new_unchecked("EVD-001"))
            .item_type(ItemType::Evidence)
            .name("Test")
            .source(source())
            .participants(participants())
            .envelope_messages(vec![msg_b_to_a, msg_a_to_b])
            .build()
            .unwrap();

        let fp1 = compute_item_fingerprint(&item1);
        let fp2 = compute_item_fingerprint(&item2);
        assert_eq!(
            fp1, fp2,
            "Fingerprint should be deterministic regardless of message order"
        );
    }

    #[test]
    fn test_fingerprinted_fields_evidence_includes_envelopes() {
        let fields = fingerprinted_fields(ItemType::Evidence);
        assert!(fields.contains(&"messages"));
        assert!(fields.contains(&"deposition"));
        assert!(fields.contains(&"flights"));
        assert!(fields.contains(&"transactions"));
    }
}
