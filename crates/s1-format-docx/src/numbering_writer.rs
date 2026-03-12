//! Generate `word/numbering.xml` from numbering definitions.

use s1_model::DocumentModel;

use crate::numbering_parser::list_format_to_ooxml;

/// Generate `word/numbering.xml` content, or `None` if there are no definitions.
pub fn write_numbering_xml(doc: &DocumentModel) -> Option<String> {
    let numbering = doc.numbering();
    if numbering.is_empty() {
        return None;
    }

    let mut xml = String::new();
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#);
    xml.push('\n');
    xml.push_str(
        r#"<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#,
    );

    // Abstract numbering definitions
    for abs in &numbering.abstract_nums {
        xml.push_str(&format!(
            r#"<w:abstractNum w:abstractNumId="{}">"#,
            abs.abstract_num_id
        ));

        if let Some(ref name) = abs.name {
            xml.push_str(&format!(r#"<w:name w:val="{name}"/>"#));
        }

        for lvl in &abs.levels {
            write_level(lvl, &mut xml);
        }

        xml.push_str("</w:abstractNum>");
    }

    // Numbering instances
    for inst in &numbering.instances {
        xml.push_str(&format!(r#"<w:num w:numId="{}">"#, inst.num_id));
        xml.push_str(&format!(
            r#"<w:abstractNumId w:val="{}"/>"#,
            inst.abstract_num_id
        ));

        for ovr in &inst.level_overrides {
            xml.push_str(&format!(r#"<w:lvlOverride w:ilvl="{}">"#, ovr.level));

            if let Some(start) = ovr.start_override {
                xml.push_str(&format!(r#"<w:startOverride w:val="{start}"/>"#));
            }

            if let Some(ref lvl_def) = ovr.level_def {
                write_level(lvl_def, &mut xml);
            }

            xml.push_str("</w:lvlOverride>");
        }

        xml.push_str("</w:num>");
    }

    xml.push_str("</w:numbering>");
    Some(xml)
}

/// Write a single `<w:lvl>` element.
fn write_level(lvl: &s1_model::NumberingLevel, xml: &mut String) {
    xml.push_str(&format!(r#"<w:lvl w:ilvl="{}">"#, lvl.level));
    xml.push_str(&format!(r#"<w:start w:val="{}"/>"#, lvl.start));
    xml.push_str(&format!(
        r#"<w:numFmt w:val="{}"/>"#,
        list_format_to_ooxml(lvl.num_format)
    ));
    xml.push_str(&format!(r#"<w:lvlText w:val="{}"/>"#, lvl.level_text));

    if let Some(ref alignment) = lvl.alignment {
        let val = match alignment {
            s1_model::Alignment::Left => "left",
            s1_model::Alignment::Center => "center",
            s1_model::Alignment::Right => "right",
            s1_model::Alignment::Justify => "left",
            _ => "left",
        };
        xml.push_str(&format!(r#"<w:lvlJc w:val="{val}"/>"#));
    }

    // Indentation
    if lvl.indent_left.is_some() || lvl.indent_hanging.is_some() {
        xml.push_str("<w:pPr>");
        let mut ind = String::new();
        if let Some(left) = lvl.indent_left {
            ind.push_str(&format!(r#" w:left="{}""#, points_to_twips(left)));
        }
        if let Some(hanging) = lvl.indent_hanging {
            ind.push_str(&format!(r#" w:hanging="{}""#, points_to_twips(hanging)));
        }
        xml.push_str(&format!("<w:ind{ind}/>"));
        xml.push_str("</w:pPr>");
    }

    // Bullet font
    if let Some(ref font) = lvl.bullet_font {
        xml.push_str("<w:rPr>");
        xml.push_str(&format!(r#"<w:rFonts w:ascii="{font}"/>"#));
        xml.push_str("</w:rPr>");
    }

    xml.push_str("</w:lvl>");
}

fn points_to_twips(pts: f64) -> i64 {
    (pts * 20.0) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::{AbstractNumbering, ListFormat, NumberingInstance, NumberingLevel};

    #[test]
    fn write_none_when_empty() {
        let doc = DocumentModel::new();
        assert!(write_numbering_xml(&doc).is_none());
    }

    #[test]
    fn write_bullet_numbering() {
        let mut doc = DocumentModel::new();
        doc.numbering_mut().abstract_nums.push(AbstractNumbering {
            abstract_num_id: 0,
            name: None,
            levels: vec![NumberingLevel {
                level: 0,
                num_format: ListFormat::Bullet,
                level_text: "\u{F0B7}".into(),
                start: 1,
                indent_left: Some(36.0),
                indent_hanging: Some(18.0),
                alignment: Some(s1_model::Alignment::Left),
                bullet_font: Some("Symbol".into()),
            }],
        });
        doc.numbering_mut().instances.push(NumberingInstance {
            num_id: 1,
            abstract_num_id: 0,
            level_overrides: vec![],
        });

        let xml = write_numbering_xml(&doc).unwrap();
        assert!(xml.contains(r#"w:abstractNumId="0""#));
        assert!(xml.contains(r#"w:numFmt w:val="bullet""#));
        assert!(xml.contains(r#"w:rFonts w:ascii="Symbol""#));
        assert!(xml.contains(r#"w:numId="1""#));
        assert!(xml.contains(r#"w:left="720""#)); // 36pt = 720 twips
        assert!(xml.contains(r#"w:hanging="360""#)); // 18pt = 360 twips
    }

    #[test]
    fn write_decimal_numbering() {
        let mut doc = DocumentModel::new();
        doc.numbering_mut().abstract_nums.push(AbstractNumbering {
            abstract_num_id: 0,
            name: Some("OrderedList".into()),
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
        });
        doc.numbering_mut().instances.push(NumberingInstance {
            num_id: 1,
            abstract_num_id: 0,
            level_overrides: vec![],
        });

        let xml = write_numbering_xml(&doc).unwrap();
        assert!(xml.contains(r#"w:numFmt w:val="decimal""#));
        assert!(xml.contains(r#"w:lvlText w:val="%1.""#));
        assert!(xml.contains(r#"w:name w:val="OrderedList""#));
    }

    #[test]
    fn write_with_level_override() {
        let mut doc = DocumentModel::new();
        doc.numbering_mut().abstract_nums.push(AbstractNumbering {
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
        });
        doc.numbering_mut().instances.push(NumberingInstance {
            num_id: 1,
            abstract_num_id: 0,
            level_overrides: vec![s1_model::LevelOverride {
                level: 0,
                start_override: Some(5),
                level_def: None,
            }],
        });

        let xml = write_numbering_xml(&doc).unwrap();
        assert!(xml.contains(r#"<w:lvlOverride w:ilvl="0">"#));
        assert!(xml.contains(r#"<w:startOverride w:val="5"/>"#));
    }
}
