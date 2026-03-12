//! Ergonomic document builder.
//!
//! [`DocumentBuilder`] provides a fluent API for constructing documents
//! without manually creating nodes and operations.
//!
//! # Example
//!
//! ```
//! use s1engine::DocumentBuilder;
//!
//! let doc = DocumentBuilder::new()
//!     .heading(1, "Introduction")
//!     .paragraph(|p| {
//!         p.text("This is ")
//!          .bold("important")
//!          .text(" content.")
//!     })
//!     .build();
//!
//! assert_eq!(doc.to_plain_text(), "Introduction\nThis is important content.");
//! ```

use s1_model::{
    AbstractNumbering, AttributeKey, AttributeMap, AttributeValue, Color, DocumentModel,
    HeaderFooterRef, HeaderFooterType, ListFormat, ListInfo, Node, NodeId, NodeType,
    NumberingInstance, NumberingLevel, SectionProperties, Style, StyleType, TableWidth,
    UnderlineStyle,
};

use crate::document::Document;

/// Fluent builder for constructing documents.
pub struct DocumentBuilder {
    model: DocumentModel,
}

impl DocumentBuilder {
    /// Create a new builder with an empty document.
    pub fn new() -> Self {
        Self {
            model: DocumentModel::new(),
        }
    }

    /// Add a heading paragraph.
    ///
    /// `level` should be 1-6. The heading is given a style reference
    /// `"Heading{level}"` and a corresponding style is auto-created if
    /// it doesn't already exist.
    pub fn heading(mut self, level: u8, text: &str) -> Self {
        let level = level.clamp(1, 6);
        let style_id = format!("Heading{level}");

        // Auto-create the heading style if it doesn't exist
        if self.model.style_by_id(&style_id).is_none() {
            let name = format!("Heading {level}");
            let font_size = match level {
                1 => 24.0,
                2 => 18.0,
                3 => 14.0,
                _ => 12.0,
            };
            let mut style = Style::new(&style_id, &name, StyleType::Paragraph);
            style.attributes = AttributeMap::new().bold(true).font_size(font_size);
            self.model.set_style(style);
        }

        let body_id = self.model.body_id().unwrap();
        let child_count = self.model.node(body_id).unwrap().children.len();

        let para_id = self.model.next_id();
        let mut para = Node::new(para_id, NodeType::Paragraph);
        para.attributes.set(
            s1_model::AttributeKey::StyleId,
            s1_model::AttributeValue::String(style_id.clone()),
        );
        self.model.insert_node(body_id, child_count, para).unwrap();

        let run_id = self.model.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        let font_size = match level {
            1 => 24.0,
            2 => 18.0,
            3 => 14.0,
            _ => 12.0,
        };
        run.attributes = AttributeMap::new().bold(true).font_size(font_size);
        self.model.insert_node(para_id, 0, run).unwrap();

        let text_id = self.model.next_id();
        self.model
            .insert_node(run_id, 0, Node::text(text_id, text))
            .unwrap();

        self
    }

    /// Add a paragraph built with a [`ParagraphBuilder`].
    pub fn paragraph(mut self, f: impl FnOnce(ParagraphBuilder) -> ParagraphBuilder) -> Self {
        let body_id = self.model.body_id().unwrap();
        let child_count = self.model.node(body_id).unwrap().children.len();

        let para_id = self.model.next_id();
        self.model
            .insert_node(
                body_id,
                child_count,
                Node::new(para_id, NodeType::Paragraph),
            )
            .unwrap();

        let pb = ParagraphBuilder {
            model: &mut self.model,
            para_id,
        };
        f(pb);

        self
    }

    /// Add a plain text paragraph (shorthand for `.paragraph(|p| p.text(...))`).
    pub fn text(self, text: &str) -> Self {
        let t = text.to_string();
        self.paragraph(move |p| p.text(&t))
    }

    /// Add a bulleted list item.
    ///
    /// Auto-creates a bullet numbering definition on first use.
    pub fn bullet(mut self, text: &str) -> Self {
        self.ensure_bullet_numbering();
        self.add_list_paragraph(text, 0, ListFormat::Bullet, 1)
    }

    /// Add a numbered list item.
    ///
    /// Auto-creates a decimal numbering definition on first use.
    pub fn numbered(mut self, text: &str) -> Self {
        self.ensure_numbered_numbering();
        self.add_list_paragraph(text, 0, ListFormat::Decimal, 2)
    }

    /// Add a list item at a specific level and format.
    ///
    /// `num_id` must match an existing numbering instance (use `bullet()`
    /// or `numbered()` for auto-created definitions).
    pub fn list_item(self, text: &str, level: u8, format: ListFormat, num_id: u32) -> Self {
        self.add_list_paragraph(text, level, format, num_id)
    }

    fn add_list_paragraph(
        mut self,
        text: &str,
        level: u8,
        format: ListFormat,
        num_id: u32,
    ) -> Self {
        let body_id = self.model.body_id().unwrap();
        let child_count = self.model.node(body_id).unwrap().children.len();

        let para_id = self.model.next_id();
        let mut para = Node::new(para_id, NodeType::Paragraph);
        para.attributes.set(
            AttributeKey::ListInfo,
            AttributeValue::ListInfo(ListInfo {
                level,
                num_format: format,
                num_id,
                start: None,
            }),
        );
        self.model.insert_node(body_id, child_count, para).unwrap();

        let run_id = self.model.next_id();
        self.model
            .insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();

        let text_id = self.model.next_id();
        self.model
            .insert_node(run_id, 0, Node::text(text_id, text))
            .unwrap();

        self
    }

    /// Ensure a bullet numbering definition exists (abstractNumId=0, numId=1).
    fn ensure_bullet_numbering(&mut self) {
        let numbering = self.model.numbering();
        if numbering.instances.iter().any(|i| i.num_id == 1) {
            return;
        }
        // Create abstract numbering for bullets
        if !numbering
            .abstract_nums
            .iter()
            .any(|a| a.abstract_num_id == 0)
        {
            self.model
                .numbering_mut()
                .abstract_nums
                .push(AbstractNumbering {
                    abstract_num_id: 0,
                    name: Some("BulletList".into()),
                    levels: vec![NumberingLevel {
                        level: 0,
                        num_format: ListFormat::Bullet,
                        level_text: "\u{2022}".into(),
                        start: 1,
                        indent_left: Some(36.0),
                        indent_hanging: Some(18.0),
                        alignment: None,
                        bullet_font: Some("Symbol".into()),
                    }],
                });
        }
        self.model
            .numbering_mut()
            .instances
            .push(NumberingInstance {
                num_id: 1,
                abstract_num_id: 0,
                level_overrides: vec![],
            });
    }

    /// Ensure a decimal numbering definition exists (abstractNumId=1, numId=2).
    fn ensure_numbered_numbering(&mut self) {
        let numbering = self.model.numbering();
        if numbering.instances.iter().any(|i| i.num_id == 2) {
            return;
        }
        if !numbering
            .abstract_nums
            .iter()
            .any(|a| a.abstract_num_id == 1)
        {
            self.model
                .numbering_mut()
                .abstract_nums
                .push(AbstractNumbering {
                    abstract_num_id: 1,
                    name: Some("NumberedList".into()),
                    levels: vec![NumberingLevel {
                        level: 0,
                        num_format: ListFormat::Decimal,
                        level_text: "%1.".into(),
                        start: 1,
                        indent_left: Some(36.0),
                        indent_hanging: Some(18.0),
                        alignment: None,
                        bullet_font: None,
                    }],
                });
        }
        self.model
            .numbering_mut()
            .instances
            .push(NumberingInstance {
                num_id: 2,
                abstract_num_id: 1,
                level_overrides: vec![],
            });
    }

    /// Add a Table of Contents block.
    ///
    /// `max_level` controls the deepest heading level included (1-9).
    /// The TOC node is created as a placeholder. Call [`Document::update_toc`]
    /// or export to have entries generated from document headings.
    pub fn table_of_contents(mut self, max_level: u8) -> Self {
        let max_level = max_level.clamp(1, 9);
        let body_id = self.model.body_id().unwrap();
        let child_count = self.model.node(body_id).unwrap().children.len();

        let toc_id = self.model.next_id();
        let mut toc = Node::new(toc_id, NodeType::TableOfContents);
        toc.attributes.set(
            AttributeKey::TocMaxLevel,
            AttributeValue::Int(max_level as i64),
        );
        self.model.insert_node(body_id, child_count, toc).unwrap();
        self
    }

    /// Add a Table of Contents block with a custom title.
    pub fn table_of_contents_with_title(mut self, max_level: u8, title: &str) -> Self {
        let max_level = max_level.clamp(1, 9);
        let body_id = self.model.body_id().unwrap();
        let child_count = self.model.node(body_id).unwrap().children.len();

        let toc_id = self.model.next_id();
        let mut toc = Node::new(toc_id, NodeType::TableOfContents);
        toc.attributes.set(
            AttributeKey::TocMaxLevel,
            AttributeValue::Int(max_level as i64),
        );
        toc.attributes.set(
            AttributeKey::TocTitle,
            AttributeValue::String(title.to_string()),
        );
        self.model.insert_node(body_id, child_count, toc).unwrap();
        self
    }

    /// Add a table built with a [`TableBuilder`].
    ///
    /// # Example
    ///
    /// ```
    /// use s1engine::DocumentBuilder;
    ///
    /// let doc = DocumentBuilder::new()
    ///     .table(|t| {
    ///         t.row(|r| r.cell("Name").cell("Age"))
    ///          .row(|r| r.cell("Alice").cell("30"))
    ///     })
    ///     .build();
    /// ```
    pub fn table(mut self, f: impl FnOnce(TableBuilder) -> TableBuilder) -> Self {
        let body_id = self.model.body_id().unwrap();
        let child_count = self.model.node(body_id).unwrap().children.len();

        let table_id = self.model.next_id();
        self.model
            .insert_node(body_id, child_count, Node::new(table_id, NodeType::Table))
            .unwrap();

        let tb = TableBuilder {
            model: &mut self.model,
            table_id,
        };
        f(tb);

        self
    }

    /// Set document title metadata.
    pub fn title(mut self, title: &str) -> Self {
        self.model.metadata_mut().title = Some(title.to_string());
        self
    }

    /// Set document author/creator metadata.
    pub fn author(mut self, author: &str) -> Self {
        self.model.metadata_mut().creator = Some(author.to_string());
        self
    }

    /// Add a section with the given properties.
    ///
    /// Sections define page layout (size, margins, orientation, columns)
    /// and can reference headers/footers. The final section applies to
    /// all content after the last section break.
    pub fn section(mut self, props: SectionProperties) -> Self {
        self.model.sections_mut().push(props);
        self
    }

    /// Add a section with default properties and a header containing the given text.
    pub fn section_with_header(mut self, header_text: &str) -> Self {
        let header_id = self.create_hf_node(NodeType::Header, header_text);

        let mut props = SectionProperties::default();
        props.headers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: header_id,
        });
        self.model.sections_mut().push(props);
        self
    }

    /// Add a section with default properties and a footer containing the given text.
    pub fn section_with_footer(mut self, footer_text: &str) -> Self {
        let footer_id = self.create_hf_node(NodeType::Footer, footer_text);

        let mut props = SectionProperties::default();
        props.footers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: footer_id,
        });
        self.model.sections_mut().push(props);
        self
    }

    /// Add a section with both a header and footer.
    pub fn section_with_header_footer(mut self, header_text: &str, footer_text: &str) -> Self {
        let header_id = self.create_hf_node(NodeType::Header, header_text);
        let footer_id = self.create_hf_node(NodeType::Footer, footer_text);

        let mut props = SectionProperties::default();
        props.headers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: header_id,
        });
        props.footers.push(HeaderFooterRef {
            hf_type: HeaderFooterType::Default,
            node_id: footer_id,
        });
        self.model.sections_mut().push(props);
        self
    }

    /// Create a header or footer node with a text paragraph as a child of the document root.
    fn create_hf_node(&mut self, node_type: NodeType, text: &str) -> NodeId {
        let root_id = self.model.root_id();
        let child_count = self.model.node(root_id).unwrap().children.len();

        let hf_id = self.model.next_id();
        self.model
            .insert_node(root_id, child_count, Node::new(hf_id, node_type))
            .unwrap();

        let para_id = self.model.next_id();
        self.model
            .insert_node(hf_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = self.model.next_id();
        self.model
            .insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();

        let text_id = self.model.next_id();
        self.model
            .insert_node(run_id, 0, Node::text(text_id, text))
            .unwrap();

        hf_id
    }

    /// Consume the builder and produce a [`Document`].
    pub fn build(self) -> Document {
        Document::from_model(self.model)
    }
}

impl Default for DocumentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for inline content within a paragraph.
pub struct ParagraphBuilder<'a> {
    model: &'a mut DocumentModel,
    para_id: NodeId,
}

impl<'a> ParagraphBuilder<'a> {
    /// Add a plain text run.
    pub fn text(self, text: &str) -> Self {
        self.run_with_attrs(text, AttributeMap::new())
    }

    /// Add a bold text run.
    pub fn bold(self, text: &str) -> Self {
        self.run_with_attrs(text, AttributeMap::new().bold(true))
    }

    /// Add an italic text run.
    pub fn italic(self, text: &str) -> Self {
        self.run_with_attrs(text, AttributeMap::new().italic(true))
    }

    /// Add a bold+italic text run.
    pub fn bold_italic(self, text: &str) -> Self {
        self.run_with_attrs(text, AttributeMap::new().bold(true).italic(true))
    }

    /// Add an underlined text run.
    pub fn underline(self, text: &str) -> Self {
        let mut attrs = AttributeMap::new();
        attrs.set(
            s1_model::AttributeKey::Underline,
            s1_model::AttributeValue::UnderlineStyle(UnderlineStyle::Single),
        );
        self.run_with_attrs(text, attrs)
    }

    /// Add a text run with a specific font and size.
    pub fn styled(self, text: &str, font: &str, size: f64) -> Self {
        self.run_with_attrs(text, AttributeMap::new().font_family(font).font_size(size))
    }

    /// Add a text run with a specific color.
    pub fn colored(self, text: &str, color: Color) -> Self {
        self.run_with_attrs(text, AttributeMap::new().color(color))
    }

    /// Add a line break within the paragraph.
    pub fn line_break(self) -> Self {
        let child_count = self.model.node(self.para_id).unwrap().children.len();
        let br_id = self.model.next_id();
        self.model
            .insert_node(
                self.para_id,
                child_count,
                Node::new(br_id, NodeType::LineBreak),
            )
            .unwrap();
        self
    }

    /// Add a superscript text run.
    pub fn superscript(self, text: &str) -> Self {
        let mut attrs = AttributeMap::new();
        attrs.set(AttributeKey::Superscript, AttributeValue::Bool(true));
        self.run_with_attrs(text, attrs)
    }

    /// Add a subscript text run.
    pub fn subscript(self, text: &str) -> Self {
        let mut attrs = AttributeMap::new();
        attrs.set(AttributeKey::Subscript, AttributeValue::Bool(true));
        self.run_with_attrs(text, attrs)
    }

    /// Add a hyperlink text run with an external URL.
    pub fn hyperlink(self, url: &str, text: &str) -> Self {
        let mut attrs = AttributeMap::new();
        attrs.set(
            AttributeKey::HyperlinkUrl,
            AttributeValue::String(url.to_string()),
        );
        self.run_with_attrs(text, attrs)
    }

    /// Add a bookmark start marker.
    pub fn bookmark_start(self, name: &str) -> Self {
        let child_count = self.model.node(self.para_id).unwrap().children.len();
        let bk_id = self.model.next_id();
        let mut bk = Node::new(bk_id, NodeType::BookmarkStart);
        bk.attributes.set(
            AttributeKey::BookmarkName,
            AttributeValue::String(name.to_string()),
        );
        self.model
            .insert_node(self.para_id, child_count, bk)
            .unwrap();
        self
    }

    /// Add a bookmark end marker.
    pub fn bookmark_end(self) -> Self {
        let child_count = self.model.node(self.para_id).unwrap().children.len();
        let bk_id = self.model.next_id();
        self.model
            .insert_node(
                self.para_id,
                child_count,
                Node::new(bk_id, NodeType::BookmarkEnd),
            )
            .unwrap();
        self
    }

    /// Add a run with custom attributes.
    fn run_with_attrs(self, text: &str, attrs: AttributeMap) -> Self {
        let child_count = self.model.node(self.para_id).unwrap().children.len();

        let run_id = self.model.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes = attrs;
        self.model
            .insert_node(self.para_id, child_count, run)
            .unwrap();

        let text_id = self.model.next_id();
        self.model
            .insert_node(run_id, 0, Node::text(text_id, text))
            .unwrap();

        self
    }
}

/// Builder for table content.
pub struct TableBuilder<'a> {
    model: &'a mut DocumentModel,
    table_id: NodeId,
}

impl<'a> TableBuilder<'a> {
    /// Add a row built with a [`RowBuilder`].
    pub fn row(self, f: impl FnOnce(RowBuilder) -> RowBuilder) -> Self {
        let row_count = self.model.node(self.table_id).unwrap().children.len();
        let row_id = self.model.next_id();
        self.model
            .insert_node(
                self.table_id,
                row_count,
                Node::new(row_id, NodeType::TableRow),
            )
            .unwrap();

        let rb = RowBuilder {
            model: self.model,
            row_id,
        };
        f(rb);

        Self {
            model: self.model,
            table_id: self.table_id,
        }
    }

    /// Set the table width.
    pub fn width(self, width: TableWidth) -> Self {
        if let Some(node) = self.model.node_mut(self.table_id) {
            node.attributes
                .set(AttributeKey::TableWidth, AttributeValue::TableWidth(width));
        }
        self
    }
}

/// Builder for table row content.
pub struct RowBuilder<'a> {
    model: &'a mut DocumentModel,
    row_id: NodeId,
}

impl<'a> RowBuilder<'a> {
    /// Add a cell with plain text content.
    pub fn cell(self, text: &str) -> Self {
        let cell_count = self.model.node(self.row_id).unwrap().children.len();
        let cell_id = self.model.next_id();
        self.model
            .insert_node(
                self.row_id,
                cell_count,
                Node::new(cell_id, NodeType::TableCell),
            )
            .unwrap();

        // Add a paragraph with a text run inside the cell
        let para_id = self.model.next_id();
        self.model
            .insert_node(cell_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = self.model.next_id();
        self.model
            .insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();

        let text_id = self.model.next_id();
        self.model
            .insert_node(run_id, 0, Node::text(text_id, text))
            .unwrap();

        Self {
            model: self.model,
            row_id: self.row_id,
        }
    }

    /// Add a cell with rich content built via a closure.
    pub fn rich_cell(self, f: impl FnOnce(ParagraphBuilder) -> ParagraphBuilder) -> Self {
        let cell_count = self.model.node(self.row_id).unwrap().children.len();
        let cell_id = self.model.next_id();
        self.model
            .insert_node(
                self.row_id,
                cell_count,
                Node::new(cell_id, NodeType::TableCell),
            )
            .unwrap();

        let para_id = self.model.next_id();
        self.model
            .insert_node(cell_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let pb = ParagraphBuilder {
            model: self.model,
            para_id,
        };
        f(pb);

        Self {
            model: self.model,
            row_id: self.row_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_empty_document() {
        let doc = DocumentBuilder::new().build();
        assert_eq!(doc.to_plain_text(), "");
    }

    #[test]
    fn build_single_paragraph() {
        let doc = DocumentBuilder::new().text("Hello World").build();
        assert_eq!(doc.to_plain_text(), "Hello World");
    }

    #[test]
    fn build_heading() {
        let doc = DocumentBuilder::new().heading(1, "Title").build();
        assert_eq!(doc.to_plain_text(), "Title");

        // Should have created a Heading1 style
        assert!(doc.style_by_id("Heading1").is_some());
    }

    #[test]
    fn build_mixed_content() {
        let doc = DocumentBuilder::new()
            .heading(1, "Introduction")
            .paragraph(|p| p.text("This is ").bold("important").text(" content."))
            .text("Plain paragraph.")
            .build();

        assert_eq!(
            doc.to_plain_text(),
            "Introduction\nThis is important content.\nPlain paragraph."
        );
        assert_eq!(doc.paragraph_count(), 3);
    }

    #[test]
    fn build_with_formatting() {
        let doc = DocumentBuilder::new()
            .paragraph(|p| {
                p.bold("Bold")
                    .text(" and ")
                    .italic("italic")
                    .text(" and ")
                    .bold_italic("both")
            })
            .build();

        assert_eq!(doc.to_plain_text(), "Bold and italic and both");

        // Check bold attribute on first run
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let bold_run = doc.node(para.children[0]).unwrap();
        assert_eq!(
            bold_run.attributes.get_bool(&s1_model::AttributeKey::Bold),
            Some(true)
        );
    }

    #[test]
    fn build_with_metadata() {
        let doc = DocumentBuilder::new()
            .title("My Report")
            .author("Alice")
            .text("Content")
            .build();

        assert_eq!(doc.metadata().title.as_deref(), Some("My Report"));
        assert_eq!(doc.metadata().creator.as_deref(), Some("Alice"));
    }

    #[test]
    fn build_with_underline() {
        let doc = DocumentBuilder::new()
            .paragraph(|p| p.underline("underlined"))
            .build();

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        let run = doc.node(para.children[0]).unwrap();
        assert!(run
            .attributes
            .get(&s1_model::AttributeKey::Underline)
            .is_some());
    }

    #[test]
    fn build_heading_levels() {
        let doc = DocumentBuilder::new()
            .heading(1, "H1")
            .heading(2, "H2")
            .heading(3, "H3")
            .build();

        assert_eq!(doc.to_plain_text(), "H1\nH2\nH3");
        assert!(doc.style_by_id("Heading1").is_some());
        assert!(doc.style_by_id("Heading2").is_some());
        assert!(doc.style_by_id("Heading3").is_some());
    }

    #[test]
    fn build_with_line_break() {
        let doc = DocumentBuilder::new()
            .paragraph(|p| p.text("Line 1").line_break().text("Line 2"))
            .build();

        // The paragraph has: Run("Line 1"), LineBreak, Run("Line 2")
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        assert_eq!(para.children.len(), 3);
        assert_eq!(
            doc.node(para.children[1]).unwrap().node_type,
            NodeType::LineBreak
        );
    }

    #[test]
    fn build_simple_table() {
        let doc = DocumentBuilder::new()
            .table(|t| {
                t.row(|r| r.cell("A1").cell("B1"))
                    .row(|r| r.cell("A2").cell("B2"))
            })
            .build();

        let text = doc.to_plain_text();
        assert!(text.contains("A1"));
        assert!(text.contains("B2"));

        // Verify structure
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        assert_eq!(body.children.len(), 1); // one table

        let table = doc.node(body.children[0]).unwrap();
        assert_eq!(table.node_type, NodeType::Table);
        assert_eq!(table.children.len(), 2); // two rows

        let row0 = doc.node(table.children[0]).unwrap();
        assert_eq!(row0.children.len(), 2); // two cells
    }

    #[test]
    fn build_table_with_rich_cells() {
        let doc = DocumentBuilder::new()
            .table(|t| t.row(|r| r.rich_cell(|p| p.bold("Header")).cell("Value")))
            .build();

        let text = doc.to_plain_text();
        assert!(text.contains("Header"));
        assert!(text.contains("Value"));
    }

    #[test]
    fn build_table_mixed_with_paragraphs() {
        let doc = DocumentBuilder::new()
            .text("Before")
            .table(|t| t.row(|r| r.cell("Cell")))
            .text("After")
            .build();

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        assert_eq!(body.children.len(), 3); // para, table, para
    }

    #[cfg(feature = "docx")]
    #[test]
    fn build_table_docx_roundtrip() {
        let doc = DocumentBuilder::new()
            .table(|t| t.row(|r| r.cell("Hello").cell("World")))
            .build();

        let bytes = doc.export(crate::Format::Docx).unwrap();

        let engine = crate::Engine::new();
        let doc2 = engine.open(&bytes).unwrap();

        let text = doc2.to_plain_text();
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));

        // Verify table structure survived
        let body_id = doc2.body_id().unwrap();
        let body = doc2.node(body_id).unwrap();
        let table = doc2.node(body.children[0]).unwrap();
        assert_eq!(table.node_type, NodeType::Table);
    }

    #[test]
    fn build_bullet_list() {
        let doc = DocumentBuilder::new()
            .bullet("First")
            .bullet("Second")
            .build();

        assert_eq!(doc.to_plain_text(), "First\nSecond");

        // Verify ListInfo on paragraphs
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        match para.attributes.get(&AttributeKey::ListInfo) {
            Some(AttributeValue::ListInfo(info)) => {
                assert_eq!(info.num_format, ListFormat::Bullet);
                assert_eq!(info.level, 0);
                assert_eq!(info.num_id, 1);
            }
            other => panic!("Expected ListInfo, got {:?}", other),
        }

        // Verify numbering definitions were created
        assert!(!doc.numbering().is_empty());
        assert_eq!(doc.numbering().instances.len(), 1);
    }

    #[test]
    fn build_numbered_list() {
        let doc = DocumentBuilder::new()
            .numbered("One")
            .numbered("Two")
            .numbered("Three")
            .build();

        assert_eq!(doc.to_plain_text(), "One\nTwo\nThree");

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let para = doc.node(body.children[0]).unwrap();
        match para.attributes.get(&AttributeKey::ListInfo) {
            Some(AttributeValue::ListInfo(info)) => {
                assert_eq!(info.num_format, ListFormat::Decimal);
                assert_eq!(info.num_id, 2);
            }
            other => panic!("Expected ListInfo, got {:?}", other),
        }
    }

    #[test]
    fn build_mixed_lists() {
        let doc = DocumentBuilder::new()
            .bullet("Bullet item")
            .numbered("Numbered item")
            .text("Plain text")
            .build();

        assert_eq!(
            doc.to_plain_text(),
            "Bullet item\nNumbered item\nPlain text"
        );

        // Should have both numbering definitions
        assert_eq!(doc.numbering().abstract_nums.len(), 2);
        assert_eq!(doc.numbering().instances.len(), 2);
    }

    #[cfg(feature = "docx")]
    #[test]
    fn build_list_docx_roundtrip() {
        let doc = DocumentBuilder::new()
            .bullet("Item A")
            .bullet("Item B")
            .numbered("Step 1")
            .build();

        let bytes = doc.export(crate::Format::Docx).unwrap();
        let engine = crate::Engine::new();
        let doc2 = engine.open(&bytes).unwrap();

        let text = doc2.to_plain_text();
        assert!(text.contains("Item A"));
        assert!(text.contains("Step 1"));

        // Verify numbering survived
        assert!(!doc2.numbering().is_empty());

        // Verify ListInfo on first paragraph
        let body_id = doc2.body_id().unwrap();
        let body = doc2.node(body_id).unwrap();
        let para = doc2.node(body.children[0]).unwrap();
        assert!(para.attributes.get(&AttributeKey::ListInfo).is_some());
    }

    #[test]
    fn build_with_section() {
        let doc = DocumentBuilder::new()
            .text("Body content")
            .section(SectionProperties::default())
            .build();

        assert_eq!(doc.sections().len(), 1);
        assert!((doc.sections()[0].page_width - 612.0).abs() < 0.01);
    }

    #[test]
    fn build_with_header_footer() {
        let doc = DocumentBuilder::new()
            .text("Body")
            .section_with_header_footer("My Header", "My Footer")
            .build();

        assert_eq!(doc.sections().len(), 1);
        assert_eq!(doc.sections()[0].headers.len(), 1);
        assert_eq!(doc.sections()[0].footers.len(), 1);

        // Verify header content
        let hdr_id = doc.sections()[0].headers[0].node_id;
        let hdr = doc.node(hdr_id).unwrap();
        assert_eq!(hdr.node_type, NodeType::Header);

        // Verify footer content
        let ftr_id = doc.sections()[0].footers[0].node_id;
        let ftr = doc.node(ftr_id).unwrap();
        assert_eq!(ftr.node_type, NodeType::Footer);
    }

    #[cfg(feature = "docx")]
    #[test]
    fn build_section_docx_roundtrip() {
        let doc = DocumentBuilder::new()
            .text("Body content")
            .section_with_header_footer("Header Text", "Footer Text")
            .build();

        let bytes = doc.export(crate::Format::Docx).unwrap();
        let engine = crate::Engine::new();
        let doc2 = engine.open(&bytes).unwrap();

        assert_eq!(doc2.to_plain_text(), "Body content");
        assert_eq!(doc2.sections().len(), 1);
        assert_eq!(doc2.sections()[0].headers.len(), 1);
        assert_eq!(doc2.sections()[0].footers.len(), 1);
    }

    #[cfg(feature = "docx")]
    #[test]
    fn build_and_export_docx() {
        let doc = DocumentBuilder::new()
            .title("Builder Test")
            .heading(1, "Hello")
            .paragraph(|p| p.text("World"))
            .build();

        let bytes = doc.export(crate::Format::Docx).unwrap();

        // Re-open and verify
        let engine = crate::Engine::new();
        let doc2 = engine.open(&bytes).unwrap();
        assert_eq!(doc2.to_plain_text(), "Hello\nWorld");
        assert_eq!(doc2.metadata().title.as_deref(), Some("Builder Test"));
    }

    #[test]
    fn build_with_superscript() {
        let doc = DocumentBuilder::new()
            .paragraph(|p| p.text("E=mc").superscript("2"))
            .build();
        assert_eq!(doc.to_plain_text(), "E=mc2");

        let body = doc.model().node(doc.model().body_id().unwrap()).unwrap();
        let para = doc.model().node(body.children[0]).unwrap();
        assert_eq!(para.children.len(), 2);

        let sup_run = doc.model().node(para.children[1]).unwrap();
        assert_eq!(
            sup_run
                .attributes
                .get_bool(&s1_model::AttributeKey::Superscript),
            Some(true)
        );
    }

    #[test]
    fn build_with_subscript() {
        let doc = DocumentBuilder::new()
            .paragraph(|p| p.text("H").subscript("2").text("O"))
            .build();
        assert_eq!(doc.to_plain_text(), "H2O");

        let body = doc.model().node(doc.model().body_id().unwrap()).unwrap();
        let para = doc.model().node(body.children[0]).unwrap();
        assert_eq!(para.children.len(), 3);

        let sub_run = doc.model().node(para.children[1]).unwrap();
        assert_eq!(
            sub_run
                .attributes
                .get_bool(&s1_model::AttributeKey::Subscript),
            Some(true)
        );
    }

    #[test]
    fn build_with_hyperlink() {
        let doc = DocumentBuilder::new()
            .paragraph(|p| {
                p.text("Visit ")
                    .hyperlink("https://example.com", "Example")
                    .text(" site")
            })
            .build();
        assert_eq!(doc.to_plain_text(), "Visit Example site");

        let body = doc.model().node(doc.model().body_id().unwrap()).unwrap();
        let para = doc.model().node(body.children[0]).unwrap();
        assert_eq!(para.children.len(), 3);

        let link_run = doc.model().node(para.children[1]).unwrap();
        assert_eq!(
            link_run
                .attributes
                .get_string(&s1_model::AttributeKey::HyperlinkUrl),
            Some("https://example.com")
        );
    }

    #[test]
    fn build_with_bookmark() {
        let doc = DocumentBuilder::new()
            .paragraph(|p| p.bookmark_start("BM1").text("Marked text").bookmark_end())
            .build();
        assert_eq!(doc.to_plain_text(), "Marked text");

        let body = doc.model().node(doc.model().body_id().unwrap()).unwrap();
        let para = doc.model().node(body.children[0]).unwrap();
        // BookmarkStart, Run, BookmarkEnd
        assert_eq!(para.children.len(), 3);

        let bk_start = doc.model().node(para.children[0]).unwrap();
        assert_eq!(bk_start.node_type, s1_model::NodeType::BookmarkStart);
        assert_eq!(
            bk_start
                .attributes
                .get_string(&s1_model::AttributeKey::BookmarkName),
            Some("BM1")
        );

        let bk_end = doc.model().node(para.children[2]).unwrap();
        assert_eq!(bk_end.node_type, s1_model::NodeType::BookmarkEnd);
    }

    #[test]
    fn build_table_of_contents() {
        let doc = DocumentBuilder::new()
            .table_of_contents(3)
            .heading(1, "Introduction")
            .heading(2, "Background")
            .text("Some content")
            .heading(1, "Conclusion")
            .build();

        // TOC node should exist
        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let toc = doc.node(body.children[0]).unwrap();
        assert_eq!(toc.node_type, NodeType::TableOfContents);
        assert_eq!(
            toc.attributes.get_i64(&AttributeKey::TocMaxLevel),
            Some(3)
        );
    }

    #[test]
    fn build_toc_with_title() {
        let doc = DocumentBuilder::new()
            .table_of_contents_with_title(2, "Contents")
            .heading(1, "Chapter 1")
            .build();

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let toc = doc.node(body.children[0]).unwrap();
        assert_eq!(
            toc.attributes.get_string(&AttributeKey::TocTitle),
            Some("Contents")
        );
    }

    #[test]
    fn update_toc_generates_entries() {
        let mut doc = DocumentBuilder::new()
            .table_of_contents(3)
            .heading(1, "First")
            .heading(2, "Second")
            .heading(3, "Third")
            .heading(4, "Fourth") // beyond max_level=3, should be excluded
            .build();

        doc.update_toc();

        let body_id = doc.body_id().unwrap();
        let body = doc.node(body_id).unwrap();
        let toc = doc.node(body.children[0]).unwrap();

        // Should have 3 entries (headings 1-3, not heading 4)
        assert_eq!(toc.children.len(), 3);

        // First entry should be "First"
        let entry1 = doc.node(toc.children[0]).unwrap();
        assert_eq!(entry1.node_type, NodeType::Paragraph);
    }

    #[cfg(feature = "docx")]
    #[test]
    fn build_toc_docx_roundtrip() {
        let mut doc = DocumentBuilder::new()
            .table_of_contents(3)
            .heading(1, "Chapter One")
            .heading(2, "Section A")
            .text("Content here")
            .build();

        doc.update_toc();

        let bytes = doc.export(crate::Format::Docx).unwrap();
        let engine = crate::Engine::new();
        let doc2 = engine.open(&bytes).unwrap();

        // TOC node should survive round-trip
        let body_id = doc2.body_id().unwrap();
        let body = doc2.node(body_id).unwrap();

        // First child should be TOC
        let first = doc2.node(body.children[0]).unwrap();
        assert_eq!(first.node_type, NodeType::TableOfContents);
    }

    #[test]
    fn build_hyperlink_docx_roundtrip() {
        let doc = DocumentBuilder::new()
            .paragraph(|p| p.hyperlink("https://example.com", "Link"))
            .build();

        let bytes = doc.export(crate::Format::Docx).unwrap();

        let engine = crate::Engine::new();
        let doc2 = engine.open(&bytes).unwrap();
        assert_eq!(doc2.to_plain_text(), "Link");

        let body = doc2.model().node(doc2.model().body_id().unwrap()).unwrap();
        let para = doc2.model().node(body.children[0]).unwrap();
        let run = doc2.model().node(para.children[0]).unwrap();
        assert_eq!(
            run.attributes
                .get_string(&s1_model::AttributeKey::HyperlinkUrl),
            Some("https://example.com")
        );
    }
}
