//! Document metadata (title, author, dates, etc.).

use std::collections::HashMap;

/// Document-level metadata. Maps to Dublin Core properties in DOCX/ODT.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub subject: Option<String>,
    pub creator: Option<String>,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    /// ISO 8601 datetime string.
    pub created: Option<String>,
    /// ISO 8601 datetime string.
    pub modified: Option<String>,
    pub revision: Option<u32>,
    /// BCP 47 language tag (e.g., "en-US").
    pub language: Option<String>,
    /// Application-specific custom properties.
    pub custom_properties: HashMap<String, String>,
    /// Whether track changes mode is active.
    pub track_changes: bool,
}

impl DocumentMetadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_creator(mut self, creator: impl Into<String>) -> Self {
        self.creator = Some(creator.into());
        self
    }

    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_metadata() {
        let meta = DocumentMetadata::new();
        assert!(meta.title.is_none());
        assert!(meta.creator.is_none());
        assert!(meta.keywords.is_empty());
        assert!(meta.custom_properties.is_empty());
    }

    #[test]
    fn builder_style() {
        let meta = DocumentMetadata::new()
            .with_title("My Document")
            .with_creator("Author")
            .with_language("en-US");
        assert_eq!(meta.title.as_deref(), Some("My Document"));
        assert_eq!(meta.creator.as_deref(), Some("Author"));
        assert_eq!(meta.language.as_deref(), Some("en-US"));
    }
}
