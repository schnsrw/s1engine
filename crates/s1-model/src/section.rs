//! Section properties for page layout, headers, and footers.
//!
//! In OOXML, `w:sectPr` defines page size, margins, orientation, columns,
//! and references to header/footer parts. A document can have multiple
//! sections with different layouts.

use crate::id::NodeId;

/// Type of section break.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SectionBreakType {
    /// Start the next section on a new page (default).
    NextPage,
    /// Start the next section on the same page.
    Continuous,
    /// Start the next section on the next even page.
    EvenPage,
    /// Start the next section on the next odd page.
    OddPage,
}

/// Type of header/footer reference.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HeaderFooterType {
    /// Used on all pages (unless first/even overrides are active).
    Default,
    /// Used on the first page only (requires `title_page = true`).
    First,
    /// Used on even pages (requires even/odd differentiation).
    Even,
}

/// A reference to a header or footer stored as a node in the document model.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct HeaderFooterRef {
    /// The type (default, first, even).
    pub hf_type: HeaderFooterType,
    /// The NodeId of the Header or Footer node in the document tree.
    pub node_id: NodeId,
}

/// Section properties — maps to `w:sectPr` in OOXML.
///
/// Each section defines page layout and references to headers/footers.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct SectionProperties {
    /// Page width in points (default: 612.0 = 8.5 inches).
    pub page_width: f64,
    /// Page height in points (default: 792.0 = 11 inches).
    pub page_height: f64,
    /// Page orientation.
    pub orientation: crate::attributes::PageOrientation,
    /// Top margin in points.
    pub margin_top: f64,
    /// Bottom margin in points.
    pub margin_bottom: f64,
    /// Left margin in points.
    pub margin_left: f64,
    /// Right margin in points.
    pub margin_right: f64,
    /// Header distance from top edge in points.
    pub header_distance: f64,
    /// Footer distance from bottom edge in points.
    pub footer_distance: f64,
    /// Number of columns (default: 1).
    pub columns: u32,
    /// Column spacing in points.
    pub column_spacing: f64,
    /// Whether all columns have equal width (default: true).
    pub equal_width: bool,
    /// Section break type. `None` for the final section.
    pub break_type: Option<SectionBreakType>,
    /// Header references for this section.
    pub headers: Vec<HeaderFooterRef>,
    /// Footer references for this section.
    pub footers: Vec<HeaderFooterRef>,
    /// Whether first-page header/footer is enabled (`w:titlePg`).
    pub title_page: bool,
    /// Whether even and odd page headers/footers are differentiated (`w:evenAndOddHeaders`).
    ///
    /// When `true`, even-page headers/footers use the `Even` type references,
    /// and odd pages use the `Default` type. This maps to the `w:evenAndOddHeaders`
    /// element in OOXML settings or section properties.
    pub even_and_odd_headers: bool,
    /// Page borders (`w:pgBorders`).
    pub page_borders: Option<crate::attributes::Borders>,
    /// Document grid type (`w:docGrid/@type`): "default", "lines", "linesAndChars", "snapToChars".
    pub doc_grid_type: Option<String>,
    /// Document grid line pitch in points (`w:docGrid/@linePitch`).
    pub doc_grid_line_pitch: Option<f64>,
    /// Line numbering start value (`w:lnNumType/@start`). None = no line numbering.
    pub line_numbering_start: Option<u32>,
    /// Line numbering count-by increment (`w:lnNumType/@countBy`).
    pub line_numbering_count_by: Option<u32>,
    /// Line numbering restart mode: "newPage", "newSection", "continuous".
    pub line_numbering_restart: Option<String>,
}

impl Default for SectionProperties {
    /// US Letter portrait, 1-inch margins.
    fn default() -> Self {
        Self {
            page_width: 612.0,  // 8.5 inches
            page_height: 792.0, // 11 inches
            orientation: crate::attributes::PageOrientation::Portrait,
            margin_top: 72.0, // 1 inch
            margin_bottom: 72.0,
            margin_left: 72.0,
            margin_right: 72.0,
            header_distance: 36.0, // 0.5 inch
            footer_distance: 36.0,
            columns: 1,
            column_spacing: 36.0,
            equal_width: true,
            break_type: None,
            headers: Vec::new(),
            footers: Vec::new(),
            title_page: false,
            even_and_odd_headers: false,
            page_borders: None,
            doc_grid_type: None,
            doc_grid_line_pitch: None,
            line_numbering_start: None,
            line_numbering_count_by: None,
            line_numbering_restart: None,
        }
    }
}

impl SectionProperties {
    /// Get the header reference for the given type, if any.
    pub fn header(&self, hf_type: HeaderFooterType) -> Option<&HeaderFooterRef> {
        self.headers.iter().find(|h| h.hf_type == hf_type)
    }

    /// Get the footer reference for the given type, if any.
    pub fn footer(&self, hf_type: HeaderFooterType) -> Option<&HeaderFooterRef> {
        self.footers.iter().find(|f| f.hf_type == hf_type)
    }

    /// Check if this section has any headers.
    pub fn has_headers(&self) -> bool {
        !self.headers.is_empty()
    }

    /// Check if this section has any footers.
    pub fn has_footers(&self) -> bool {
        !self.footers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::PageOrientation;

    #[test]
    fn default_section_properties() {
        let props = SectionProperties::default();
        assert!((props.page_width - 612.0).abs() < 0.01);
        assert!((props.page_height - 792.0).abs() < 0.01);
        assert_eq!(props.orientation, PageOrientation::Portrait);
        assert!((props.margin_top - 72.0).abs() < 0.01);
        assert_eq!(props.columns, 1);
        assert!((props.column_spacing - 36.0).abs() < 0.01);
        assert!(props.equal_width);
        assert!(props.break_type.is_none());
        assert!(!props.title_page);
        assert!(props.headers.is_empty());
        assert!(props.footers.is_empty());
    }

    #[test]
    fn multi_column_properties() {
        let mut props = SectionProperties::default();
        props.columns = 3;
        props.column_spacing = 18.0;
        props.equal_width = false;
        assert_eq!(props.columns, 3);
        assert!((props.column_spacing - 18.0).abs() < 0.01);
        assert!(!props.equal_width);
    }

    #[test]
    fn section_break_types() {
        let mut props = SectionProperties::default();
        props.break_type = Some(SectionBreakType::Continuous);
        assert_eq!(props.break_type, Some(SectionBreakType::Continuous));

        props.break_type = Some(SectionBreakType::EvenPage);
        assert_eq!(props.break_type, Some(SectionBreakType::EvenPage));
    }

    #[test]
    fn header_footer_refs() {
        let mut props = SectionProperties::default();
        let header_id = NodeId::new(0, 100);
        let footer_id = NodeId::new(0, 101);

        props.headers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: header_id,
        });
        props.footers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: footer_id,
        });

        assert!(props.has_headers());
        assert!(props.has_footers());
        assert_eq!(
            props.header(HeaderFooterType::Default).unwrap().node_id,
            header_id
        );
        assert_eq!(
            props.footer(HeaderFooterType::Default).unwrap().node_id,
            footer_id
        );
        assert!(props.header(HeaderFooterType::First).is_none());
    }

    #[test]
    fn first_page_header_footer() {
        let mut props = SectionProperties::default();
        props.title_page = true;

        let default_hdr = NodeId::new(0, 10);
        let first_hdr = NodeId::new(0, 11);

        props.headers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: default_hdr,
        });
        props.headers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::First,
            node_id: first_hdr,
        });

        assert!(props.title_page);
        assert_eq!(props.headers.len(), 2);
        assert_eq!(
            props.header(HeaderFooterType::First).unwrap().node_id,
            first_hdr
        );
    }
}
