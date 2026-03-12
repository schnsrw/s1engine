//! Parse ODF styles (named styles from `styles.xml`, automatic styles from `content.xml`).

use std::collections::HashMap;

use quick_xml::events::Event;
use quick_xml::Reader;
use s1_model::{AttributeMap, DocumentModel, Style, StyleType};

use crate::error::OdtError;
use crate::property_parser::{
    parse_paragraph_properties, parse_paragraph_properties_children, parse_text_properties,
};
use crate::xml_util::{get_attr, parse_length};

/// Page layout properties extracted from `<style:page-layout>` in styles.xml.
#[derive(Debug, Clone, Default)]
pub struct OdtPageLayout {
    /// Page width in points.
    pub page_width: Option<f64>,
    /// Page height in points.
    pub page_height: Option<f64>,
    /// Orientation string ("portrait" or "landscape").
    pub orientation: Option<String>,
    /// Top margin in points.
    pub margin_top: Option<f64>,
    /// Bottom margin in points.
    pub margin_bottom: Option<f64>,
    /// Left margin in points.
    pub margin_left: Option<f64>,
    /// Right margin in points.
    pub margin_right: Option<f64>,
}

/// A segment of text in a header/footer paragraph.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum HfSegment {
    /// Plain text.
    Text(String),
    /// Page number field.
    PageNumber,
    /// Page count field.
    PageCount,
}

/// Parsed header/footer content from a master page.
#[derive(Debug, Clone, Default)]
pub struct HfContent {
    /// Each paragraph is a list of segments.
    pub paragraphs: Vec<Vec<HfSegment>>,
}

/// Master page info parsed from `<style:master-page>` in styles.xml.
#[derive(Debug, Clone, Default)]
pub struct OdtMasterPage {
    /// Page layout properties (dimensions, margins, orientation).
    pub layout: OdtPageLayout,
    /// Default header content.
    pub header: Option<HfContent>,
    /// Default footer content.
    pub footer: Option<HfContent>,
    /// First-page header content (ODF 1.3).
    pub first_header: Option<HfContent>,
    /// First-page footer content (ODF 1.3).
    pub first_footer: Option<HfContent>,
}

/// Parse `styles.xml` and populate `doc` with named styles.
///
/// Also extracts page layout and master page info (headers/footers).
/// Returns `Some(OdtMasterPage)` if page layout or header/footer info was found.
pub fn parse_styles_xml(
    xml: &str,
    doc: &mut DocumentModel,
) -> Result<Option<OdtMasterPage>, OdtError> {
    let mut reader = Reader::from_str(xml);
    let mut page_layouts: HashMap<String, OdtPageLayout> = HashMap::new();
    let mut master_page_info: Option<OdtMasterPage> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => match e.local_name().as_ref() {
                b"style" => {
                    if let Some(style) = parse_style_element(&mut reader, &e)? {
                        doc.set_style(style);
                    }
                }
                b"page-layout" => {
                    let name = get_attr(&e, b"name").unwrap_or_default();
                    let layout = parse_page_layout(&mut reader)?;
                    if !name.is_empty() {
                        page_layouts.insert(name, layout);
                    }
                }
                b"master-page" => {
                    let layout_name = get_attr(&e, b"page-layout-name").unwrap_or_default();
                    let layout = page_layouts.get(&layout_name).cloned().unwrap_or_default();
                    master_page_info = Some(parse_master_page(&mut reader, layout)?);
                }
                _ => {}
            },
            Ok(Event::Empty(e)) => match e.local_name().as_ref() {
                b"style" => {
                    if let Some(style) = parse_empty_style(&e) {
                        doc.set_style(style);
                    }
                }
                b"master-page" => {
                    let layout_name = get_attr(&e, b"page-layout-name").unwrap_or_default();
                    let layout = page_layouts.get(&layout_name).cloned().unwrap_or_default();
                    master_page_info = Some(OdtMasterPage {
                        layout,
                        ..Default::default()
                    });
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }
    Ok(master_page_info)
}

/// Parse automatic styles from a reader positioned inside `<office:automatic-styles>`.
///
/// Returns a map of style name → merged attributes (paragraph + text props combined).
pub fn parse_automatic_styles(
    reader: &mut Reader<&[u8]>,
) -> Result<HashMap<String, AttributeMap>, OdtError> {
    let mut auto_styles: HashMap<String, AttributeMap> = HashMap::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"style" => {
                let name = get_attr(e, b"name").unwrap_or_default();
                let family = get_attr(e, b"family").unwrap_or_default();
                let parent = get_attr(e, b"parent-style-name");

                let mut attrs = AttributeMap::new();

                // Record parent style reference
                if let Some(ref parent_name) = parent {
                    attrs.set(
                        s1_model::AttributeKey::StyleId,
                        s1_model::AttributeValue::String(parent_name.clone()),
                    );
                }

                // Parse child property elements
                loop {
                    match reader.read_event() {
                        Ok(Event::Start(ref pe)) => {
                            let local = pe.local_name();
                            match local.as_ref() {
                                b"text-properties" => {
                                    attrs.merge(&parse_text_properties(pe));
                                    // Skip to end of text-properties
                                    skip_to_end(reader, b"text-properties")?;
                                }
                                b"paragraph-properties" => {
                                    let mut para_attrs = parse_paragraph_properties(pe);
                                    // Parse children (tab stops, etc.)
                                    parse_paragraph_properties_children(reader, &mut para_attrs);
                                    attrs.merge(&para_attrs);
                                }
                                _ => {}
                            }
                        }
                        Ok(Event::Empty(ref pe)) => {
                            let local = pe.local_name();
                            match local.as_ref() {
                                b"text-properties" => {
                                    attrs.merge(&parse_text_properties(pe));
                                }
                                b"paragraph-properties" => {
                                    attrs.merge(&parse_paragraph_properties(pe));
                                }
                                _ => {}
                            }
                        }
                        Ok(Event::End(ref ee)) if ee.local_name().as_ref() == b"style" => {
                            break;
                        }
                        Ok(Event::Eof) => break,
                        Err(e) => return Err(OdtError::Xml(e.to_string())),
                        _ => {}
                    }
                }

                if !name.is_empty() {
                    // Store family info for distinguishing paragraph vs text auto-styles
                    if family == "text" {
                        // Mark as text/character style (no paragraph-level props expected)
                    }
                    auto_styles.insert(name, attrs);
                }
            }
            Ok(Event::Empty(ref e)) if e.local_name().as_ref() == b"style" => {
                let name = get_attr(e, b"name").unwrap_or_default();
                let parent = get_attr(e, b"parent-style-name");

                let mut attrs = AttributeMap::new();
                if let Some(ref parent_name) = parent {
                    attrs.set(
                        s1_model::AttributeKey::StyleId,
                        s1_model::AttributeValue::String(parent_name.clone()),
                    );
                }
                if !name.is_empty() {
                    auto_styles.insert(name, attrs);
                }
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"automatic-styles" => {
                break;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(auto_styles)
}

/// Parse a `<style:style>` element (non-empty) into a `Style`.
fn parse_style_element(
    reader: &mut Reader<&[u8]>,
    e: &quick_xml::events::BytesStart<'_>,
) -> Result<Option<Style>, OdtError> {
    let name = match get_attr(e, b"name") {
        Some(n) => n,
        None => {
            skip_to_end(reader, b"style")?;
            return Ok(None);
        }
    };

    let display_name = get_attr(e, b"display-name").unwrap_or_else(|| name.clone());
    let family = get_attr(e, b"family").unwrap_or_default();
    let parent_name = get_attr(e, b"parent-style-name");

    let style_type = match family.as_str() {
        "paragraph" => StyleType::Paragraph,
        "text" => StyleType::Character,
        "table" => StyleType::Table,
        "list" => StyleType::List,
        _ => StyleType::Paragraph,
    };

    let mut style = Style::new(&name, &display_name, style_type);
    if let Some(parent) = parent_name {
        style = style.with_parent(parent);
    }

    let mut attrs = AttributeMap::new();

    // Parse child property elements
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref pe)) => {
                let local = pe.local_name();
                match local.as_ref() {
                    b"text-properties" => {
                        attrs.merge(&parse_text_properties(pe));
                        skip_to_end(reader, b"text-properties")?;
                    }
                    b"paragraph-properties" => {
                        let mut para_attrs = parse_paragraph_properties(pe);
                        parse_paragraph_properties_children(reader, &mut para_attrs);
                        attrs.merge(&para_attrs);
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref pe)) => {
                let local = pe.local_name();
                match local.as_ref() {
                    b"text-properties" => attrs.merge(&parse_text_properties(pe)),
                    b"paragraph-properties" => attrs.merge(&parse_paragraph_properties(pe)),
                    _ => {}
                }
            }
            Ok(Event::End(ref ee)) if ee.local_name().as_ref() == b"style" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }

    if !attrs.is_empty() {
        style = style.with_attributes(attrs);
    }

    Ok(Some(style))
}

/// Parse a self-closing `<style:style ... />` into a `Style` (no child properties).
fn parse_empty_style(e: &quick_xml::events::BytesStart<'_>) -> Option<Style> {
    let name = get_attr(e, b"name")?;
    let display_name = get_attr(e, b"display-name").unwrap_or_else(|| name.clone());
    let family = get_attr(e, b"family").unwrap_or_default();
    let parent_name = get_attr(e, b"parent-style-name");

    let style_type = match family.as_str() {
        "paragraph" => StyleType::Paragraph,
        "text" => StyleType::Character,
        "table" => StyleType::Table,
        "list" => StyleType::List,
        _ => StyleType::Paragraph,
    };

    let mut style = Style::new(&name, &display_name, style_type);
    if let Some(parent) = parent_name {
        style = style.with_parent(parent);
    }
    Some(style)
}

/// Parse a `<style:page-layout>` element to extract page dimensions and margins.
fn parse_page_layout(reader: &mut Reader<&[u8]>) -> Result<OdtPageLayout, OdtError> {
    let mut layout = OdtPageLayout::default();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"page-layout-properties" => {
                extract_page_layout_props(e, &mut layout);
                skip_to_end(reader, b"page-layout-properties")?;
            }
            Ok(Event::Empty(ref e)) if e.local_name().as_ref() == b"page-layout-properties" => {
                extract_page_layout_props(e, &mut layout);
            }
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"page-layout" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(layout)
}

/// Extract page layout properties from `<style:page-layout-properties>` attributes.
fn extract_page_layout_props(e: &quick_xml::events::BytesStart<'_>, layout: &mut OdtPageLayout) {
    if let Some(w) = get_attr(e, b"page-width").and_then(|s| parse_length(&s)) {
        layout.page_width = Some(w);
    }
    if let Some(h) = get_attr(e, b"page-height").and_then(|s| parse_length(&s)) {
        layout.page_height = Some(h);
    }
    layout.orientation = get_attr(e, b"print-orientation");
    if let Some(v) = get_attr(e, b"margin-top").and_then(|s| parse_length(&s)) {
        layout.margin_top = Some(v);
    }
    if let Some(v) = get_attr(e, b"margin-bottom").and_then(|s| parse_length(&s)) {
        layout.margin_bottom = Some(v);
    }
    if let Some(v) = get_attr(e, b"margin-left").and_then(|s| parse_length(&s)) {
        layout.margin_left = Some(v);
    }
    if let Some(v) = get_attr(e, b"margin-right").and_then(|s| parse_length(&s)) {
        layout.margin_right = Some(v);
    }
}

/// Parse a `<style:master-page>` element including header/footer content.
fn parse_master_page(
    reader: &mut Reader<&[u8]>,
    layout: OdtPageLayout,
) -> Result<OdtMasterPage, OdtError> {
    let mut mp = OdtMasterPage {
        layout,
        ..Default::default()
    };

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => match e.local_name().as_ref() {
                b"header" => {
                    mp.header = Some(parse_hf_content(reader, b"header")?);
                }
                b"footer" => {
                    mp.footer = Some(parse_hf_content(reader, b"footer")?);
                }
                b"header-first" => {
                    mp.first_header = Some(parse_hf_content(reader, b"header-first")?);
                }
                b"footer-first" => {
                    mp.first_footer = Some(parse_hf_content(reader, b"footer-first")?);
                }
                _ => {}
            },
            Ok(Event::End(ref e)) if e.local_name().as_ref() == b"master-page" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(mp)
}

/// Parse content inside a `<style:header>` or `<style:footer>` element.
///
/// Returns paragraph segments (text and field references).
fn parse_hf_content(reader: &mut Reader<&[u8]>, end_tag: &[u8]) -> Result<HfContent, OdtError> {
    let mut content = HfContent::default();
    let mut current_para: Vec<HfSegment> = Vec::new();
    let mut in_paragraph = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) if e.local_name().as_ref() == b"p" => {
                in_paragraph = true;
                current_para.clear();
            }
            Ok(Event::Text(ref t)) if in_paragraph => {
                if let Ok(text) = t.unescape() {
                    let s = text.to_string();
                    if !s.is_empty() {
                        current_para.push(HfSegment::Text(s));
                    }
                }
            }
            Ok(Event::Empty(ref e)) if in_paragraph => match e.local_name().as_ref() {
                b"page-number" => current_para.push(HfSegment::PageNumber),
                b"page-count" => current_para.push(HfSegment::PageCount),
                _ => {}
            },
            Ok(Event::End(ref e)) => match e.local_name().as_ref() {
                b"p" if in_paragraph => {
                    content.paragraphs.push(current_para.clone());
                    current_para.clear();
                    in_paragraph = false;
                }
                tag if tag == end_tag => break,
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(content)
}

/// Skip the reader past the end of an element.
fn skip_to_end(reader: &mut Reader<&[u8]>, tag: &[u8]) -> Result<(), OdtError> {
    let mut depth = 1u32;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) if e.local_name().as_ref() == tag => depth += 1,
            Ok(Event::End(e)) if e.local_name().as_ref() == tag => {
                depth -= 1;
                if depth == 0 {
                    return Ok(());
                }
            }
            Ok(Event::Eof) => return Ok(()),
            Err(e) => return Err(OdtError::Xml(e.to_string())),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::AttributeKey;

    #[test]
    fn parse_named_style_paragraph() {
        let xml = r#"<?xml version="1.0"?>
<office:document-styles xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
<office:styles>
  <style:style style:name="Heading1" style:display-name="Heading 1" style:family="paragraph">
    <style:text-properties fo:font-weight="bold" fo:font-size="24pt"/>
    <style:paragraph-properties fo:text-align="center"/>
  </style:style>
</office:styles>
</office:document-styles>"#;

        let mut doc = DocumentModel::new();
        parse_styles_xml(xml, &mut doc).unwrap();

        let style = doc.style_by_id("Heading1").unwrap();
        assert_eq!(style.name, "Heading 1");
        assert_eq!(style.style_type, StyleType::Paragraph);
        assert_eq!(style.attributes.get_bool(&AttributeKey::Bold), Some(true));
        assert_eq!(
            style.attributes.get_f64(&AttributeKey::FontSize),
            Some(24.0)
        );
    }

    #[test]
    fn parse_style_with_parent() {
        let xml = r#"<?xml version="1.0"?>
<office:document-styles xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
<office:styles>
  <style:style style:name="TextBody" style:family="paragraph"/>
  <style:style style:name="MyStyle" style:family="paragraph" style:parent-style-name="TextBody">
    <style:text-properties fo:font-style="italic"/>
  </style:style>
</office:styles>
</office:document-styles>"#;

        let mut doc = DocumentModel::new();
        parse_styles_xml(xml, &mut doc).unwrap();

        let style = doc.style_by_id("MyStyle").unwrap();
        assert_eq!(style.parent_id.as_deref(), Some("TextBody"));
        assert_eq!(style.attributes.get_bool(&AttributeKey::Italic), Some(true));
    }

    #[test]
    fn parse_auto_styles() {
        let xml = br#"<office:automatic-styles>
  <style:style style:name="P1" style:family="paragraph" style:parent-style-name="Standard">
    <style:paragraph-properties fo:text-align="center"/>
  </style:style>
  <style:style style:name="T1" style:family="text">
    <style:text-properties fo:font-weight="bold"/>
  </style:style>
</office:automatic-styles>"#;

        let mut reader = Reader::from_reader(xml.as_ref());
        // Advance past the opening tag
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) if e.local_name().as_ref() == b"automatic-styles" => break,
                Ok(Event::Eof) => panic!("unexpected EOF"),
                _ => {}
            }
        }

        let auto = parse_automatic_styles(&mut reader).unwrap();
        assert_eq!(auto.len(), 2);

        let p1 = auto.get("P1").unwrap();
        assert_eq!(p1.get_string(&AttributeKey::StyleId), Some("Standard"));

        let t1 = auto.get("T1").unwrap();
        assert_eq!(t1.get_bool(&AttributeKey::Bold), Some(true));
    }

    #[test]
    fn parse_page_layout_basic() {
        let xml = r#"<?xml version="1.0"?>
<office:document-styles xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0">
<office:automatic-styles>
  <style:page-layout style:name="pm1">
    <style:page-layout-properties fo:page-width="21cm" fo:page-height="29.7cm"
      style:print-orientation="portrait"
      fo:margin-top="2cm" fo:margin-bottom="2cm"
      fo:margin-left="2.5cm" fo:margin-right="2.5cm"/>
  </style:page-layout>
</office:automatic-styles>
<office:master-styles>
  <style:master-page style:name="Standard" style:page-layout-name="pm1"/>
</office:master-styles>
</office:document-styles>"#;

        let mut doc = DocumentModel::new();
        let mp = parse_styles_xml(xml, &mut doc).unwrap();
        assert!(mp.is_some());
        let mp = mp.unwrap();
        // 21cm ≈ 595.28 points
        assert!((mp.layout.page_width.unwrap() - 595.276).abs() < 1.0);
        // 29.7cm ≈ 841.89 points
        assert!((mp.layout.page_height.unwrap() - 841.89).abs() < 1.0);
        assert_eq!(mp.layout.orientation.as_deref(), Some("portrait"));
        // 2cm ≈ 56.69 points
        assert!((mp.layout.margin_top.unwrap() - 56.69).abs() < 1.0);
        // 2.5cm ≈ 70.87 points
        assert!((mp.layout.margin_left.unwrap() - 70.87).abs() < 1.0);
    }

    #[test]
    fn parse_master_page_with_header_footer() {
        let xml = r#"<?xml version="1.0"?>
<office:document-styles xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
<office:automatic-styles>
  <style:page-layout style:name="pm1">
    <style:page-layout-properties fo:page-width="8.5in" fo:page-height="11in"/>
  </style:page-layout>
</office:automatic-styles>
<office:master-styles>
  <style:master-page style:name="Standard" style:page-layout-name="pm1">
    <style:header>
      <text:p>My Header</text:p>
    </style:header>
    <style:footer>
      <text:p>Page <text:page-number/> of <text:page-count/></text:p>
    </style:footer>
  </style:master-page>
</office:master-styles>
</office:document-styles>"#;

        let mut doc = DocumentModel::new();
        let mp = parse_styles_xml(xml, &mut doc).unwrap().unwrap();

        // Header
        let header = mp.header.unwrap();
        assert_eq!(header.paragraphs.len(), 1);
        assert_eq!(header.paragraphs[0].len(), 1);
        assert!(matches!(&header.paragraphs[0][0], HfSegment::Text(t) if t == "My Header"));

        // Footer with fields
        let footer = mp.footer.unwrap();
        assert_eq!(footer.paragraphs.len(), 1);
        assert_eq!(footer.paragraphs[0].len(), 4);
        assert!(matches!(&footer.paragraphs[0][0], HfSegment::Text(t) if t == "Page "));
        assert!(matches!(footer.paragraphs[0][1], HfSegment::PageNumber));
        assert!(matches!(&footer.paragraphs[0][2], HfSegment::Text(t) if t == " of "));
        assert!(matches!(footer.paragraphs[0][3], HfSegment::PageCount));
    }

    #[test]
    fn parse_master_page_first_page_header() {
        let xml = r#"<?xml version="1.0"?>
<office:document-styles xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
<office:automatic-styles>
  <style:page-layout style:name="pm1">
    <style:page-layout-properties fo:page-width="8.5in" fo:page-height="11in"/>
  </style:page-layout>
</office:automatic-styles>
<office:master-styles>
  <style:master-page style:name="Standard" style:page-layout-name="pm1">
    <style:header>
      <text:p>Default Header</text:p>
    </style:header>
    <style:header-first>
      <text:p>Title Page Header</text:p>
    </style:header-first>
  </style:master-page>
</office:master-styles>
</office:document-styles>"#;

        let mut doc = DocumentModel::new();
        let mp = parse_styles_xml(xml, &mut doc).unwrap().unwrap();

        assert!(mp.header.is_some());
        assert!(mp.first_header.is_some());

        let first = mp.first_header.unwrap();
        assert_eq!(first.paragraphs.len(), 1);
        assert!(matches!(&first.paragraphs[0][0], HfSegment::Text(t) if t == "Title Page Header"));
    }

    #[test]
    fn parse_styles_no_master_page() {
        let xml = r#"<?xml version="1.0"?>
<office:document-styles xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0">
<office:styles>
  <style:style style:name="Default" style:family="paragraph"/>
</office:styles>
</office:document-styles>"#;

        let mut doc = DocumentModel::new();
        let mp = parse_styles_xml(xml, &mut doc).unwrap();
        assert!(mp.is_none());
        assert!(doc.style_by_id("Default").is_some());
    }

    #[test]
    fn parse_empty_style_element() {
        let xml = r#"<?xml version="1.0"?>
<office:document-styles xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0">
<office:styles>
  <style:style style:name="Default" style:family="paragraph"/>
</office:styles>
</office:document-styles>"#;

        let mut doc = DocumentModel::new();
        parse_styles_xml(xml, &mut doc).unwrap();

        let style = doc.style_by_id("Default").unwrap();
        assert_eq!(style.style_type, StyleType::Paragraph);
        assert!(style.attributes.is_empty());
    }
}
