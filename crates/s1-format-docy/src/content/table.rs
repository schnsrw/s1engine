use crate::constants::*;
use crate::writer::DocyWriter;
use crate::content::paragraph;
use s1_model::{DocumentModel, NodeType, NodeId, AttributeKey, AttributeValue};

/// Table type IDs matching c_oSerDocTableType in Serialize2.js:480
mod tbl {
    pub const TBL_PR: u8 = 0;
    pub const TBL_GRID: u8 = 1;
    pub const TBL_GRID_ITEM_TWIPS: u8 = 13; // tblGrid_ItemTwips
    pub const CONTENT: u8 = 3;
    pub const ROW: u8 = 4;
    pub const ROW_PR: u8 = 4;
    pub const ROW_CONTENT: u8 = 5;
    pub const CELL: u8 = 6;
    pub const CELL_PR: u8 = 7;
    pub const CELL_CONTENT: u8 = 8;
}

pub fn write(w: &mut DocyWriter, model: &DocumentModel, table_id: NodeId) {
    let table = match model.node(table_id) {
        Some(n) => n,
        None => return,
    };

    // Collect rows
    let rows: Vec<NodeId> = table.children.iter()
        .filter(|id| model.node(**id).map_or(false, |n| n.node_type == NodeType::TableRow))
        .copied()
        .collect();

    if rows.is_empty() { return; }

    // Count max columns for grid
    let max_cols = rows.iter().map(|rid| {
        model.node(*rid).map_or(0, |r| {
            r.children.iter().filter(|id| {
                model.node(**id).map_or(false, |n| n.node_type == NodeType::TableCell)
            }).count()
        })
    }).max().unwrap_or(0);

    // tblPr (table properties)
    w.write_item(tbl::TBL_PR, |w| {
        write_table_props(w, table);
    });

    // tblGrid (column widths in twips)
    w.write_item(tbl::TBL_GRID, |w| {
        // Default column width: page width / cols (approx 4680 twips for letter)
        let default_col_width = 4680u32 / (max_cols.max(1) as u32);
        for _ in 0..max_cols {
            w.write_prop_long(tbl::TBL_GRID_ITEM_TWIPS, default_col_width);
        }
    });

    // Content (rows)
    w.write_item(tbl::CONTENT, |w| {
        for row_id in &rows {
            w.write_item(tbl::ROW, |w| {
                write_row(w, model, *row_id);
            });
        }
    });
}

fn write_table_props(w: &mut DocyWriter, table: &s1_model::Node) {
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

    // Table width — uses WriteW format: type=7(TABLE_W), Variable, [Type byte + value long]
    if let Some(AttributeValue::TableWidth(tw)) = table.attributes.get(&AttributeKey::TableWidth) {
        w.write_prop_item(tbl_pr::TABLE_W, |w| {
            match tw {
                s1_model::TableWidth::Auto => {
                    w.write_byte(0); // type: auto
                    w.write_long(0);
                }
                s1_model::TableWidth::Fixed(v) => {
                    w.write_byte(3); // type: dxa (twips)
                    w.write_long(pts_to_twips(*v) as u32);
                }
                s1_model::TableWidth::Percent(v) => {
                    w.write_byte(1); // type: pct
                    w.write_long((*v * 50.0) as u32);
                }
                _ => {
                    w.write_byte(0);
                    w.write_long(0);
                }
            }
        });
    }
}

fn write_row(w: &mut DocyWriter, model: &DocumentModel, row_id: NodeId) {
    let row = match model.node(row_id) {
        Some(n) => n,
        None => return,
    };

    // Row_Pr (row properties)
    w.write_item(tbl::ROW_PR, |w| {
        if let Some(true) = row.attributes.get_bool(&AttributeKey::TableHeaderRow) {
            w.write_prop_bool(row_pr::TABLE_HEADER, true);
        }
        if let Some(h) = row.attributes.get_f64(&AttributeKey::RowHeight) {
            w.write_prop_item(row_pr::HEIGHT, |w| {
                w.write_prop_byte(row_pr::HEIGHT_RULE, 1);
                w.write_prop_long(row_pr::HEIGHT_VALUE_TWIPS, pts_to_twips(h) as u32);
            });
        }
    });

    // Row_Content (cells)
    let cells: Vec<NodeId> = row.children.iter()
        .filter(|id| model.node(**id).map_or(false, |n| n.node_type == NodeType::TableCell))
        .copied()
        .collect();

    w.write_item(tbl::ROW_CONTENT, |w| {
        for cell_id in &cells {
            w.write_item(tbl::CELL, |w| {
                write_cell(w, model, *cell_id);
            });
        }
    });
}

fn write_cell(w: &mut DocyWriter, model: &DocumentModel, cell_id: NodeId) {
    let cell = match model.node(cell_id) {
        Some(n) => n,
        None => return,
    };

    // Cell_Pr (cell properties)
    w.write_item(tbl::CELL_PR, |w| {
        // Grid span
        if let Some(AttributeValue::Int(span)) = cell.attributes.get(&AttributeKey::ColSpan) {
            if *span > 1 {
                w.write_prop_long(cell_pr::GRID_SPAN, *span as u32);
            }
        }

        // Vertical merge
        if let Some(merge) = cell.attributes.get_string(&AttributeKey::RowSpan) {
            let val = match merge {
                "restart" => 1u8,
                "continue" => 2,
                _ => 0,
            };
            if val > 0 {
                w.write_prop_byte(cell_pr::VMERGE, val);
            }
        }

        // Cell width
        if let Some(AttributeValue::TableWidth(tw)) = cell.attributes.get(&AttributeKey::CellWidth) {
            w.write_prop_item(cell_pr::CELL_W, |w| {
                match tw {
                    s1_model::TableWidth::Auto => { w.write_byte(0); w.write_long(0); }
                    s1_model::TableWidth::Fixed(v) => { w.write_byte(3); w.write_long(pts_to_twips(*v) as u32); }
                    s1_model::TableWidth::Percent(v) => { w.write_byte(1); w.write_long((*v * 50.0) as u32); }
                    _ => { w.write_byte(0); w.write_long(0); }
                }
            });
        }

        // Vertical alignment
        if let Some(AttributeValue::VerticalAlignment(va)) = cell.attributes.get(&AttributeKey::VerticalAlign) {
            let val = match va {
                s1_model::VerticalAlignment::Top => 0u8,
                s1_model::VerticalAlignment::Center => 1,
                s1_model::VerticalAlignment::Bottom => 2,
                _ => 0,
            };
            w.write_prop_byte(cell_pr::VALIGN, val);
        }

        // Cell background
        if let Some(AttributeValue::Color(c)) = cell.attributes.get(&AttributeKey::CellBackground) {
            w.write_prop_item(cell_pr::SHD, |w| {
                w.write_byte(color::RGB);
                w.write_color_rgb(c.r, c.g, c.b);
            });
        }
    });

    // Cell_Content (paragraphs, nested tables)
    w.write_item(tbl::CELL_CONTENT, |w| {
        for child_id in &cell.children {
            let child = match model.node(*child_id) {
                Some(n) => n,
                None => continue,
            };
            match child.node_type {
                NodeType::Paragraph => {
                    w.write_item(par::PAR, |w| {
                        paragraph::write(w, model, *child_id);
                    });
                }
                NodeType::Table => {
                    w.write_item(par::TABLE, |w| {
                        write(w, model, *child_id);
                    });
                }
                _ => {}
            }
        }
    });
}
