//! Numbering definitions for list support.
//!
//! In OOXML, lists work through a two-level indirection:
//! - `AbstractNumbering` defines the numbering patterns (bullet/decimal/etc per level)
//! - `NumberingInstance` references an abstract numbering and can override specific levels
//! - Paragraphs reference a `NumberingInstance` via `numId` + `ilvl` (stored in `ListInfo`)

use crate::attributes::{Alignment, ListFormat};

/// A single level definition within an abstract numbering.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct NumberingLevel {
    /// Level index (0-8).
    pub level: u8,
    /// Numbering format at this level.
    pub num_format: ListFormat,
    /// Level text pattern, e.g. "%1.", "%1.%2."
    pub level_text: String,
    /// Start value (default 1).
    pub start: u32,
    /// Left indentation in points.
    pub indent_left: Option<f64>,
    /// Hanging indent in points.
    pub indent_hanging: Option<f64>,
    /// Justification of the number.
    pub alignment: Option<Alignment>,
    /// Font for the bullet character (e.g., "Symbol", "Wingdings").
    pub bullet_font: Option<String>,
}

/// An abstract numbering definition (the template for a list style).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct AbstractNumbering {
    /// Unique ID for this abstract numbering.
    pub abstract_num_id: u32,
    /// Human-readable name (optional).
    pub name: Option<String>,
    /// Per-level definitions (typically up to 9 levels, 0-8).
    pub levels: Vec<NumberingLevel>,
}

/// A numbering instance that references an abstract numbering.
/// Paragraphs point to this via `numId` in `ListInfo`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct NumberingInstance {
    /// The numId referenced by paragraphs.
    pub num_id: u32,
    /// The abstract numbering this instance is based on.
    pub abstract_num_id: u32,
    /// Per-level overrides.
    pub level_overrides: Vec<LevelOverride>,
}

/// An override for a specific level in a numbering instance.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct LevelOverride {
    /// Level index being overridden.
    pub level: u8,
    /// Override start number.
    pub start_override: Option<u32>,
    /// Full level definition override (replaces abstract level).
    pub level_def: Option<NumberingLevel>,
}

/// Container for all numbering definitions in a document.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NumberingDefinitions {
    pub abstract_nums: Vec<AbstractNumbering>,
    pub instances: Vec<NumberingInstance>,
}

impl NumberingDefinitions {
    /// Check if there are any numbering definitions.
    pub fn is_empty(&self) -> bool {
        self.abstract_nums.is_empty() && self.instances.is_empty()
    }

    /// Resolve the `ListFormat` for a given `numId` and level.
    pub fn resolve_format(&self, num_id: u32, level: u8) -> Option<ListFormat> {
        // Find the numbering instance
        let instance = self.instances.iter().find(|n| n.num_id == num_id)?;

        // Check for level override first
        if let Some(ovr) = instance.level_overrides.iter().find(|o| o.level == level) {
            if let Some(ref lvl_def) = ovr.level_def {
                return Some(lvl_def.num_format);
            }
        }

        // Fall back to the abstract numbering
        let abstract_num = self
            .abstract_nums
            .iter()
            .find(|a| a.abstract_num_id == instance.abstract_num_id)?;

        abstract_num
            .levels
            .iter()
            .find(|l| l.level == level)
            .map(|l| l.num_format)
    }

    /// Resolve the start value for a given `numId` and level.
    pub fn resolve_start(&self, num_id: u32, level: u8) -> Option<u32> {
        let instance = self.instances.iter().find(|n| n.num_id == num_id)?;

        // Check for start override
        if let Some(ovr) = instance.level_overrides.iter().find(|o| o.level == level) {
            if let Some(start) = ovr.start_override {
                return Some(start);
            }
            if let Some(ref lvl_def) = ovr.level_def {
                return Some(lvl_def.start);
            }
        }

        let abstract_num = self
            .abstract_nums
            .iter()
            .find(|a| a.abstract_num_id == instance.abstract_num_id)?;

        abstract_num
            .levels
            .iter()
            .find(|l| l.level == level)
            .map(|l| l.start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bullet_defs() -> NumberingDefinitions {
        NumberingDefinitions {
            abstract_nums: vec![AbstractNumbering {
                abstract_num_id: 0,
                name: Some("BulletList".into()),
                levels: vec![NumberingLevel {
                    level: 0,
                    num_format: ListFormat::Bullet,
                    level_text: "\u{F0B7}".into(),
                    start: 1,
                    indent_left: Some(36.0),
                    indent_hanging: Some(18.0),
                    alignment: None,
                    bullet_font: Some("Symbol".into()),
                }],
            }],
            instances: vec![NumberingInstance {
                num_id: 1,
                abstract_num_id: 0,
                level_overrides: vec![],
            }],
        }
    }

    #[test]
    fn resolve_bullet_format() {
        let defs = make_bullet_defs();
        assert_eq!(defs.resolve_format(1, 0), Some(ListFormat::Bullet));
    }

    #[test]
    fn resolve_missing_num_id() {
        let defs = make_bullet_defs();
        assert_eq!(defs.resolve_format(99, 0), None);
    }

    #[test]
    fn resolve_missing_level() {
        let defs = make_bullet_defs();
        assert_eq!(defs.resolve_format(1, 5), None);
    }

    #[test]
    fn resolve_with_level_override() {
        let mut defs = NumberingDefinitions {
            abstract_nums: vec![AbstractNumbering {
                abstract_num_id: 0,
                name: None,
                levels: vec![NumberingLevel {
                    level: 0,
                    num_format: ListFormat::Decimal,
                    level_text: "%1.".into(),
                    start: 1,
                    indent_left: None,
                    indent_hanging: None,
                    alignment: None,
                    bullet_font: None,
                }],
            }],
            instances: vec![NumberingInstance {
                num_id: 1,
                abstract_num_id: 0,
                level_overrides: vec![LevelOverride {
                    level: 0,
                    start_override: Some(5),
                    level_def: None,
                }],
            }],
        };

        // Format should still come from abstract (override only changes start)
        assert_eq!(defs.resolve_format(1, 0), Some(ListFormat::Decimal));
        assert_eq!(defs.resolve_start(1, 0), Some(5));

        // Now add a full level override that changes the format
        defs.instances[0].level_overrides[0].level_def = Some(NumberingLevel {
            level: 0,
            num_format: ListFormat::LowerRoman,
            level_text: "%1)".into(),
            start: 1,
            indent_left: None,
            indent_hanging: None,
            alignment: None,
            bullet_font: None,
        });
        assert_eq!(defs.resolve_format(1, 0), Some(ListFormat::LowerRoman));
    }

    #[test]
    fn is_empty() {
        assert!(NumberingDefinitions::default().is_empty());
        assert!(!make_bullet_defs().is_empty());
    }
}
