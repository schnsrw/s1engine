//! Resolve effective styles for document nodes.
//!
//! Computes the final attribute values for each node by walking up the style
//! chain: direct attributes → character style → paragraph style → defaults.

use s1_model::{
    Alignment, AttributeKey, AttributeValue, Color, DocumentModel, LineSpacing, Node, NodeId,
};

/// Default font family used when none is specified.
pub const DEFAULT_FONT_FAMILY: &str = "Times New Roman";
/// Default font size in points.
pub const DEFAULT_FONT_SIZE: f64 = 12.0;
/// Default line spacing.
pub const DEFAULT_LINE_SPACING: f64 = 1.15;
/// Default text color (black).
pub const DEFAULT_COLOR: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

/// Resolved paragraph style attributes.
#[derive(Debug, Clone)]
pub struct ResolvedParagraphStyle {
    /// Text alignment.
    pub alignment: Alignment,
    /// Space before paragraph in points.
    pub space_before: f64,
    /// Space after paragraph in points.
    pub space_after: f64,
    /// Line spacing.
    pub line_spacing: LineSpacing,
    /// Left indent in points.
    pub indent_left: f64,
    /// Right indent in points.
    pub indent_right: f64,
    /// First line indent in points.
    pub indent_first_line: f64,
    /// Keep with next paragraph.
    pub keep_with_next: bool,
    /// Keep all lines together.
    pub keep_lines: bool,
    /// Page break before.
    pub page_break_before: bool,
}

impl Default for ResolvedParagraphStyle {
    fn default() -> Self {
        Self {
            alignment: Alignment::Left,
            space_before: 0.0,
            space_after: 8.0,
            line_spacing: LineSpacing::Multiple(DEFAULT_LINE_SPACING),
            indent_left: 0.0,
            indent_right: 0.0,
            indent_first_line: 0.0,
            keep_with_next: false,
            keep_lines: false,
            page_break_before: false,
        }
    }
}

/// Resolved run (character) style attributes.
#[derive(Debug, Clone)]
pub struct ResolvedRunStyle {
    /// Font family name.
    pub font_family: String,
    /// Font size in points.
    pub font_size: f64,
    /// Text color.
    pub color: Color,
    /// Bold.
    pub bold: bool,
    /// Italic.
    pub italic: bool,
    /// Underline.
    pub underline: bool,
    /// Strikethrough.
    pub strikethrough: bool,
    /// Superscript.
    pub superscript: bool,
    /// Subscript.
    pub subscript: bool,
    /// Highlight/background color.
    pub highlight_color: Option<Color>,
    /// Character spacing in points (letter-spacing).
    pub character_spacing: f64,
    /// Revision type for track changes (e.g., "insertion", "deletion").
    pub revision_type: Option<String>,
    /// Revision author for track changes.
    pub revision_author: Option<String>,
}

impl Default for ResolvedRunStyle {
    fn default() -> Self {
        Self {
            font_family: DEFAULT_FONT_FAMILY.to_string(),
            font_size: DEFAULT_FONT_SIZE,
            color: DEFAULT_COLOR,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            superscript: false,
            subscript: false,
            highlight_color: None,
            character_spacing: 0.0,
            revision_type: None,
            revision_author: None,
        }
    }
}

/// Resolve paragraph style from a paragraph node.
pub fn resolve_paragraph_style(doc: &DocumentModel, node_id: NodeId) -> ResolvedParagraphStyle {
    let mut style = ResolvedParagraphStyle::default();

    let node = match doc.node(node_id) {
        Some(n) => n,
        None => return style,
    };

    // If the paragraph references a style, resolve from styles first
    if let Some(style_id) = node.attributes.get_string(&AttributeKey::StyleId) {
        apply_paragraph_style_chain(doc, style_id, &mut style);
    }

    // Then apply direct attributes (they override style)
    apply_paragraph_attrs(node, &mut style);

    style
}

/// Resolve run style from a run node.
pub fn resolve_run_style(doc: &DocumentModel, node_id: NodeId) -> ResolvedRunStyle {
    let mut style = ResolvedRunStyle::default();

    let node = match doc.node(node_id) {
        Some(n) => n,
        None => return style,
    };

    // If the run's parent paragraph has a style, get default run props from it
    if let Some(parent_id) = node.parent {
        if let Some(parent) = doc.node(parent_id) {
            if let Some(style_id) = parent.attributes.get_string(&AttributeKey::StyleId) {
                apply_run_style_from_paragraph_chain(doc, style_id, &mut style);
            }
        }
    }

    // Apply direct attributes on the run
    apply_run_attrs(node, &mut style);

    style
}

fn apply_paragraph_style_chain(
    doc: &DocumentModel,
    style_id: &str,
    resolved: &mut ResolvedParagraphStyle,
) {
    let chain = build_style_chain(doc, style_id);
    for sid in chain {
        if let Some(s) = doc.style_by_id(&sid) {
            if let Some(AttributeValue::Alignment(a)) = s.attributes.get(&AttributeKey::Alignment) {
                resolved.alignment = *a;
            }
            apply_spacing_from_attrs(&s.attributes, resolved);
        }
    }
}

fn apply_run_style_from_paragraph_chain(
    doc: &DocumentModel,
    style_id: &str,
    resolved: &mut ResolvedRunStyle,
) {
    let chain = build_style_chain(doc, style_id);
    for sid in chain {
        if let Some(s) = doc.style_by_id(&sid) {
            apply_run_attrs_from_map(&s.attributes, resolved);
        }
    }
}

fn build_style_chain(doc: &DocumentModel, style_id: &str) -> Vec<String> {
    let mut chain = Vec::new();
    let mut current = Some(style_id.to_string());

    while let Some(sid) = current {
        if chain.contains(&sid) {
            break; // prevent cycles
        }
        chain.push(sid.clone());
        current = doc.style_by_id(&sid).and_then(|s| s.parent_id.clone());
    }

    chain.reverse(); // root first
    chain
}

fn apply_paragraph_attrs(node: &Node, style: &mut ResolvedParagraphStyle) {
    if let Some(AttributeValue::Alignment(a)) = node.attributes.get(&AttributeKey::Alignment) {
        style.alignment = *a;
    }
    apply_spacing_from_attrs(&node.attributes, style);
    if let Some(AttributeValue::Bool(v)) = node.attributes.get(&AttributeKey::KeepWithNext) {
        style.keep_with_next = *v;
    }
    if let Some(AttributeValue::Bool(v)) = node.attributes.get(&AttributeKey::KeepLinesTogether) {
        style.keep_lines = *v;
    }
    if let Some(AttributeValue::Bool(v)) = node.attributes.get(&AttributeKey::PageBreakBefore) {
        style.page_break_before = *v;
    }
}

fn apply_spacing_from_attrs(attrs: &s1_model::AttributeMap, style: &mut ResolvedParagraphStyle) {
    if let Some(AttributeValue::Float(v)) = attrs.get(&AttributeKey::SpacingBefore) {
        style.space_before = *v;
    }
    if let Some(AttributeValue::Float(v)) = attrs.get(&AttributeKey::SpacingAfter) {
        style.space_after = *v;
    }
    if let Some(AttributeValue::LineSpacing(ls)) = attrs.get(&AttributeKey::LineSpacing) {
        style.line_spacing = *ls;
    }
    if let Some(AttributeValue::Float(v)) = attrs.get(&AttributeKey::IndentLeft) {
        style.indent_left = *v;
    }
    if let Some(AttributeValue::Float(v)) = attrs.get(&AttributeKey::IndentRight) {
        style.indent_right = *v;
    }
    if let Some(AttributeValue::Float(v)) = attrs.get(&AttributeKey::IndentFirstLine) {
        style.indent_first_line = *v;
    }
}

fn apply_run_attrs(node: &Node, style: &mut ResolvedRunStyle) {
    apply_run_attrs_from_map(&node.attributes, style);
}

fn apply_run_attrs_from_map(attrs: &s1_model::AttributeMap, style: &mut ResolvedRunStyle) {
    if let Some(AttributeValue::String(f)) = attrs.get(&AttributeKey::FontFamily) {
        style.font_family = f.clone();
    }
    if let Some(AttributeValue::Float(s)) = attrs.get(&AttributeKey::FontSize) {
        style.font_size = *s;
    }
    if let Some(AttributeValue::Color(c)) = attrs.get(&AttributeKey::Color) {
        style.color = *c;
    }
    if let Some(AttributeValue::Bool(v)) = attrs.get(&AttributeKey::Bold) {
        style.bold = *v;
    }
    if let Some(AttributeValue::Bool(v)) = attrs.get(&AttributeKey::Italic) {
        style.italic = *v;
    }
    if let Some(val) = attrs.get(&AttributeKey::Underline) {
        match val {
            AttributeValue::Bool(v) => style.underline = *v,
            AttributeValue::String(s) => style.underline = s != "none",
            _ => {}
        }
    }
    if let Some(AttributeValue::Bool(v)) = attrs.get(&AttributeKey::Strikethrough) {
        style.strikethrough = *v;
    }
    if let Some(AttributeValue::Bool(v)) = attrs.get(&AttributeKey::Superscript) {
        style.superscript = *v;
    }
    if let Some(AttributeValue::Bool(v)) = attrs.get(&AttributeKey::Subscript) {
        style.subscript = *v;
    }
    if let Some(AttributeValue::Color(c)) = attrs.get(&AttributeKey::HighlightColor) {
        style.highlight_color = Some(*c);
    }
    if let Some(AttributeValue::Float(v)) = attrs.get(&AttributeKey::FontSpacing) {
        style.character_spacing = *v;
    }
    if let Some(AttributeValue::String(v)) = attrs.get(&AttributeKey::RevisionType) {
        style.revision_type = Some(v.clone());
    }
    if let Some(AttributeValue::String(v)) = attrs.get(&AttributeKey::RevisionAuthor) {
        style.revision_author = Some(v.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::{DocumentModel, NodeType};

    #[test]
    fn default_paragraph_style() {
        let style = ResolvedParagraphStyle::default();
        assert_eq!(style.alignment, Alignment::Left);
        assert_eq!(style.space_before, 0.0);
        assert!(!style.keep_with_next);
    }

    #[test]
    fn default_run_style() {
        let style = ResolvedRunStyle::default();
        assert_eq!(style.font_family, "Times New Roman");
        assert_eq!(style.font_size, 12.0);
        assert!(!style.bold);
        assert!(!style.italic);
    }

    #[test]
    fn resolve_paragraph_with_direct_attrs() {
        let mut doc = DocumentModel::new();
        let root_id = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root_id, 0, Node::new(body_id, NodeType::Body))
            .unwrap();
        let para_id = doc.next_id();
        let mut para = Node::new(para_id, NodeType::Paragraph);
        para.attributes.set(
            AttributeKey::Alignment,
            AttributeValue::Alignment(Alignment::Center),
        );
        para.attributes
            .set(AttributeKey::SpacingBefore, AttributeValue::Float(12.0));
        doc.insert_node(body_id, 0, para).unwrap();

        let style = resolve_paragraph_style(&doc, para_id);
        assert_eq!(style.alignment, Alignment::Center);
        assert_eq!(style.space_before, 12.0);
    }

    #[test]
    fn resolve_run_with_direct_attrs() {
        let mut doc = DocumentModel::new();
        let root_id = doc.root_id();
        let body_id = doc.next_id();
        doc.insert_node(root_id, 0, Node::new(body_id, NodeType::Body))
            .unwrap();
        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();
        let run_id = doc.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes
            .set(AttributeKey::Bold, AttributeValue::Bool(true));
        run.attributes
            .set(AttributeKey::FontSize, AttributeValue::Float(24.0));
        run.attributes.set(
            AttributeKey::FontFamily,
            AttributeValue::String("Arial".into()),
        );
        doc.insert_node(para_id, 0, run).unwrap();

        let style = resolve_run_style(&doc, run_id);
        assert!(style.bold);
        assert_eq!(style.font_size, 24.0);
        assert_eq!(style.font_family, "Arial");
    }

    #[test]
    fn resolve_nonexistent_node() {
        let doc = DocumentModel::new();
        let fake_id = NodeId::new(999, 999);
        let p_style = resolve_paragraph_style(&doc, fake_id);
        assert_eq!(p_style.alignment, Alignment::Left);
        let r_style = resolve_run_style(&doc, fake_id);
        assert_eq!(r_style.font_family, "Times New Roman");
    }

    #[test]
    fn page_layout_letter() {
        let layout = PageLayout::letter();
        assert_eq!(layout.width, 612.0);
        assert_eq!(layout.height, 792.0);
        assert_eq!(layout.content_width(), 468.0);
        assert_eq!(layout.content_height(), 648.0);
    }

    #[test]
    fn page_layout_content_rect() {
        let layout = PageLayout::letter();
        let rect = layout.content_rect();
        assert_eq!(rect.x, 72.0);
        assert_eq!(rect.y, 72.0);
        assert_eq!(rect.width, 468.0);
        assert_eq!(rect.height, 648.0);
    }

    use crate::types::PageLayout;
}
