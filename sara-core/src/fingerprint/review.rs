//! Frontmatter update helpers for review and stamp operations.

use std::path::Path;

use serde_yaml::Value;

use crate::parser::{extract_frontmatter, update_frontmatter};

/// Updates file content with review fingerprint and stamps for all upstream targets.
///
/// Parses existing YAML frontmatter, adds/updates `reviewed` and `stamps` fields,
/// then reconstructs the file with the updated frontmatter.
pub fn apply_review(
    content: &str,
    reviewed: &str,
    stamps: &[(String, String)],
    file_path: &Path,
) -> Result<String, String> {
    let extracted = extract_frontmatter(content, file_path).map_err(|e| e.to_string())?;

    let mut yaml: Value =
        serde_yaml::from_str(&extracted.yaml).map_err(|e| format!("YAML parse error: {e}"))?;

    let map = yaml
        .as_mapping_mut()
        .ok_or_else(|| "frontmatter is not a YAML mapping".to_string())?;

    // Set reviewed field
    map.insert(
        Value::String("reviewed".to_string()),
        Value::String(reviewed.to_string()),
    );

    // Build stamps mapping
    if !stamps.is_empty() {
        let mut stamps_map = serde_yaml::Mapping::new();
        for (target_id, stamp) in stamps {
            stamps_map.insert(
                Value::String(target_id.clone()),
                Value::String(stamp.clone()),
            );
        }
        map.insert(
            Value::String("stamps".to_string()),
            Value::Mapping(stamps_map),
        );
    } else {
        map.remove("stamps");
    }

    let new_yaml =
        serde_yaml::to_string(&yaml).map_err(|e| format!("YAML serialize error: {e}"))?;
    Ok(update_frontmatter(content, &new_yaml))
}

/// Updates file content with a single stamp for a specific target.
///
/// Parses existing YAML frontmatter, updates the stamp for the given target ID,
/// then reconstructs the file with the updated frontmatter.
pub fn apply_stamp(
    content: &str,
    target_id: &str,
    stamp: &str,
    file_path: &Path,
) -> Result<String, String> {
    let extracted = extract_frontmatter(content, file_path).map_err(|e| e.to_string())?;

    let mut yaml: Value =
        serde_yaml::from_str(&extracted.yaml).map_err(|e| format!("YAML parse error: {e}"))?;

    let map = yaml
        .as_mapping_mut()
        .ok_or_else(|| "frontmatter is not a YAML mapping".to_string())?;

    // Get or create stamps mapping
    let stamps_value = map
        .entry(Value::String("stamps".to_string()))
        .or_insert_with(|| Value::Mapping(serde_yaml::Mapping::new()));

    let stamps_map = stamps_value
        .as_mapping_mut()
        .ok_or_else(|| "stamps field is not a mapping".to_string())?;

    stamps_map.insert(
        Value::String(target_id.to_string()),
        Value::String(stamp.to_string()),
    );

    let new_yaml =
        serde_yaml::to_string(&yaml).map_err(|e| format!("YAML serialize error: {e}"))?;
    Ok(update_frontmatter(content, &new_yaml))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_review_adds_reviewed_and_stamps() {
        let content = "---\nid: \"EVD-001\"\ntype: evidence\nname: \"Test\"\n---\nBody text.\n";
        let result = apply_review(
            content,
            "abcd1234",
            &[("ITM-001".into(), "ef567890".into())],
            Path::new("EVD-001.md"),
        )
        .unwrap();

        assert!(result.contains("reviewed:"));
        assert!(result.contains("abcd1234"));
        assert!(result.contains("stamps:"));
        assert!(result.contains("ITM-001"));
        assert!(result.contains("ef567890"));
        assert!(result.contains("Body text."));
    }

    #[test]
    fn test_apply_review_preserves_existing_fields() {
        let content =
            "---\nid: \"EVD-001\"\ntype: evidence\nname: \"Test\"\noutcome: open\n---\nBody.\n";
        let result = apply_review(content, "abcd1234", &[], Path::new("EVD-001.md")).unwrap();

        assert!(result.contains("id:"));
        assert!(result.contains("EVD-001"));
        assert!(result.contains("outcome:"));
        assert!(result.contains("open"));
        assert!(result.contains("reviewed:"));
    }

    #[test]
    fn test_apply_review_removes_stamps_when_empty() {
        let content =
            "---\nid: \"EVD-001\"\ntype: evidence\nstamps:\n  ITM-001: oldstamp\n---\nBody.\n";
        let result = apply_review(content, "abcd1234", &[], Path::new("EVD-001.md")).unwrap();

        assert!(result.contains("reviewed:"));
        assert!(
            !result.contains("stamps:"),
            "stamps key should be removed when list is empty"
        );
        assert!(!result.contains("oldstamp"));
    }

    #[test]
    fn test_apply_stamp_adds_single_stamp() {
        let content = "---\nid: \"EVD-001\"\ntype: evidence\n---\nBody.\n";
        let result = apply_stamp(content, "ITM-001", "abcd1234", Path::new("EVD-001.md")).unwrap();

        assert!(result.contains("stamps:"));
        assert!(result.contains("ITM-001"));
        assert!(result.contains("abcd1234"));
    }

    #[test]
    fn test_apply_stamp_updates_existing_stamp() {
        let content =
            "---\nid: \"EVD-001\"\ntype: evidence\nstamps:\n  ITM-001: oldstamp\n---\nBody.\n";
        let result = apply_stamp(content, "ITM-001", "newstamp", Path::new("EVD-001.md")).unwrap();

        assert!(result.contains("newstamp"));
        assert!(!result.contains("oldstamp"));
    }
}
