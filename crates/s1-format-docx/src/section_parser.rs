//! Parse `w:sectPr` elements into section properties.
//!
//! The parser returns a [`RawSectionProperties`] with header/footer rId strings.
//! The reader resolves rIds to actual NodeIds after parsing header/footer XML files.

use quick_xml::events::Event;
use quick_xml::Reader;
use s1_model::{PageOrientation, SectionBreakType, SectionProperties};

use crate::error::DocxError;
use crate::xml_util::{get_attr, twips_to_points};

/// A raw header/footer reference before rId resolution.
#[derive(Debug, Clone)]
pub struct RawHeaderFooterRef {
    /// "default", "first", or "even"
    pub hf_type: String,
    /// The rId (e.g., "rId6")
    pub rid: String,
    /// Whether this is a header (true) or footer (false).
    pub is_header: bool,
}

/// Section properties with raw rId references (before NodeId resolution).
#[derive(Debug, Clone, Default)]
pub struct RawSectionProperties {
    /// The parsed section properties (without headers/footers populated).
    pub props: SectionProperties,
    /// Raw header/footer rId references to resolve later.
    pub hf_refs: Vec<RawHeaderFooterRef>,
}

/// Parse a `<w:sectPr>` element. The reader should be positioned just after
/// reading the `Start(sectPr)` event.
pub fn parse_section_properties(
    reader: &mut Reader<&[u8]>,
) -> Result<RawSectionProperties, DocxError> {
    let mut props = SectionProperties::default();
    let mut hf_refs: Vec<RawHeaderFooterRef> = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"pgSz" => {
                        // w:w and w:h are in twips
                        if let Some(w) = get_attr(&e, b"w").and_then(|v| twips_to_points(&v)) {
                            props.page_width = w;
                        }
                        if let Some(h) = get_attr(&e, b"h").and_then(|v| twips_to_points(&v)) {
                            props.page_height = h;
                        }
                        if let Some(orient) = get_attr(&e, b"orient") {
                            if orient == "landscape" {
                                props.orientation = PageOrientation::Landscape;
                            }
                        }
                        skip_element(reader)?;
                    }
                    b"pgMar" => {
                        parse_margins(&e, &mut props);
                        skip_element(reader)?;
                    }
                    b"cols" => {
                        if let Some(num) = get_attr(&e, b"num").and_then(|v| v.parse::<u32>().ok())
                        {
                            props.columns = num;
                        }
                        if let Some(space) =
                            get_attr(&e, b"space").and_then(|v| twips_to_points(&v))
                        {
                            props.column_spacing = space;
                        }
                        if let Some(eq) = get_attr(&e, b"equalWidth") {
                            props.equal_width = eq == "1" || eq == "true";
                        }
                        skip_element(reader)?;
                    }
                    b"type" => {
                        if let Some(val) = get_attr(&e, b"val") {
                            props.break_type = parse_section_break_type(&val);
                        }
                        skip_element(reader)?;
                    }
                    _ => {
                        #[cfg(debug_assertions)]
                        {
                            let tag = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                            if matches!(
                                tag.as_str(),
                                "pgBorders" | "lnNumType" | "docGrid" | "vAlign"
                            ) {
                                eprintln!(
                                    "[s1-format-docx] Note: section property <w:{tag}> skipped (not yet modeled)"
                                );
                            }
                        }
                        skip_element(reader)?;
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                let name = e.local_name().as_ref().to_vec();
                match name.as_slice() {
                    b"pgSz" => {
                        if let Some(w) = get_attr(&e, b"w").and_then(|v| twips_to_points(&v)) {
                            props.page_width = w;
                        }
                        if let Some(h) = get_attr(&e, b"h").and_then(|v| twips_to_points(&v)) {
                            props.page_height = h;
                        }
                        if let Some(orient) = get_attr(&e, b"orient") {
                            if orient == "landscape" {
                                props.orientation = PageOrientation::Landscape;
                            }
                        }
                    }
                    b"pgMar" => {
                        parse_margins(&e, &mut props);
                    }
                    b"cols" => {
                        if let Some(num) = get_attr(&e, b"num").and_then(|v| v.parse::<u32>().ok())
                        {
                            props.columns = num;
                        }
                        if let Some(space) =
                            get_attr(&e, b"space").and_then(|v| twips_to_points(&v))
                        {
                            props.column_spacing = space;
                        }
                        if let Some(eq) = get_attr(&e, b"equalWidth") {
                            props.equal_width = eq == "1" || eq == "true";
                        }
                    }
                    b"type" => {
                        if let Some(val) = get_attr(&e, b"val") {
                            props.break_type = parse_section_break_type(&val);
                        }
                    }
                    b"headerReference" => {
                        if let (Some(hf_type), Some(rid)) =
                            (get_attr(&e, b"type"), get_attr(&e, b"id"))
                        {
                            hf_refs.push(RawHeaderFooterRef {
                                hf_type,
                                rid,
                                is_header: true,
                            });
                        }
                    }
                    b"footerReference" => {
                        if let (Some(hf_type), Some(rid)) =
                            (get_attr(&e, b"type"), get_attr(&e, b"id"))
                        {
                            hf_refs.push(RawHeaderFooterRef {
                                hf_type,
                                rid,
                                is_header: false,
                            });
                        }
                    }
                    b"titlePg" => {
                        props.title_page = true;
                    }
                    b"evenAndOddHeaders" => {
                        props.even_and_odd_headers = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"sectPr" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }

    Ok(RawSectionProperties { props, hf_refs })
}

/// Parse `w:pgMar` attributes.
fn parse_margins(e: &quick_xml::events::BytesStart<'_>, props: &mut SectionProperties) {
    if let Some(v) = get_attr(e, b"top").and_then(|v| twips_to_points(&v)) {
        props.margin_top = v;
    }
    if let Some(v) = get_attr(e, b"bottom").and_then(|v| twips_to_points(&v)) {
        props.margin_bottom = v;
    }
    if let Some(v) = get_attr(e, b"left").and_then(|v| twips_to_points(&v)) {
        props.margin_left = v;
    }
    if let Some(v) = get_attr(e, b"right").and_then(|v| twips_to_points(&v)) {
        props.margin_right = v;
    }
    if let Some(v) = get_attr(e, b"header").and_then(|v| twips_to_points(&v)) {
        props.header_distance = v;
    }
    if let Some(v) = get_attr(e, b"footer").and_then(|v| twips_to_points(&v)) {
        props.footer_distance = v;
    }
}

/// Parse a section break type string.
fn parse_section_break_type(val: &str) -> Option<SectionBreakType> {
    match val {
        "nextPage" => Some(SectionBreakType::NextPage),
        "continuous" => Some(SectionBreakType::Continuous),
        "evenPage" => Some(SectionBreakType::EvenPage),
        "oddPage" => Some(SectionBreakType::OddPage),
        _ => None,
    }
}

/// Convert a header/footer type string to the model enum.
pub fn parse_hf_type(val: &str) -> s1_model::HeaderFooterType {
    match val {
        "first" => s1_model::HeaderFooterType::First,
        "even" => s1_model::HeaderFooterType::Even,
        _ => s1_model::HeaderFooterType::Default,
    }
}

/// Skip an element and all its children.
fn skip_element(reader: &mut Reader<&[u8]>) -> Result<(), DocxError> {
    let mut depth = 1u32;
    loop {
        match reader.read_event() {
            Ok(Event::Start(_)) => depth += 1,
            Ok(Event::End(_)) => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(format!("{e}"))),
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_sect_pr(xml: &str) -> RawSectionProperties {
        let mut reader = Reader::from_str(xml);
        // Advance past the start tag (handle both Start and Empty)
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) if e.local_name().as_ref() == b"sectPr" => break,
                Ok(Event::Empty(e)) if e.local_name().as_ref() == b"sectPr" => {
                    // Self-closing sectPr — return defaults
                    return RawSectionProperties::default();
                }
                Ok(Event::Eof) => panic!("unexpected EOF"),
                _ => {}
            }
        }
        parse_section_properties(&mut reader).unwrap()
    }

    #[test]
    fn parse_minimal_section() {
        // Self-closing sectPr should give defaults
        let _raw = parse_sect_pr(
            r#"<w:sectPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"/>"#,
        );

        // Start/end sectPr should also give defaults
        let raw = parse_sect_pr(
            r#"<w:sectPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            </w:sectPr>"#,
        );
        let _ = raw;
        assert!(raw.props.break_type.is_none());
        assert_eq!(raw.hf_refs.len(), 0);
    }

    #[test]
    fn parse_page_size_and_margins() {
        let raw = parse_sect_pr(
            r#"<w:sectPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
                <w:pgSz w:w="15840" w:h="12240" w:orient="landscape"/>
                <w:pgMar w:top="1440" w:bottom="1440" w:left="1800" w:right="1800" w:header="720" w:footer="720"/>
            </w:sectPr>"#,
        );
        // 15840 twips = 792pt, 12240 twips = 612pt
        assert!((raw.props.page_width - 792.0).abs() < 0.01);
        assert!((raw.props.page_height - 612.0).abs() < 0.01);
        assert_eq!(raw.props.orientation, PageOrientation::Landscape);
        // 1440 twips = 72pt, 1800 twips = 90pt
        assert!((raw.props.margin_top - 72.0).abs() < 0.01);
        assert!((raw.props.margin_left - 90.0).abs() < 0.01);
        assert!((raw.props.header_distance - 36.0).abs() < 0.01);
    }

    #[test]
    fn parse_columns() {
        let raw = parse_sect_pr(
            r#"<w:sectPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
                <w:cols w:num="2" w:space="720"/>
            </w:sectPr>"#,
        );
        assert_eq!(raw.props.columns, 2);
        assert!((raw.props.column_spacing - 36.0).abs() < 0.01); // 720 twips = 36pt
                                                                 // Default equal_width is true, w:equalWidth not present keeps it true
        assert!(raw.props.equal_width);
    }

    #[test]
    fn parse_columns_with_equal_width() {
        let raw = parse_sect_pr(
            r#"<w:sectPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
                <w:cols w:num="3" w:space="360" w:equalWidth="1"/>
            </w:sectPr>"#,
        );
        assert_eq!(raw.props.columns, 3);
        assert!((raw.props.column_spacing - 18.0).abs() < 0.01); // 360 twips = 18pt
        assert!(raw.props.equal_width);
    }

    #[test]
    fn parse_columns_unequal_width() {
        let raw = parse_sect_pr(
            r#"<w:sectPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
                <w:cols w:num="2" w:space="720" w:equalWidth="0"/>
            </w:sectPr>"#,
        );
        assert_eq!(raw.props.columns, 2);
        assert!(!raw.props.equal_width);
    }

    #[test]
    fn parse_section_break() {
        let raw = parse_sect_pr(
            r#"<w:sectPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
                <w:type w:val="continuous"/>
            </w:sectPr>"#,
        );
        assert_eq!(raw.props.break_type, Some(SectionBreakType::Continuous));
    }

    #[test]
    fn parse_header_footer_references() {
        let raw = parse_sect_pr(
            r#"<w:sectPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
                         xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
                <w:headerReference w:type="default" r:id="rId6"/>
                <w:headerReference w:type="first" r:id="rId7"/>
                <w:footerReference w:type="default" r:id="rId8"/>
                <w:titlePg/>
            </w:sectPr>"#,
        );
        assert_eq!(raw.hf_refs.len(), 3);
        assert!(raw.props.title_page);

        let default_hdr = raw
            .hf_refs
            .iter()
            .find(|r| r.is_header && r.hf_type == "default")
            .unwrap();
        assert_eq!(default_hdr.rid, "rId6");

        let first_hdr = raw
            .hf_refs
            .iter()
            .find(|r| r.is_header && r.hf_type == "first")
            .unwrap();
        assert_eq!(first_hdr.rid, "rId7");

        let default_ftr = raw
            .hf_refs
            .iter()
            .find(|r| !r.is_header && r.hf_type == "default")
            .unwrap();
        assert_eq!(default_ftr.rid, "rId8");
    }

    #[test]
    fn parse_even_odd_section_break() {
        let raw = parse_sect_pr(
            r#"<w:sectPr xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
                <w:type w:val="evenPage"/>
            </w:sectPr>"#,
        );
        assert_eq!(raw.props.break_type, Some(SectionBreakType::EvenPage));
    }
}
