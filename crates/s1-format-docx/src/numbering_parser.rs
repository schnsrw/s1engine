//! Parse `word/numbering.xml` — list numbering definitions.

use quick_xml::events::Event;
use quick_xml::Reader;
use s1_model::{
    AbstractNumbering, Alignment, DocumentModel, LevelOverride, ListFormat, NumberingInstance,
    NumberingLevel,
};

use crate::error::DocxError;
use crate::xml_util::{get_attr, get_val, twips_to_points};

/// Parse `word/numbering.xml` and add numbering definitions to the document model.
pub fn parse_numbering_xml(xml: &str, doc: &mut DocumentModel) -> Result<(), DocxError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) if e.local_name().as_ref() == b"abstractNum" => {
                if let Some(id_str) = get_attr(&e, b"abstractNumId") {
                    if let Ok(id) = id_str.parse::<u32>() {
                        let abs = parse_abstract_num(&mut reader, id)?;
                        doc.numbering_mut().abstract_nums.push(abs);
                    }
                }
            }
            Ok(Event::Start(e)) if e.local_name().as_ref() == b"num" => {
                if let Some(id_str) = get_attr(&e, b"numId") {
                    if let Ok(num_id) = id_str.parse::<u32>() {
                        let inst = parse_num_instance(&mut reader, num_id)?;
                        doc.numbering_mut().instances.push(inst);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(())
}

/// Parse a single `<w:abstractNum>` element.
fn parse_abstract_num(
    reader: &mut Reader<&[u8]>,
    abstract_num_id: u32,
) -> Result<AbstractNumbering, DocxError> {
    let mut abs = AbstractNumbering {
        abstract_num_id,
        name: None,
        levels: Vec::new(),
    };

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) if e.local_name().as_ref() == b"lvl" => {
                let ilvl = get_attr(&e, b"ilvl")
                    .and_then(|v| v.parse::<u8>().ok())
                    .unwrap_or(0);
                let level = parse_level(reader, ilvl)?;
                abs.levels.push(level);
            }
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.local_name().as_ref() == b"name" => {
                abs.name = get_val(&e);
            }
            Ok(Event::End(e)) if e.local_name().as_ref() == b"abstractNum" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(abs)
}

/// Parse a single `<w:lvl>` element.
fn parse_level(reader: &mut Reader<&[u8]>, level: u8) -> Result<NumberingLevel, DocxError> {
    let mut lvl = NumberingLevel {
        level,
        num_format: ListFormat::Decimal,
        level_text: String::new(),
        start: 1,
        indent_left: None,
        indent_hanging: None,
        alignment: None,
        bullet_font: None,
    };

    loop {
        match reader.read_event() {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => match e.local_name().as_ref() {
                b"start" => {
                    if let Some(v) = get_val(&e) {
                        lvl.start = v.parse().unwrap_or(1);
                    }
                }
                b"numFmt" => {
                    if let Some(v) = get_val(&e) {
                        lvl.num_format = parse_num_format(&v);
                    }
                }
                b"lvlText" => {
                    lvl.level_text = get_val(&e).unwrap_or_default();
                }
                b"lvlJc" => {
                    if let Some(v) = get_val(&e) {
                        lvl.alignment = Some(match v.as_str() {
                            "center" => Alignment::Center,
                            "right" => Alignment::Right,
                            _ => Alignment::Left,
                        });
                    }
                }
                b"ind" => {
                    if let Some(left) = get_attr(&e, b"left") {
                        lvl.indent_left = twips_to_points(&left);
                    }
                    if let Some(hanging) = get_attr(&e, b"hanging") {
                        lvl.indent_hanging = twips_to_points(&hanging);
                    }
                }
                b"rFonts" => {
                    lvl.bullet_font = get_attr(&e, b"ascii");
                }
                _ => {}
            },
            Ok(Event::End(e)) if e.local_name().as_ref() == b"lvl" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(lvl)
}

/// Parse a `<w:num>` element.
fn parse_num_instance(
    reader: &mut Reader<&[u8]>,
    num_id: u32,
) -> Result<NumberingInstance, DocxError> {
    let mut inst = NumberingInstance {
        num_id,
        abstract_num_id: 0,
        level_overrides: Vec::new(),
    };

    loop {
        match reader.read_event() {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => match e.local_name().as_ref() {
                b"abstractNumId" => {
                    if let Some(v) = get_val(&e) {
                        inst.abstract_num_id = v.parse().unwrap_or(0);
                    }
                }
                b"lvlOverride" => {
                    let ilvl = get_attr(&e, b"ilvl")
                        .and_then(|v| v.parse::<u8>().ok())
                        .unwrap_or(0);
                    let ovr = parse_level_override(reader, ilvl)?;
                    inst.level_overrides.push(ovr);
                }
                _ => {}
            },
            Ok(Event::End(e)) if e.local_name().as_ref() == b"num" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(inst)
}

/// Parse a `<w:lvlOverride>` element.
fn parse_level_override(reader: &mut Reader<&[u8]>, level: u8) -> Result<LevelOverride, DocxError> {
    let mut ovr = LevelOverride {
        level,
        start_override: None,
        level_def: None,
    };

    loop {
        match reader.read_event() {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => match e.local_name().as_ref() {
                b"startOverride" => {
                    if let Some(v) = get_val(&e) {
                        ovr.start_override = v.parse().ok();
                    }
                }
                b"lvl" => {
                    let ilvl = get_attr(&e, b"ilvl")
                        .and_then(|v| v.parse::<u8>().ok())
                        .unwrap_or(level);
                    ovr.level_def = Some(parse_level(reader, ilvl)?);
                }
                _ => {}
            },
            Ok(Event::End(e)) if e.local_name().as_ref() == b"lvlOverride" => break,
            Ok(Event::Eof) => break,
            Err(e) => return Err(DocxError::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(ovr)
}

/// Map OOXML `w:numFmt` value to `ListFormat`.
fn parse_num_format(val: &str) -> ListFormat {
    match val {
        "bullet" => ListFormat::Bullet,
        "decimal" => ListFormat::Decimal,
        "decimalZero" => ListFormat::DecimalZero,
        "lowerLetter" => ListFormat::LowerAlpha,
        "upperLetter" => ListFormat::UpperAlpha,
        "lowerRoman" => ListFormat::LowerRoman,
        "upperRoman" => ListFormat::UpperRoman,
        "none" => ListFormat::None,
        // Lenient: treat unknown formats as Decimal rather than failing
        _ => ListFormat::Decimal,
    }
}

/// Map `ListFormat` back to OOXML `w:numFmt` value.
pub fn list_format_to_ooxml(fmt: ListFormat) -> &'static str {
    match fmt {
        ListFormat::Bullet => "bullet",
        ListFormat::Decimal => "decimal",
        ListFormat::DecimalZero => "decimalZero",
        ListFormat::LowerAlpha => "lowerLetter",
        ListFormat::UpperAlpha => "upperLetter",
        ListFormat::LowerRoman => "lowerRoman",
        ListFormat::UpperRoman => "upperRoman",
        ListFormat::None => "none",
        _ => "decimal",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(xml: &str) -> DocumentModel {
        let mut doc = DocumentModel::new();
        parse_numbering_xml(xml, &mut doc).unwrap();
        doc
    }

    #[test]
    fn parse_bullet_abstract_num() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0">
      <w:start w:val="1"/>
      <w:numFmt w:val="bullet"/>
      <w:lvlText w:val="&#xF0B7;"/>
      <w:lvlJc w:val="left"/>
      <w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr>
      <w:rPr><w:rFonts w:ascii="Symbol"/></w:rPr>
    </w:lvl>
  </w:abstractNum>
</w:numbering>"#;
        let doc = parse(xml);
        let abs = &doc.numbering().abstract_nums;
        assert_eq!(abs.len(), 1);
        assert_eq!(abs[0].abstract_num_id, 0);
        assert_eq!(abs[0].levels.len(), 1);

        let lvl = &abs[0].levels[0];
        assert_eq!(lvl.level, 0);
        assert_eq!(lvl.num_format, ListFormat::Bullet);
        // quick_xml's get_attr returns the raw attribute value;
        // XML entity &#xF0B7; is decoded by quick_xml attribute parsing
        assert!(!lvl.level_text.is_empty());
        assert_eq!(lvl.start, 1);
        assert!((lvl.indent_left.unwrap() - 36.0).abs() < 0.01); // 720 twips
        assert!((lvl.indent_hanging.unwrap() - 18.0).abs() < 0.01); // 360 twips
        assert_eq!(lvl.bullet_font.as_deref(), Some("Symbol"));
    }

    #[test]
    fn parse_decimal_multi_level() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="1">
    <w:lvl w:ilvl="0">
      <w:start w:val="1"/>
      <w:numFmt w:val="decimal"/>
      <w:lvlText w:val="%1."/>
    </w:lvl>
    <w:lvl w:ilvl="1">
      <w:start w:val="1"/>
      <w:numFmt w:val="lowerLetter"/>
      <w:lvlText w:val="%2)"/>
    </w:lvl>
    <w:lvl w:ilvl="2">
      <w:start w:val="1"/>
      <w:numFmt w:val="lowerRoman"/>
      <w:lvlText w:val="%3."/>
    </w:lvl>
  </w:abstractNum>
</w:numbering>"#;
        let doc = parse(xml);
        let abs = &doc.numbering().abstract_nums[0];
        assert_eq!(abs.levels.len(), 3);
        assert_eq!(abs.levels[0].num_format, ListFormat::Decimal);
        assert_eq!(abs.levels[1].num_format, ListFormat::LowerAlpha);
        assert_eq!(abs.levels[1].level_text, "%2)");
        assert_eq!(abs.levels[2].num_format, ListFormat::LowerRoman);
    }

    #[test]
    fn parse_numbering_instance() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0">
      <w:numFmt w:val="decimal"/>
      <w:lvlText w:val="%1."/>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="1">
    <w:abstractNumId w:val="0"/>
  </w:num>
  <w:num w:numId="2">
    <w:abstractNumId w:val="0"/>
  </w:num>
</w:numbering>"#;
        let doc = parse(xml);
        let instances = &doc.numbering().instances;
        assert_eq!(instances.len(), 2);
        assert_eq!(instances[0].num_id, 1);
        assert_eq!(instances[0].abstract_num_id, 0);
        assert_eq!(instances[1].num_id, 2);
    }

    #[test]
    fn parse_level_override_start() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0">
      <w:start w:val="1"/>
      <w:numFmt w:val="decimal"/>
      <w:lvlText w:val="%1."/>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="1">
    <w:abstractNumId w:val="0"/>
    <w:lvlOverride w:ilvl="0">
      <w:startOverride w:val="5"/>
    </w:lvlOverride>
  </w:num>
</w:numbering>"#;
        let doc = parse(xml);
        let inst = &doc.numbering().instances[0];
        assert_eq!(inst.level_overrides.len(), 1);
        assert_eq!(inst.level_overrides[0].level, 0);
        assert_eq!(inst.level_overrides[0].start_override, Some(5));

        // Verify resolve_start uses override
        assert_eq!(doc.numbering().resolve_start(1, 0), Some(5));
        // Verify resolve_format still falls back to abstract
        assert_eq!(
            doc.numbering().resolve_format(1, 0),
            Some(ListFormat::Decimal)
        );
    }

    #[test]
    fn parse_empty_numbering() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
</w:numbering>"#;
        let doc = parse(xml);
        assert!(doc.numbering().is_empty());
    }

    #[test]
    fn parse_unknown_num_format() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0">
      <w:numFmt w:val="chicago"/>
      <w:lvlText w:val="%1."/>
    </w:lvl>
  </w:abstractNum>
</w:numbering>"#;
        let doc = parse(xml);
        // Unknown format defaults to Decimal (lenient)
        assert_eq!(
            doc.numbering().abstract_nums[0].levels[0].num_format,
            ListFormat::Decimal
        );
    }
}
