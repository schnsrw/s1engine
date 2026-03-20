//! Generate `w:sectPr` XML from section properties.

use s1_model::{HeaderFooterType, PageOrientation, SectionBreakType, SectionProperties};

/// A relationship entry for a header or footer to include in the sectPr.
pub struct HfRelEntry {
    /// The relationship ID (e.g., "rHdr1").
    pub rid: String,
    /// Header or footer type.
    pub hf_type: HeaderFooterType,
    /// Whether this is a header (true) or footer (false).
    pub is_header: bool,
}

/// Generate `w:sectPr` XML content (without the outer tags — caller wraps).
pub fn write_section_properties(props: &SectionProperties, hf_rels: &[HfRelEntry]) -> String {
    let mut xml = String::new();

    // Header references
    for rel in hf_rels.iter().filter(|r| r.is_header) {
        let type_str = hf_type_to_str(rel.hf_type);
        xml.push_str(&format!(
            r#"<w:headerReference w:type="{type_str}" r:id="{}"/>"#,
            rel.rid
        ));
    }

    // Footer references
    for rel in hf_rels.iter().filter(|r| !r.is_header) {
        let type_str = hf_type_to_str(rel.hf_type);
        xml.push_str(&format!(
            r#"<w:footerReference w:type="{type_str}" r:id="{}"/>"#,
            rel.rid
        ));
    }

    // Section break type
    if let Some(break_type) = &props.break_type {
        let val = match break_type {
            SectionBreakType::NextPage => "nextPage",
            SectionBreakType::Continuous => "continuous",
            SectionBreakType::EvenPage => "evenPage",
            SectionBreakType::OddPage => "oddPage",
            _ => "nextPage",
        };
        xml.push_str(&format!(r#"<w:type w:val="{val}"/>"#));
    }

    // Page size
    let orient = if props.orientation == PageOrientation::Landscape {
        r#" w:orient="landscape""#
    } else {
        ""
    };
    xml.push_str(&format!(
        r#"<w:pgSz w:w="{}" w:h="{}"{orient}/>"#,
        points_to_twips(props.page_width),
        points_to_twips(props.page_height),
    ));

    // Page margins
    xml.push_str(&format!(
        r#"<w:pgMar w:top="{}" w:right="{}" w:bottom="{}" w:left="{}" w:header="{}" w:footer="{}"/>"#,
        points_to_twips(props.margin_top),
        points_to_twips(props.margin_right),
        points_to_twips(props.margin_bottom),
        points_to_twips(props.margin_left),
        points_to_twips(props.header_distance),
        points_to_twips(props.footer_distance),
    ));

    // Columns
    if props.columns > 1 {
        let equal_width_val = if props.equal_width { "1" } else { "0" };
        xml.push_str(&format!(
            r#"<w:cols w:num="{}" w:space="{}" w:equalWidth="{}"/>"#,
            props.columns,
            points_to_twips(props.column_spacing),
            equal_width_val,
        ));
    }

    // Even and odd headers flag
    if props.even_and_odd_headers {
        xml.push_str("<w:evenAndOddHeaders/>");
    }

    // Title page flag
    if props.title_page {
        xml.push_str("<w:titlePg/>");
    }

    xml
}

fn hf_type_to_str(hf_type: HeaderFooterType) -> &'static str {
    match hf_type {
        HeaderFooterType::Default => "default",
        HeaderFooterType::First => "first",
        HeaderFooterType::Even => "even",
        _ => "default",
    }
}

fn points_to_twips(pts: f64) -> i64 {
    (pts * 20.0) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_default_section() {
        let props = SectionProperties::default();
        let xml = write_section_properties(&props, &[]);

        // US Letter: 612pt = 12240 twips, 792pt = 15840 twips
        assert!(xml.contains(r#"w:w="12240""#));
        assert!(xml.contains(r#"w:h="15840""#));
        assert!(!xml.contains("w:orient"));
        assert!(xml.contains("w:pgMar"));
        assert!(!xml.contains("w:cols")); // 1 column = no cols element
        assert!(!xml.contains("w:type")); // no break type
        assert!(!xml.contains("w:titlePg")); // no title page
    }

    #[test]
    fn write_landscape_section() {
        let mut props = SectionProperties::default();
        props.orientation = PageOrientation::Landscape;
        props.page_width = 792.0;
        props.page_height = 612.0;

        let xml = write_section_properties(&props, &[]);
        assert!(xml.contains(r#"w:orient="landscape""#));
        assert!(xml.contains(r#"w:w="15840""#));
    }

    #[test]
    fn write_section_with_break() {
        let mut props = SectionProperties::default();
        props.break_type = Some(SectionBreakType::Continuous);

        let xml = write_section_properties(&props, &[]);
        assert!(xml.contains(r#"<w:type w:val="continuous"/>"#));
    }

    #[test]
    fn write_section_with_columns() {
        let mut props = SectionProperties::default();
        props.columns = 2;
        props.column_spacing = 36.0;

        let xml = write_section_properties(&props, &[]);
        assert!(xml.contains(r#"w:num="2""#));
        assert!(xml.contains(r#"w:space="720""#)); // 36pt = 720 twips
        assert!(xml.contains(r#"w:equalWidth="1""#));
    }

    #[test]
    fn write_section_with_unequal_columns() {
        let mut props = SectionProperties::default();
        props.columns = 3;
        props.column_spacing = 18.0;
        props.equal_width = false;

        let xml = write_section_properties(&props, &[]);
        assert!(xml.contains(r#"w:num="3""#));
        assert!(xml.contains(r#"w:equalWidth="0""#));
    }

    #[test]
    fn write_section_with_header_footer_refs() {
        let props = SectionProperties {
            title_page: true,
            ..SectionProperties::default()
        };
        let hf_rels = vec![
            HfRelEntry {
                rid: "rHdr1".to_string(),
                hf_type: HeaderFooterType::Default,
                is_header: true,
            },
            HfRelEntry {
                rid: "rHdr2".to_string(),
                hf_type: HeaderFooterType::First,
                is_header: true,
            },
            HfRelEntry {
                rid: "rFtr1".to_string(),
                hf_type: HeaderFooterType::Default,
                is_header: false,
            },
        ];

        let xml = write_section_properties(&props, &hf_rels);
        assert!(xml.contains(r#"<w:headerReference w:type="default" r:id="rHdr1"/>"#));
        assert!(xml.contains(r#"<w:headerReference w:type="first" r:id="rHdr2"/>"#));
        assert!(xml.contains(r#"<w:footerReference w:type="default" r:id="rFtr1"/>"#));
        assert!(xml.contains("<w:titlePg/>"));
    }
}
