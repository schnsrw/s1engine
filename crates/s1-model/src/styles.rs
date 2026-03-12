//! Style definitions and resolution.
//!
//! Styles form an inheritance chain. Effective formatting is resolved by:
//! ```text
//! Direct formatting → Character style → Paragraph style → Default style
//! (highest priority)                                      (lowest priority)
//! ```

use crate::attributes::AttributeMap;

/// A named style definition.
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    /// Unique identifier (e.g., "Heading1", "Normal").
    pub id: String,
    /// Human-readable name (e.g., "Heading 1", "Normal").
    pub name: String,
    /// Whether this is a paragraph, character, table, or list style.
    pub style_type: StyleType,
    /// Parent style ID for inheritance. `None` for root styles.
    pub parent_id: Option<String>,
    /// Style applied to the next paragraph after pressing Enter.
    pub next_style_id: Option<String>,
    /// The attributes this style defines.
    pub attributes: AttributeMap,
    /// Whether this is the default style for its type.
    pub is_default: bool,
}

impl Style {
    /// Create a new style with the given id, name, and type.
    pub fn new(id: impl Into<String>, name: impl Into<String>, style_type: StyleType) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            style_type,
            parent_id: None,
            next_style_id: None,
            attributes: AttributeMap::new(),
            is_default: false,
        }
    }

    /// Set the parent style.
    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    /// Set the attributes.
    pub fn with_attributes(mut self, attrs: AttributeMap) -> Self {
        self.attributes = attrs;
        self
    }

    /// Mark as default style.
    pub fn as_default(mut self) -> Self {
        self.is_default = true;
        self
    }
}

/// The type of a style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StyleType {
    Paragraph,
    Character,
    Table,
    List,
}

/// Resolves the effective attributes for a style by walking the inheritance chain.
///
/// Returns a fully merged `AttributeMap` with all inherited attributes resolved.
pub fn resolve_style_chain(style_id: &str, styles: &[Style]) -> AttributeMap {
    let mut result = AttributeMap::new();
    let mut chain = Vec::new();

    // Walk up the inheritance chain, collecting styles
    let mut current_id = Some(style_id.to_string());
    let mut visited = std::collections::HashSet::new();

    while let Some(id) = current_id {
        if !visited.insert(id.clone()) {
            break; // Circular reference protection
        }
        if let Some(style) = styles.iter().find(|s| s.id == id) {
            chain.push(style);
            current_id = style.parent_id.clone();
        } else {
            break;
        }
    }

    // Apply in reverse order (most general first, most specific last)
    for style in chain.iter().rev() {
        result.merge(&style.attributes);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::{AttributeKey, AttributeValue};

    #[test]
    fn create_style() {
        let style = Style::new("Normal", "Normal", StyleType::Paragraph).as_default();
        assert_eq!(style.id, "Normal");
        assert_eq!(style.name, "Normal");
        assert_eq!(style.style_type, StyleType::Paragraph);
        assert!(style.is_default);
        assert!(style.parent_id.is_none());
    }

    #[test]
    fn style_with_parent() {
        let style = Style::new("Heading1", "Heading 1", StyleType::Paragraph).with_parent("Normal");
        assert_eq!(style.parent_id.as_deref(), Some("Normal"));
    }

    #[test]
    fn resolve_single_style() {
        let mut attrs = AttributeMap::new();
        attrs.set(AttributeKey::Bold, AttributeValue::Bool(true));
        attrs.set(AttributeKey::FontSize, AttributeValue::Float(12.0));

        let styles =
            vec![Style::new("Normal", "Normal", StyleType::Paragraph).with_attributes(attrs)];

        let resolved = resolve_style_chain("Normal", &styles);
        assert_eq!(resolved.get_bool(&AttributeKey::Bold), Some(true));
        assert_eq!(resolved.get_f64(&AttributeKey::FontSize), Some(12.0));
    }

    #[test]
    fn resolve_inherited_style() {
        let mut normal_attrs = AttributeMap::new();
        normal_attrs.set(AttributeKey::FontSize, AttributeValue::Float(12.0));
        normal_attrs.set(
            AttributeKey::FontFamily,
            AttributeValue::String("Times".into()),
        );

        let mut heading_attrs = AttributeMap::new();
        heading_attrs.set(AttributeKey::FontSize, AttributeValue::Float(24.0));
        heading_attrs.set(AttributeKey::Bold, AttributeValue::Bool(true));

        let styles = vec![
            Style::new("Normal", "Normal", StyleType::Paragraph).with_attributes(normal_attrs),
            Style::new("Heading1", "Heading 1", StyleType::Paragraph)
                .with_parent("Normal")
                .with_attributes(heading_attrs),
        ];

        let resolved = resolve_style_chain("Heading1", &styles);
        // Heading1 overrides FontSize
        assert_eq!(resolved.get_f64(&AttributeKey::FontSize), Some(24.0));
        // Heading1 adds Bold
        assert_eq!(resolved.get_bool(&AttributeKey::Bold), Some(true));
        // Inherited from Normal
        assert_eq!(
            resolved.get_string(&AttributeKey::FontFamily),
            Some("Times")
        );
    }

    #[test]
    fn resolve_deep_chain() {
        let mut base = AttributeMap::new();
        base.set(AttributeKey::FontSize, AttributeValue::Float(10.0));
        base.set(
            AttributeKey::FontFamily,
            AttributeValue::String("Serif".into()),
        );

        let mut mid = AttributeMap::new();
        mid.set(AttributeKey::FontSize, AttributeValue::Float(12.0));

        let mut top = AttributeMap::new();
        top.set(AttributeKey::Bold, AttributeValue::Bool(true));

        let styles = vec![
            Style::new("Base", "Base", StyleType::Paragraph).with_attributes(base),
            Style::new("Mid", "Mid", StyleType::Paragraph)
                .with_parent("Base")
                .with_attributes(mid),
            Style::new("Top", "Top", StyleType::Paragraph)
                .with_parent("Mid")
                .with_attributes(top),
        ];

        let resolved = resolve_style_chain("Top", &styles);
        assert_eq!(resolved.get_bool(&AttributeKey::Bold), Some(true)); // from Top
        assert_eq!(resolved.get_f64(&AttributeKey::FontSize), Some(12.0)); // from Mid (overrides Base)
        assert_eq!(
            resolved.get_string(&AttributeKey::FontFamily),
            Some("Serif")
        ); // from Base
    }

    #[test]
    fn resolve_missing_style() {
        let styles = vec![];
        let resolved = resolve_style_chain("NonExistent", &styles);
        assert!(resolved.is_empty());
    }

    #[test]
    fn resolve_circular_reference_protection() {
        let styles = vec![
            Style::new("A", "A", StyleType::Paragraph).with_parent("B"),
            Style::new("B", "B", StyleType::Paragraph).with_parent("A"),
        ];
        // Should not loop forever
        let resolved = resolve_style_chain("A", &styles);
        let _ = resolved; // Just verify it terminates
    }
}
