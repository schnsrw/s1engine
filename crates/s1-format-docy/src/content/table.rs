use crate::constants::*;
use crate::writer::DocyWriter;
use crate::content::paragraph;
use s1_model::{DocumentModel, NodeType, NodeId, AttributeKey, AttributeValue};

pub fn write(w: &mut DocyWriter, model: &DocumentModel, table_id: NodeId) {
    let table = match model.node(table_id) {
        Some(n) => n,
        None => return,
    };

    // Table properties
    w.write_item(tbl_pr::ROWS, |w| {
        // Count rows for the Rows property
        let row_count = table.children.iter()
            .filter(|id| model.node(**id).map_or(false, |n| n.node_type == NodeType::TableRow))
            .count();
        w.write_long(row_count as u32);
    });

    // Table width
    if let Some(AttributeValue::TableWidth(tw)) = table.attributes.get(&AttributeKey::TableWidth) {
        w.write_item(tbl_pr::TABLE_W, |w| {
            match tw {
                s1_model::TableWidth::Auto => {
                    w.write_byte(0); // auto
                }
                s1_model::TableWidth::Fixed(v) => {
                    w.write_byte(1); // fixed
                    w.write_long(pts_to_twips(*v) as u32);
                }
                s1_model::TableWidth::Percent(v) => {
                    w.write_byte(2); // percent
                    w.write_long((*v * 50.0) as u32); // percent × 50
                }
                &_ => w.write_byte(0),
            }
        });
    }

    // Table alignment
    if let Some(AttributeValue::Alignment(a)) = table.attributes.get(&AttributeKey::TableAlignment) {
        let val = match a {
            s1_model::Alignment::Left => align::LEFT,
            s1_model::Alignment::Center => align::CENTER,
            s1_model::Alignment::Right => align::RIGHT,
            _ => align::LEFT,
        };
        w.write_prop_byte(tbl_pr::JC, val);
    }

    // Table rows
    for row_id in &table.children {
        let row = match model.node(*row_id) {
            Some(n) if n.node_type == NodeType::TableRow => n,
            _ => continue,
        };

        w.write_item(par::PAR, |w| { // Row uses PAR type in table context
            // Row properties
            if let Some(true) = row.attributes.get_bool(&AttributeKey::TableHeaderRow) {
                w.write_prop_bool(row_pr::TABLE_HEADER, true);
            }
            if let Some(h) = row.attributes.get_f64(&AttributeKey::RowHeight) {
                w.write_prop_long(row_pr::HEIGHT, pts_to_twips(h) as u32);
            }

            // Cells
            for cell_id in &row.children {
                let cell = match model.node(*cell_id) {
                    Some(n) if n.node_type == NodeType::TableCell => n,
                    _ => continue,
                };

                w.write_item(par::TABLE, |w| { // Cell uses TABLE type in row context
                    // Cell properties
                    write_cell_props(w, cell);

                    // Cell content (paragraphs, nested tables)
                    for content_id in &cell.children {
                        let content = match model.node(*content_id) {
                            Some(n) => n,
                            None => continue,
                        };
                        match content.node_type {
                            NodeType::Paragraph => {
                                w.write_item(par::PAR, |w| {
                                    paragraph::write(w, model, *content_id);
                                });
                            }
                            NodeType::Table => {
                                w.write_item(par::TABLE, |w| {
                                    write(w, model, *content_id); // recursive for nested tables
                                });
                            }
                            _ => {}
                        }
                    }
                });
            }
        });
    }
}

fn write_cell_props(w: &mut DocyWriter, cell: &s1_model::Node) {
    // Grid span (column merge)
    if let Some(AttributeValue::Int(span)) = cell.attributes.get(&AttributeKey::ColSpan) {
        if *span > 1 {
            w.write_prop_long(cell_pr::GRID_SPAN, *span as u32);
        }
    }

    // Vertical merge
    if let Some(merge) = cell.attributes.get_string(&AttributeKey::RowSpan) {
        let val = match merge {
            "restart" => 1,
            "continue" => 2,
            _ => 0,
        };
        if val > 0 {
            w.write_prop_byte(cell_pr::VMERGE, val);
        }
    }

    // Cell width
    if let Some(AttributeValue::TableWidth(tw)) = cell.attributes.get(&AttributeKey::CellWidth) {
        w.write_item(cell_pr::CELL_W, |w| {
            match tw {
                s1_model::TableWidth::Auto => w.write_byte(0),
                s1_model::TableWidth::Fixed(v) => {
                    w.write_byte(1);
                    w.write_long(pts_to_twips(*v) as u32);
                }
                s1_model::TableWidth::Percent(v) => {
                    w.write_byte(2);
                    w.write_long((*v * 50.0) as u32);
                }
                &_ => w.write_byte(0),
            }
        });
    }

    // Vertical alignment
    if let Some(AttributeValue::VerticalAlignment(va)) = cell.attributes.get(&AttributeKey::VerticalAlign) {
        let val = match va {
            s1_model::VerticalAlignment::Top => 0,
            s1_model::VerticalAlignment::Center => 1,
            s1_model::VerticalAlignment::Bottom => 2, _ => 0,
        };
        w.write_prop_byte(cell_pr::VALIGN, val);
    }

    // Cell background
    if let Some(AttributeValue::Color(c)) = cell.attributes.get(&AttributeKey::CellBackground) {
        w.write_item(cell_pr::SHD, |w| {
            w.write_byte(color::RGB);
            w.write_color_rgb(c.r, c.g, c.b);
        });
    }
}
