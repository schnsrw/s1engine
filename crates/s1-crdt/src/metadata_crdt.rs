//! LWW registers for document metadata and styles.
//!
//! Metadata (title, author, etc.) and style definitions use a Last-Writer-Wins
//! strategy: the operation with the highest [`OpId`] wins for each key.

use std::collections::HashMap;

use crate::op_id::OpId;
use s1_model::Style;

/// LWW register for metadata and styles.
#[derive(Debug, Clone)]
pub struct MetadataCrdt {
    /// Metadata key -> (op_id, value). `None` value means the key was deleted.
    metadata: HashMap<String, (OpId, Option<String>)>,
    /// Style ID -> (op_id, style). `None` means the style was removed.
    styles: HashMap<String, (OpId, Option<Style>)>,
}

impl MetadataCrdt {
    /// Create a new empty metadata CRDT.
    pub fn new() -> Self {
        Self {
            metadata: HashMap::new(),
            styles: HashMap::new(),
        }
    }

    /// Integrate a SetMetadata operation.
    ///
    /// Returns `true` if the value was actually updated (LWW comparison passed).
    pub fn integrate_set_metadata(
        &mut self,
        key: &str,
        value: Option<String>,
        op_id: OpId,
    ) -> bool {
        let should_apply = match self.metadata.get(key) {
            Some((existing_op, _)) => op_id > *existing_op,
            None => true,
        };

        if should_apply {
            self.metadata.insert(key.to_string(), (op_id, value));
            true
        } else {
            false
        }
    }

    /// Integrate a SetStyle operation.
    ///
    /// Returns `true` if the style was actually updated.
    pub fn integrate_set_style(&mut self, style: &Style, op_id: OpId) -> bool {
        let should_apply = match self.styles.get(&style.id) {
            Some((existing_op, _)) => op_id > *existing_op,
            None => true,
        };

        if should_apply {
            self.styles
                .insert(style.id.clone(), (op_id, Some(style.clone())));
            true
        } else {
            false
        }
    }

    /// Integrate a RemoveStyle operation.
    ///
    /// Returns `true` if the style was actually removed.
    pub fn integrate_remove_style(&mut self, style_id: &str, op_id: OpId) -> bool {
        let should_apply = match self.styles.get(style_id) {
            Some((existing_op, _)) => op_id > *existing_op,
            None => false, // Nothing to remove
        };

        if should_apply {
            self.styles.insert(style_id.to_string(), (op_id, None));
            true
        } else {
            false
        }
    }

    /// Get the current value of a metadata key.
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).and_then(|(_, v)| v.as_deref())
    }

    /// Get the current style by ID (if not removed).
    pub fn get_style(&self, style_id: &str) -> Option<&Style> {
        self.styles.get(style_id).and_then(|(_, s)| s.as_ref())
    }

    /// Register existing metadata (during init).
    pub fn register_metadata(&mut self, key: &str, value: Option<String>, op_id: OpId) {
        self.metadata.insert(key.to_string(), (op_id, value));
    }

    /// Register an existing style (during init).
    pub fn register_style(&mut self, style: &Style, op_id: OpId) {
        self.styles
            .insert(style.id.clone(), (op_id, Some(style.clone())));
    }
}

impl Default for MetadataCrdt {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::StyleType;

    #[test]
    fn set_metadata() {
        let mut crdt = MetadataCrdt::new();
        assert!(crdt.integrate_set_metadata("title", Some("Doc".into()), OpId::new(1, 1)));
        assert_eq!(crdt.get_metadata("title"), Some("Doc"));
    }

    #[test]
    fn metadata_lww_newer_wins() {
        let mut crdt = MetadataCrdt::new();
        crdt.integrate_set_metadata("title", Some("Old".into()), OpId::new(1, 1));
        crdt.integrate_set_metadata("title", Some("New".into()), OpId::new(2, 2));
        assert_eq!(crdt.get_metadata("title"), Some("New"));
    }

    #[test]
    fn metadata_lww_older_ignored() {
        let mut crdt = MetadataCrdt::new();
        crdt.integrate_set_metadata("title", Some("New".into()), OpId::new(2, 2));
        assert!(!crdt.integrate_set_metadata("title", Some("Old".into()), OpId::new(1, 1)));
        assert_eq!(crdt.get_metadata("title"), Some("New"));
    }

    #[test]
    fn metadata_delete() {
        let mut crdt = MetadataCrdt::new();
        crdt.integrate_set_metadata("title", Some("Doc".into()), OpId::new(1, 1));
        crdt.integrate_set_metadata("title", None, OpId::new(1, 2));
        assert_eq!(crdt.get_metadata("title"), None);
    }

    #[test]
    fn set_style() {
        let mut crdt = MetadataCrdt::new();
        let style = Style::new("H1", "Heading 1", StyleType::Paragraph);
        assert!(crdt.integrate_set_style(&style, OpId::new(1, 1)));
        assert!(crdt.get_style("H1").is_some());
    }

    #[test]
    fn style_lww() {
        let mut crdt = MetadataCrdt::new();
        let style1 = Style::new("H1", "Heading 1", StyleType::Paragraph);
        let style2 = Style::new("H1", "Heading 1 Updated", StyleType::Paragraph);

        crdt.integrate_set_style(&style1, OpId::new(1, 1));
        crdt.integrate_set_style(&style2, OpId::new(2, 2));

        assert_eq!(crdt.get_style("H1").unwrap().name, "Heading 1 Updated");
    }

    #[test]
    fn remove_style() {
        let mut crdt = MetadataCrdt::new();
        let style = Style::new("H1", "Heading 1", StyleType::Paragraph);
        crdt.integrate_set_style(&style, OpId::new(1, 1));
        assert!(crdt.integrate_remove_style("H1", OpId::new(1, 2)));
        assert!(crdt.get_style("H1").is_none());
    }

    #[test]
    fn remove_style_older_ignored() {
        let mut crdt = MetadataCrdt::new();
        let style = Style::new("H1", "Heading 1", StyleType::Paragraph);
        crdt.integrate_set_style(&style, OpId::new(1, 5));
        assert!(!crdt.integrate_remove_style("H1", OpId::new(1, 3)));
        assert!(crdt.get_style("H1").is_some());
    }

    #[test]
    fn set_style_after_remove() {
        let mut crdt = MetadataCrdt::new();
        let style = Style::new("H1", "Heading 1", StyleType::Paragraph);
        crdt.integrate_set_style(&style, OpId::new(1, 1));
        crdt.integrate_remove_style("H1", OpId::new(1, 2));
        assert!(crdt.get_style("H1").is_none());

        let style2 = Style::new("H1", "Heading 1 v2", StyleType::Paragraph);
        crdt.integrate_set_style(&style2, OpId::new(1, 3));
        assert_eq!(crdt.get_style("H1").unwrap().name, "Heading 1 v2");
    }

    #[test]
    fn register_existing() {
        let mut crdt = MetadataCrdt::new();
        crdt.register_metadata("title", Some("Doc".into()), OpId::new(0, 1));
        assert_eq!(crdt.get_metadata("title"), Some("Doc"));

        let style = Style::new("Normal", "Normal", StyleType::Paragraph);
        crdt.register_style(&style, OpId::new(0, 1));
        assert!(crdt.get_style("Normal").is_some());
    }
}
