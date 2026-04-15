use crate::constants::*;
use crate::writer::DocyWriter;
use s1_model::SectionProperties;

pub fn write(w: &mut DocyWriter, sec: &SectionProperties) {
    // Page size (points → twips)
    w.write_prop_long(sec_pr::PG_SZ_W, pts_to_twips(sec.page_width) as u32);
    w.write_prop_long(sec_pr::PG_SZ_H, pts_to_twips(sec.page_height) as u32);

    // Orientation
    let orient = match sec.orientation {
        s1_model::PageOrientation::Portrait => 0,
        s1_model::PageOrientation::Landscape => 1, _ => 0,
    };
    w.write_prop_byte(sec_pr::PG_SZ_ORIENT, orient);

    // Margins (points → twips)
    w.write_prop_long(sec_pr::PG_MAR_TOP, pts_to_twips(sec.margin_top) as u32);
    w.write_prop_long(sec_pr::PG_MAR_BOTTOM, pts_to_twips(sec.margin_bottom) as u32);
    w.write_prop_long(sec_pr::PG_MAR_LEFT, pts_to_twips(sec.margin_left) as u32);
    w.write_prop_long(sec_pr::PG_MAR_RIGHT, pts_to_twips(sec.margin_right) as u32);

    // Header/footer distance
    w.write_prop_long(sec_pr::PG_MAR_HEADER, pts_to_twips(sec.header_distance) as u32);
    w.write_prop_long(sec_pr::PG_MAR_FOOTER, pts_to_twips(sec.footer_distance) as u32);

    // Columns
    if sec.columns > 1 {
        w.write_item(sec_pr::COLS, |w| {
            w.write_byte(sec.columns as u8);
            w.write_long(pts_to_twips(sec.column_spacing) as u32);
        });
    }

    // Title page (different first page header/footer)
    if sec.title_page {
        w.write_prop_bool(sec_pr::TITLE_PG, true);
    }
}
