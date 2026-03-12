//! ODT writer — serialize a `DocumentModel` into an ODT ZIP archive.

use std::io::{Cursor, Write as IoWrite};

use s1_model::DocumentModel;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::content_writer::write_content_xml;
use crate::error::OdtError;
use crate::manifest_writer::write_manifest_xml;
use crate::metadata_writer::write_meta_xml;
use crate::style_writer::write_styles_xml;

/// Write a `DocumentModel` as ODT bytes.
///
/// # Errors
///
/// Returns `OdtError` if ZIP writing fails.
pub fn write(doc: &DocumentModel) -> Result<Vec<u8>, OdtError> {
    let mut buf = Vec::new();
    let cursor = Cursor::new(&mut buf);
    let mut zip = ZipWriter::new(cursor);

    let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let deflated =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // 1. mimetype — MUST be first entry, stored (uncompressed), no extra fields
    zip.start_file("mimetype", stored)?;
    zip.write_all(b"application/vnd.oasis.opendocument.text")?;

    // 2. Generate content.xml + collect image entries
    let (content_xml, image_entries) = write_content_xml(doc);

    // 3. Generate styles.xml (optional)
    let styles_xml = write_styles_xml(doc);

    // 4. Generate meta.xml (optional)
    let meta_xml = write_meta_xml(doc);

    // 5. Write content.xml
    zip.start_file("content.xml", deflated)?;
    zip.write_all(content_xml.as_bytes())?;

    // 6. Write styles.xml
    if let Some(ref styles) = styles_xml {
        zip.start_file("styles.xml", deflated)?;
        zip.write_all(styles.as_bytes())?;
    }

    // 7. Write meta.xml
    if let Some(ref meta) = meta_xml {
        zip.start_file("meta.xml", deflated)?;
        zip.write_all(meta.as_bytes())?;
    }

    // 8. Write images to Pictures/
    for entry in &image_entries {
        if let Some(media) = doc.media().get(entry.media_id) {
            zip.start_file(&entry.href, deflated)?;
            zip.write_all(&media.data)?;
        }
    }

    // 9. Write META-INF/manifest.xml
    let image_paths: Vec<&str> = image_entries.iter().map(|e| e.href.as_str()).collect();
    let manifest = write_manifest_xml(&image_paths);
    zip.start_file("META-INF/manifest.xml", deflated)?;
    zip.write_all(manifest.as_bytes())?;

    zip.finish()?;

    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::{AttributeMap, Node, NodeType, Style, StyleType};

    #[test]
    fn write_minimal_odt() {
        let doc = DocumentModel::new();
        let bytes = write(&doc).unwrap();

        // Verify it's a valid ZIP
        let cursor = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();

        // Check mimetype
        let mut mimetype = String::new();
        archive
            .by_name("mimetype")
            .unwrap()
            .read_to_string(&mut mimetype)
            .unwrap();
        assert_eq!(mimetype, "application/vnd.oasis.opendocument.text");

        // Check content.xml exists
        assert!(archive.by_name("content.xml").is_ok());
        assert!(archive.by_name("META-INF/manifest.xml").is_ok());
    }

    #[test]
    fn write_with_content() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let para = Node::new(para_id, NodeType::Paragraph);
        doc.insert_node(body_id, 0, para).unwrap();

        let run_id = doc.next_id();
        let run = Node::new(run_id, NodeType::Run);
        doc.insert_node(para_id, 0, run).unwrap();

        let text_id = doc.next_id();
        let text = Node::text(text_id, "Hello ODT");
        doc.insert_node(run_id, 0, text).unwrap();

        let bytes = write(&doc).unwrap();

        // Verify content.xml contains our text
        let cursor = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        let mut content = String::new();
        archive
            .by_name("content.xml")
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert!(content.contains("Hello ODT"));
    }

    #[test]
    fn write_with_styles() {
        let mut doc = DocumentModel::new();
        let style = Style::new("Heading1", "Heading 1", StyleType::Paragraph)
            .with_attributes(AttributeMap::new().bold(true));
        doc.set_style(style);

        let bytes = write(&doc).unwrap();
        let cursor = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();

        // styles.xml should exist
        assert!(archive.by_name("styles.xml").is_ok());

        let mut styles = String::new();
        archive
            .by_name("styles.xml")
            .unwrap()
            .read_to_string(&mut styles)
            .unwrap();
        assert!(styles.contains("Heading1"));
    }

    #[test]
    fn write_with_metadata() {
        let mut doc = DocumentModel::new();
        doc.metadata_mut().title = Some("Test Doc".to_string());
        doc.metadata_mut().creator = Some("Author".to_string());

        let bytes = write(&doc).unwrap();
        let cursor = Cursor::new(&bytes);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();

        assert!(archive.by_name("meta.xml").is_ok());

        let mut meta = String::new();
        archive
            .by_name("meta.xml")
            .unwrap()
            .read_to_string(&mut meta)
            .unwrap();
        assert!(meta.contains("Test Doc"));
        assert!(meta.contains("Author"));
    }

    #[test]
    fn roundtrip_basic() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Round trip test"))
            .unwrap();

        let bytes = write(&doc).unwrap();
        let doc2 = crate::reader::read(&bytes).unwrap();

        let body_id2 = doc2.body_id().unwrap();
        let body2 = doc2.node(body_id2).unwrap();
        assert_eq!(body2.children.len(), 1);

        // Verify text content is preserved
        let plain = doc2.to_plain_text();
        assert!(plain.contains("Round trip test"));
    }

    #[test]
    fn roundtrip_metadata() {
        let mut doc = DocumentModel::new();
        doc.metadata_mut().title = Some("My Title".to_string());
        doc.metadata_mut().creator = Some("Author Name".to_string());

        let bytes = write(&doc).unwrap();
        let doc2 = crate::reader::read(&bytes).unwrap();

        assert_eq!(doc2.metadata().title.as_deref(), Some("My Title"));
        assert_eq!(doc2.metadata().creator.as_deref(), Some("Author Name"));
    }

    #[test]
    fn roundtrip_styles() {
        let mut doc = DocumentModel::new();
        let style = Style::new("MyStyle", "My Style", StyleType::Paragraph)
            .with_attributes(AttributeMap::new().bold(true).font_size(16.0));
        doc.set_style(style);

        let bytes = write(&doc).unwrap();
        let doc2 = crate::reader::read(&bytes).unwrap();

        let s = doc2.style_by_id("MyStyle").unwrap();
        assert_eq!(s.name, "My Style");
        assert_eq!(
            s.attributes.get_bool(&s1_model::AttributeKey::Bold),
            Some(true)
        );
        assert_eq!(
            s.attributes.get_f64(&s1_model::AttributeKey::FontSize),
            Some(16.0)
        );
    }

    use std::io::Read as _;

    #[test]
    fn roundtrip_page_layout() {
        use s1_model::{PageOrientation, SectionProperties};

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();
        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Hello"))
            .unwrap();

        let mut sect = SectionProperties::default();
        sect.page_width = 595.276; // A4
        sect.page_height = 841.89;
        sect.orientation = PageOrientation::Portrait;
        sect.margin_top = 72.0;
        sect.margin_bottom = 72.0;
        sect.margin_left = 90.0;
        sect.margin_right = 90.0;
        doc.sections_mut().push(sect);

        let bytes = write(&doc).unwrap();
        let doc2 = crate::reader::read(&bytes).unwrap();

        assert_eq!(doc2.sections().len(), 1);
        let s = &doc2.sections()[0];
        assert!((s.page_width - 595.276).abs() < 1.0);
        assert!((s.page_height - 841.89).abs() < 1.0);
        assert!((s.margin_top - 72.0).abs() < 1.0);
        assert!((s.margin_left - 90.0).abs() < 1.0);
    }

    #[test]
    fn roundtrip_header_footer() {
        use s1_model::{HeaderFooterRef, HeaderFooterType, Node, SectionProperties};

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();
        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Body"))
            .unwrap();

        // Create header
        let hdr_id = doc.next_id();
        let root_id = doc.root_id();
        let idx = doc.node(root_id).map(|n| n.children.len()).unwrap_or(0);
        doc.insert_node(root_id, idx, Node::new(hdr_id, NodeType::Header))
            .unwrap();

        let hp_id = doc.next_id();
        doc.insert_node(hdr_id, 0, Node::new(hp_id, NodeType::Paragraph))
            .unwrap();
        let hr_id = doc.next_id();
        doc.insert_node(hp_id, 0, Node::new(hr_id, NodeType::Run))
            .unwrap();
        let ht_id = doc.next_id();
        doc.insert_node(hr_id, 0, Node::text(ht_id, "Header Text"))
            .unwrap();

        // Create footer
        let ftr_id = doc.next_id();
        let idx = doc.node(root_id).map(|n| n.children.len()).unwrap_or(0);
        doc.insert_node(root_id, idx, Node::new(ftr_id, NodeType::Footer))
            .unwrap();

        let fp_id = doc.next_id();
        doc.insert_node(ftr_id, 0, Node::new(fp_id, NodeType::Paragraph))
            .unwrap();
        let fr_id = doc.next_id();
        doc.insert_node(fp_id, 0, Node::new(fr_id, NodeType::Run))
            .unwrap();
        let ft_id = doc.next_id();
        doc.insert_node(fr_id, 0, Node::text(ft_id, "Footer Text"))
            .unwrap();

        // Set up section
        let mut sect = SectionProperties::default();
        sect.headers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: hdr_id,
        });
        sect.footers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: ftr_id,
        });
        doc.sections_mut().push(sect);

        let bytes = write(&doc).unwrap();
        let doc2 = crate::reader::read(&bytes).unwrap();

        assert_eq!(doc2.sections().len(), 1);
        let s = &doc2.sections()[0];
        assert!(s.has_headers());
        assert!(s.has_footers());

        // Verify header text
        let hdr_ref = s.header(HeaderFooterType::Default).unwrap();
        let hdr = doc2.node(hdr_ref.node_id).unwrap();
        assert_eq!(hdr.node_type, NodeType::Header);

        // Walk to text
        let para = doc2.node(hdr.children[0]).unwrap();
        let run = doc2.node(para.children[0]).unwrap();
        let text = doc2.node(run.children[0]).unwrap();
        assert_eq!(text.text_content.as_deref(), Some("Header Text"));

        // Verify footer text
        let ftr_ref = s.footer(HeaderFooterType::Default).unwrap();
        let ftr = doc2.node(ftr_ref.node_id).unwrap();
        let para = doc2.node(ftr.children[0]).unwrap();
        let run = doc2.node(para.children[0]).unwrap();
        let text = doc2.node(run.children[0]).unwrap();
        assert_eq!(text.text_content.as_deref(), Some("Footer Text"));
    }

    #[test]
    fn roundtrip_first_page_header() {
        use s1_model::{HeaderFooterRef, HeaderFooterType, Node, SectionProperties};

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();
        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Content"))
            .unwrap();

        // Default header
        let hdr_id = doc.next_id();
        let root_id = doc.root_id();
        let idx = doc.node(root_id).map(|n| n.children.len()).unwrap_or(0);
        doc.insert_node(root_id, idx, Node::new(hdr_id, NodeType::Header))
            .unwrap();
        let hp_id = doc.next_id();
        doc.insert_node(hdr_id, 0, Node::new(hp_id, NodeType::Paragraph))
            .unwrap();
        let hr_id = doc.next_id();
        doc.insert_node(hp_id, 0, Node::new(hr_id, NodeType::Run))
            .unwrap();
        let ht_id = doc.next_id();
        doc.insert_node(hr_id, 0, Node::text(ht_id, "Default Hdr"))
            .unwrap();

        // First-page header
        let first_id = doc.next_id();
        let idx = doc.node(root_id).map(|n| n.children.len()).unwrap_or(0);
        doc.insert_node(root_id, idx, Node::new(first_id, NodeType::Header))
            .unwrap();
        let fp_id = doc.next_id();
        doc.insert_node(first_id, 0, Node::new(fp_id, NodeType::Paragraph))
            .unwrap();
        let fr_id = doc.next_id();
        doc.insert_node(fp_id, 0, Node::new(fr_id, NodeType::Run))
            .unwrap();
        let ft_id = doc.next_id();
        doc.insert_node(fr_id, 0, Node::text(ft_id, "First Page"))
            .unwrap();

        let mut sect = SectionProperties::default();
        sect.title_page = true;
        sect.headers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: hdr_id,
        });
        sect.headers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::First,
            node_id: first_id,
        });
        doc.sections_mut().push(sect);

        let bytes = write(&doc).unwrap();
        let doc2 = crate::reader::read(&bytes).unwrap();

        let s = &doc2.sections()[0];
        assert!(s.title_page);
        assert_eq!(s.headers.len(), 2);

        // Verify first-page header
        let first_ref = s.header(HeaderFooterType::First).unwrap();
        let first = doc2.node(first_ref.node_id).unwrap();
        let para = doc2.node(first.children[0]).unwrap();
        let run = doc2.node(para.children[0]).unwrap();
        let text = doc2.node(run.children[0]).unwrap();
        assert_eq!(text.text_content.as_deref(), Some("First Page"));
    }

    #[test]
    fn roundtrip_footer_with_page_number() {
        use s1_model::{
            AttributeKey, AttributeValue, FieldType, HeaderFooterRef, HeaderFooterType, Node,
            SectionProperties,
        };

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();
        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Body"))
            .unwrap();

        // Footer: "Page {PAGE} of {NUMPAGES}"
        let ftr_id = doc.next_id();
        let root_id = doc.root_id();
        let idx = doc.node(root_id).map(|n| n.children.len()).unwrap_or(0);
        doc.insert_node(root_id, idx, Node::new(ftr_id, NodeType::Footer))
            .unwrap();

        let fp_id = doc.next_id();
        doc.insert_node(ftr_id, 0, Node::new(fp_id, NodeType::Paragraph))
            .unwrap();

        // "Page "
        let r1 = doc.next_id();
        doc.insert_node(fp_id, 0, Node::new(r1, NodeType::Run))
            .unwrap();
        let t1 = doc.next_id();
        doc.insert_node(r1, 0, Node::text(t1, "Page ")).unwrap();

        // PAGE field
        let f1 = doc.next_id();
        let mut field1 = Node::new(f1, NodeType::Field);
        field1.attributes.set(
            AttributeKey::FieldType,
            AttributeValue::FieldType(FieldType::PageNumber),
        );
        doc.insert_node(fp_id, 1, field1).unwrap();

        // " of "
        let r2 = doc.next_id();
        doc.insert_node(fp_id, 2, Node::new(r2, NodeType::Run))
            .unwrap();
        let t2 = doc.next_id();
        doc.insert_node(r2, 0, Node::text(t2, " of ")).unwrap();

        // NUMPAGES field
        let f2 = doc.next_id();
        let mut field2 = Node::new(f2, NodeType::Field);
        field2.attributes.set(
            AttributeKey::FieldType,
            AttributeValue::FieldType(FieldType::PageCount),
        );
        doc.insert_node(fp_id, 3, field2).unwrap();

        let mut sect = SectionProperties::default();
        sect.footers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: ftr_id,
        });
        doc.sections_mut().push(sect);

        let bytes = write(&doc).unwrap();
        let doc2 = crate::reader::read(&bytes).unwrap();

        let s = &doc2.sections()[0];
        let ftr_ref = s.footer(HeaderFooterType::Default).unwrap();
        let ftr = doc2.node(ftr_ref.node_id).unwrap();
        let para = doc2.node(ftr.children[0]).unwrap();

        // Should have: Run("Page "), Field(PageNumber), Run(" of "), Field(PageCount)
        assert_eq!(para.children.len(), 4);

        let child0 = doc2.node(para.children[0]).unwrap();
        assert_eq!(child0.node_type, NodeType::Run);

        let child1 = doc2.node(para.children[1]).unwrap();
        assert_eq!(child1.node_type, NodeType::Field);
        assert_eq!(
            child1.attributes.get(&AttributeKey::FieldType),
            Some(&AttributeValue::FieldType(FieldType::PageNumber))
        );

        let child3 = doc2.node(para.children[3]).unwrap();
        assert_eq!(child3.node_type, NodeType::Field);
        assert_eq!(
            child3.attributes.get(&AttributeKey::FieldType),
            Some(&AttributeValue::FieldType(FieldType::PageCount))
        );
    }
}
