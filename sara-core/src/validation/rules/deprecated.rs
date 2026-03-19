//! Deprecated field detection rule.

use crate::config::ValidationConfig;
use crate::error::ValidationError;
use crate::model::Item;
use crate::validation::rule::{Severity, ValidationRule};

/// Warns when items use fields listed in config `deprecated_fields`.
pub struct DeprecatedFieldsRule;

impl ValidationRule for DeprecatedFieldsRule {
    fn pre_validate(&self, items: &[Item], config: &ValidationConfig) -> Vec<ValidationError> {
        if config.deprecated_fields.is_empty() {
            return Vec::new();
        }

        let mut warnings = Vec::new();
        for item in items {
            let type_str = item.item_type.as_str();
            if let Some(deprecated_list) = config.deprecated_fields.get(type_str) {
                for key in &item.raw_field_keys {
                    if deprecated_list.contains(key) {
                        warnings.push(ValidationError::DeprecatedField {
                            field: key.clone(),
                            file: item.source.file_path.display().to_string(),
                            reason: format!(
                                "Field '{}' is deprecated for type '{}'",
                                key, type_str
                            ),
                        });
                    }
                }
            }
        }
        warnings
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ItemBuilder, ItemId, ItemType, SourceLocation};
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_deprecated_field_detected() {
        let source = SourceLocation::new(PathBuf::from("/test"), "EVD-001.md".to_string());
        let item = ItemBuilder::new()
            .id(ItemId::new_unchecked("EVD-001"))
            .item_type(ItemType::Evidence)
            .name("Test Evidence")
            .source(source)
            .raw_field_keys(vec![
                "id".to_string(),
                "type".to_string(),
                "name".to_string(),
                "sourcing".to_string(),
            ])
            .build()
            .unwrap();

        let mut deprecated = HashMap::new();
        deprecated.insert("evidence".to_string(), vec!["sourcing".to_string()]);
        let config = ValidationConfig {
            deprecated_fields: deprecated,
            ..Default::default()
        };

        let warnings = DeprecatedFieldsRule.pre_validate(&[item], &config);
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            &warnings[0],
            ValidationError::DeprecatedField { field, .. } if field == "sourcing"
        ));
    }

    #[test]
    fn test_no_deprecated_fields() {
        let source = SourceLocation::new(PathBuf::from("/test"), "EVD-001.md".to_string());
        let item = ItemBuilder::new()
            .id(ItemId::new_unchecked("EVD-001"))
            .item_type(ItemType::Evidence)
            .name("Test Evidence")
            .source(source)
            .raw_field_keys(vec![
                "id".to_string(),
                "type".to_string(),
                "name".to_string(),
            ])
            .build()
            .unwrap();

        let mut deprecated = HashMap::new();
        deprecated.insert("evidence".to_string(), vec!["sourcing".to_string()]);
        let config = ValidationConfig {
            deprecated_fields: deprecated,
            ..Default::default()
        };

        let warnings = DeprecatedFieldsRule.pre_validate(&[item], &config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_empty_deprecated_config() {
        let source = SourceLocation::new(PathBuf::from("/test"), "EVD-001.md".to_string());
        let item = ItemBuilder::new()
            .id(ItemId::new_unchecked("EVD-001"))
            .item_type(ItemType::Evidence)
            .name("Test Evidence")
            .source(source)
            .raw_field_keys(vec!["sourcing".to_string()])
            .build()
            .unwrap();

        let config = ValidationConfig::default();
        let warnings = DeprecatedFieldsRule.pre_validate(&[item], &config);
        assert!(warnings.is_empty());
    }
}
