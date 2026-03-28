//! Round-trip tests for all new attributes added in the fidelity improvement phases.
//!
//! Each test: build model with attributes → write DOCX → read back → verify attributes.

use s1_model::*;

/// Helper: write a DocumentModel to DOCX bytes and read it back.
fn roundtrip_docx(model: &DocumentModel) -> DocumentModel {
    let bytes = s1_format_docx::write(model).expect("write DOCX");
    s1_format_docx::read(&bytes).expect("read DOCX")
}

/// Helper: build a simple doc with one paragraph, apply attrs to the first run.
fn doc_with_run_attrs(attrs: AttributeMap) -> DocumentModel {
    let mut doc = DocumentModel::new();
    let body_id = doc.body_id().unwrap();

    let para_id = doc.next_id();
    let para = Node::new(para_id, NodeType::Paragraph);
    doc.insert_node(body_id, 0, para).unwrap();

    let run_id = doc.next_id();
    let mut run = Node::new(run_id, NodeType::Run);
    run.attributes = attrs;
    doc.insert_node(para_id, 0, run).unwrap();

    let text_id = doc.next_id();
    let text = Node::text(text_id, "Test");
    doc.insert_node(run_id, 0, text).unwrap();

    doc
}

/// Helper: build a simple doc with one paragraph, apply attrs to the paragraph.
fn doc_with_para_attrs(attrs: AttributeMap) -> DocumentModel {
    let mut doc = DocumentModel::new();
    let body_id = doc.body_id().unwrap();

    let para_id = doc.next_id();
    let mut para = Node::new(para_id, NodeType::Paragraph);
    para.attributes = attrs;
    doc.insert_node(body_id, 0, para).unwrap();

    let run_id = doc.next_id();
    let run = Node::new(run_id, NodeType::Run);
    doc.insert_node(para_id, 0, run).unwrap();

    let text_id = doc.next_id();
    let text = Node::text(text_id, "Test");
    doc.insert_node(run_id, 0, text).unwrap();

    doc
}

/// Helper: get first run's attributes from a model.
fn first_run_attrs(model: &DocumentModel) -> &AttributeMap {
    let body_id = model.body_id().unwrap();
    let body = model.node(body_id).unwrap();
    let para_id = body.children[0];
    let para = model.node(para_id).unwrap();
    // First child that is a Run
    for &child_id in &para.children {
        let child = model.node(child_id).unwrap();
        if child.node_type == NodeType::Run {
            return &child.attributes;
        }
    }
    panic!("No run found");
}

/// Helper: get first paragraph's attributes from a model.
fn first_para_attrs(model: &DocumentModel) -> &AttributeMap {
    let body_id = model.body_id().unwrap();
    let body = model.node(body_id).unwrap();
    let para_id = body.children[0];
    &model.node(para_id).unwrap().attributes
}

// ─── Run Property Round-trips ────────────────────────────────────────────

#[test]
fn roundtrip_caps() {
    let mut attrs = AttributeMap::new();
    attrs.set(AttributeKey::Caps, AttributeValue::Bool(true));
    let rt = roundtrip_docx(&doc_with_run_attrs(attrs));
    assert_eq!(first_run_attrs(&rt).get_bool(&AttributeKey::Caps), Some(true));
}

#[test]
fn roundtrip_small_caps() {
    let mut attrs = AttributeMap::new();
    attrs.set(AttributeKey::SmallCaps, AttributeValue::Bool(true));
    let rt = roundtrip_docx(&doc_with_run_attrs(attrs));
    assert_eq!(first_run_attrs(&rt).get_bool(&AttributeKey::SmallCaps), Some(true));
}

#[test]
fn roundtrip_hidden() {
    let mut attrs = AttributeMap::new();
    attrs.set(AttributeKey::Hidden, AttributeValue::Bool(true));
    let rt = roundtrip_docx(&doc_with_run_attrs(attrs));
    assert_eq!(first_run_attrs(&rt).get_bool(&AttributeKey::Hidden), Some(true));
}

#[test]
fn roundtrip_double_strikethrough() {
    let mut attrs = AttributeMap::new();
    attrs.set(AttributeKey::DoubleStrikethrough, AttributeValue::Bool(true));
    let rt = roundtrip_docx(&doc_with_run_attrs(attrs));
    assert_eq!(
        first_run_attrs(&rt).get_bool(&AttributeKey::DoubleStrikethrough),
        Some(true)
    );
}

#[test]
fn roundtrip_font_size_cs() {
    let mut attrs = AttributeMap::new();
    attrs.set(AttributeKey::FontSizeCS, AttributeValue::Float(14.0));
    let rt = roundtrip_docx(&doc_with_run_attrs(attrs));
    assert_eq!(first_run_attrs(&rt).get_f64(&AttributeKey::FontSizeCS), Some(14.0));
}

#[test]
fn roundtrip_bold_cs() {
    let mut attrs = AttributeMap::new();
    attrs.set(AttributeKey::BoldCS, AttributeValue::Bool(true));
    let rt = roundtrip_docx(&doc_with_run_attrs(attrs));
    assert_eq!(first_run_attrs(&rt).get_bool(&AttributeKey::BoldCS), Some(true));
}

#[test]
fn roundtrip_italic_cs() {
    let mut attrs = AttributeMap::new();
    attrs.set(AttributeKey::ItalicCS, AttributeValue::Bool(true));
    let rt = roundtrip_docx(&doc_with_run_attrs(attrs));
    assert_eq!(first_run_attrs(&rt).get_bool(&AttributeKey::ItalicCS), Some(true));
}

#[test]
fn roundtrip_font_family_east_asia() {
    let mut attrs = AttributeMap::new();
    attrs.set(
        AttributeKey::FontFamilyEastAsia,
        AttributeValue::String("MS Mincho".into()),
    );
    let rt = roundtrip_docx(&doc_with_run_attrs(attrs));
    assert_eq!(
        first_run_attrs(&rt).get_string(&AttributeKey::FontFamilyEastAsia),
        Some("MS Mincho")
    );
}

#[test]
fn roundtrip_font_family_cs() {
    let mut attrs = AttributeMap::new();
    attrs.set(
        AttributeKey::FontFamilyCS,
        AttributeValue::String("Arial".into()),
    );
    let rt = roundtrip_docx(&doc_with_run_attrs(attrs));
    assert_eq!(
        first_run_attrs(&rt).get_string(&AttributeKey::FontFamilyCS),
        Some("Arial")
    );
}

#[test]
fn roundtrip_baseline_shift() {
    let mut attrs = AttributeMap::new();
    attrs.set(AttributeKey::BaselineShift, AttributeValue::Float(6.0));
    let rt = roundtrip_docx(&doc_with_run_attrs(attrs));
    assert_eq!(
        first_run_attrs(&rt).get_f64(&AttributeKey::BaselineShift),
        Some(6.0)
    );
}

// ─── Paragraph Property Round-trips ──────────────────────────────────────

#[test]
fn roundtrip_widow_control_false() {
    let mut attrs = AttributeMap::new();
    attrs.set(AttributeKey::WidowControl, AttributeValue::Bool(false));
    let rt = roundtrip_docx(&doc_with_para_attrs(attrs));
    assert_eq!(
        first_para_attrs(&rt).get_bool(&AttributeKey::WidowControl),
        Some(false)
    );
}

#[test]
fn roundtrip_outline_level() {
    let mut attrs = AttributeMap::new();
    attrs.set(AttributeKey::OutlineLevel, AttributeValue::Int(2));
    let rt = roundtrip_docx(&doc_with_para_attrs(attrs));
    assert_eq!(
        first_para_attrs(&rt).get_i64(&AttributeKey::OutlineLevel),
        Some(2)
    );
}

#[test]
fn roundtrip_hanging_indent() {
    let mut attrs = AttributeMap::new();
    attrs.set(AttributeKey::IndentLeft, AttributeValue::Float(36.0));
    attrs.set(AttributeKey::IndentFirstLine, AttributeValue::Float(-18.0));
    let rt = roundtrip_docx(&doc_with_para_attrs(attrs));
    let pa = first_para_attrs(&rt);
    assert_eq!(pa.get_f64(&AttributeKey::IndentLeft), Some(36.0));
    assert_eq!(pa.get_f64(&AttributeKey::IndentFirstLine), Some(-18.0));
}

#[test]
fn roundtrip_writing_mode() {
    let mut attrs = AttributeMap::new();
    attrs.set(
        AttributeKey::ParagraphWritingMode,
        AttributeValue::WritingMode(WritingMode::TbRl),
    );
    let rt = roundtrip_docx(&doc_with_para_attrs(attrs));
    assert_eq!(
        first_para_attrs(&rt).get(&AttributeKey::ParagraphWritingMode),
        Some(&AttributeValue::WritingMode(WritingMode::TbRl))
    );
}

// ─── Table Property Round-trips ──────────────────────────────────────────

/// Helper: build a doc with a simple 1x1 table, apply attrs to the table node.
fn doc_with_table_attrs(
    tbl_attrs: AttributeMap,
    row_attrs: AttributeMap,
    cell_attrs: AttributeMap,
) -> DocumentModel {
    let mut doc = DocumentModel::new();
    let body_id = doc.body_id().unwrap();

    let tbl_id = doc.next_id();
    let mut tbl = Node::new(tbl_id, NodeType::Table);
    tbl.attributes = tbl_attrs;
    doc.insert_node(body_id, 0, tbl).unwrap();

    let row_id = doc.next_id();
    let mut row = Node::new(row_id, NodeType::TableRow);
    row.attributes = row_attrs;
    doc.insert_node(tbl_id, 0, row).unwrap();

    let cell_id = doc.next_id();
    let mut cell = Node::new(cell_id, NodeType::TableCell);
    cell.attributes = cell_attrs;
    doc.insert_node(row_id, 0, cell).unwrap();

    let para_id = doc.next_id();
    doc.insert_node(cell_id, 0, Node::new(para_id, NodeType::Paragraph))
        .unwrap();
    let run_id = doc.next_id();
    doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
        .unwrap();
    let text_id = doc.next_id();
    doc.insert_node(run_id, 0, Node::text(text_id, "Cell"))
        .unwrap();

    doc
}

fn first_table_attrs(model: &DocumentModel) -> &AttributeMap {
    let body = model.node(model.body_id().unwrap()).unwrap();
    &model.node(body.children[0]).unwrap().attributes
}

fn first_row_attrs(model: &DocumentModel) -> &AttributeMap {
    let body = model.node(model.body_id().unwrap()).unwrap();
    let tbl = model.node(body.children[0]).unwrap();
    &model.node(tbl.children[0]).unwrap().attributes
}

fn first_cell_attrs(model: &DocumentModel) -> &AttributeMap {
    let body = model.node(model.body_id().unwrap()).unwrap();
    let tbl = model.node(body.children[0]).unwrap();
    let row = model.node(tbl.children[0]).unwrap();
    &model.node(row.children[0]).unwrap().attributes
}

#[test]
fn roundtrip_table_layout_fixed() {
    let mut ta = AttributeMap::new();
    ta.set(
        AttributeKey::TableLayout,
        AttributeValue::TableLayoutMode(TableLayoutMode::Fixed),
    );
    let rt = roundtrip_docx(&doc_with_table_attrs(ta, AttributeMap::new(), AttributeMap::new()));
    assert_eq!(
        first_table_attrs(&rt).get_table_layout(&AttributeKey::TableLayout),
        Some(TableLayoutMode::Fixed)
    );
}

#[test]
fn roundtrip_table_indent() {
    let mut ta = AttributeMap::new();
    ta.set(AttributeKey::TableIndent, AttributeValue::Float(36.0));
    let rt = roundtrip_docx(&doc_with_table_attrs(ta, AttributeMap::new(), AttributeMap::new()));
    assert_eq!(
        first_table_attrs(&rt).get_f64(&AttributeKey::TableIndent),
        Some(36.0)
    );
}

#[test]
fn roundtrip_table_cell_margins() {
    let mut ta = AttributeMap::new();
    ta.set(
        AttributeKey::TableDefaultCellMargins,
        AttributeValue::Margins(Margins::new(5.0, 5.0, 10.0, 10.0)),
    );
    let rt = roundtrip_docx(&doc_with_table_attrs(ta, AttributeMap::new(), AttributeMap::new()));
    let m = first_table_attrs(&rt)
        .get_margins(&AttributeKey::TableDefaultCellMargins)
        .expect("margins should round-trip");
    assert!((m.top - 5.0).abs() < 0.5);
    assert!((m.left - 10.0).abs() < 0.5);
}

#[test]
fn roundtrip_row_height() {
    let mut ra = AttributeMap::new();
    ra.set(AttributeKey::RowHeight, AttributeValue::Float(24.0));
    ra.set(
        AttributeKey::RowHeightRule,
        AttributeValue::String("exact".into()),
    );
    let rt = roundtrip_docx(&doc_with_table_attrs(AttributeMap::new(), ra, AttributeMap::new()));
    assert_eq!(first_row_attrs(&rt).get_f64(&AttributeKey::RowHeight), Some(24.0));
    assert_eq!(
        first_row_attrs(&rt).get_string(&AttributeKey::RowHeightRule),
        Some("exact")
    );
}

#[test]
fn roundtrip_row_no_split() {
    let mut ra = AttributeMap::new();
    ra.set(AttributeKey::RowNoSplit, AttributeValue::Bool(true));
    let rt = roundtrip_docx(&doc_with_table_attrs(AttributeMap::new(), ra, AttributeMap::new()));
    assert_eq!(
        first_row_attrs(&rt).get_bool(&AttributeKey::RowNoSplit),
        Some(true)
    );
}

#[test]
fn roundtrip_cell_no_wrap() {
    let mut ca = AttributeMap::new();
    ca.set(AttributeKey::CellNoWrap, AttributeValue::Bool(true));
    let rt = roundtrip_docx(&doc_with_table_attrs(AttributeMap::new(), AttributeMap::new(), ca));
    assert_eq!(
        first_cell_attrs(&rt).get_bool(&AttributeKey::CellNoWrap),
        Some(true)
    );
}

#[test]
fn roundtrip_cell_text_direction() {
    let mut ca = AttributeMap::new();
    ca.set(
        AttributeKey::CellTextDirection,
        AttributeValue::String("btLr".into()),
    );
    let rt = roundtrip_docx(&doc_with_table_attrs(AttributeMap::new(), AttributeMap::new(), ca));
    assert_eq!(
        first_cell_attrs(&rt).get_string(&AttributeKey::CellTextDirection),
        Some("btLr")
    );
}

#[test]
fn roundtrip_cell_padding() {
    let mut ca = AttributeMap::new();
    ca.set(
        AttributeKey::CellPadding,
        AttributeValue::Margins(Margins::new(2.0, 2.0, 4.0, 4.0)),
    );
    let rt = roundtrip_docx(&doc_with_table_attrs(AttributeMap::new(), AttributeMap::new(), ca));
    let m = first_cell_attrs(&rt)
        .get_margins(&AttributeKey::CellPadding)
        .expect("cell padding should round-trip");
    assert!((m.top - 2.0).abs() < 0.5);
    assert!((m.left - 4.0).abs() < 0.5);
}

// ─── Section Property Round-trips ────────────────────────────────────────

#[test]
fn roundtrip_page_borders() {
    let mut doc = DocumentModel::new();
    let mut section = SectionProperties::default();
    section.page_borders = Some(Borders {
        top: Some(BorderSide {
            style: BorderStyle::Single,
            width: 1.0,
            color: Color::RED,
            spacing: 0.0,
        }),
        bottom: Some(BorderSide {
            style: BorderStyle::Double,
            width: 2.0,
            color: Color::BLACK,
            spacing: 0.0,
        }),
        left: None,
        right: None,
    });
    doc.sections_mut().push(section);

    let rt = roundtrip_docx(&doc);
    let sect = &rt.sections()[0];
    assert!(sect.page_borders.is_some());
    let pb = sect.page_borders.as_ref().unwrap();
    assert!(pb.top.is_some());
    assert_eq!(pb.top.as_ref().unwrap().style, BorderStyle::Single);
    assert!(pb.bottom.is_some());
    assert_eq!(pb.bottom.as_ref().unwrap().style, BorderStyle::Double);
}

#[test]
fn roundtrip_doc_grid() {
    let mut doc = DocumentModel::new();
    let mut section = SectionProperties::default();
    section.doc_grid_type = Some("lines".to_string());
    section.doc_grid_line_pitch = Some(18.0);
    doc.sections_mut().push(section);

    let rt = roundtrip_docx(&doc);
    let sect = &rt.sections()[0];
    assert_eq!(sect.doc_grid_type.as_deref(), Some("lines"));
    assert!((sect.doc_grid_line_pitch.unwrap() - 18.0).abs() < 0.5);
}

#[test]
fn roundtrip_line_numbering() {
    let mut doc = DocumentModel::new();
    let mut section = SectionProperties::default();
    section.line_numbering_start = Some(1);
    section.line_numbering_count_by = Some(5);
    section.line_numbering_restart = Some("newPage".to_string());
    doc.sections_mut().push(section);

    let rt = roundtrip_docx(&doc);
    let sect = &rt.sections()[0];
    assert_eq!(sect.line_numbering_start, Some(1));
    assert_eq!(sect.line_numbering_count_by, Some(5));
    assert_eq!(sect.line_numbering_restart.as_deref(), Some("newPage"));
}

// ─── Field Type Round-trips ──────────────────────────────────────────────

#[test]
fn roundtrip_field_types() {
    let mut doc = DocumentModel::new();
    let body_id = doc.body_id().unwrap();

    let para_id = doc.next_id();
    doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
        .unwrap();

    // PAGE field
    let field_id = doc.next_id();
    let mut field = Node::new(field_id, NodeType::Field);
    field.attributes.set(
        AttributeKey::FieldType,
        AttributeValue::FieldType(FieldType::PageNumber),
    );
    field
        .attributes
        .set(AttributeKey::FieldCode, AttributeValue::String("PAGE".into()));
    doc.insert_node(para_id, 0, field).unwrap();

    let rt = roundtrip_docx(&doc);
    let rt_body = rt.node(rt.body_id().unwrap()).unwrap();
    let rt_para = rt.node(rt_body.children[0]).unwrap();
    // Find the field node
    let field_node = rt_para.children.iter().find_map(|&cid| {
        let n = rt.node(cid)?;
        if n.node_type == NodeType::Field {
            Some(n)
        } else {
            None
        }
    });
    assert!(field_node.is_some());
    assert_eq!(
        field_node.unwrap().attributes.get_field_type(&AttributeKey::FieldType),
        Some(FieldType::PageNumber)
    );
}
