use crate::constants::*;
use crate::writer::DocyWriter;
use s1_model::{PageOrientation, SectionBreakType, SectionProperties};

pub fn write(w: &mut DocyWriter, sec: &SectionProperties) {
    w.write_item(sec_pr::PG_SZ, |w| write_page_size(w, sec));
    w.write_item(sec_pr::PG_MAR, |w| write_page_margins(w, sec));
    w.write_item(sec_pr::SETTINGS, |w| write_settings(w, sec));

    if sec.columns > 1 || !sec.equal_width {
        w.write_item(sec_pr::COLS, |w| write_columns(w, sec));
    }
}

fn write_page_size(w: &mut DocyWriter, sec: &SectionProperties) {
    w.write_prop_long(sec_pg_sz::W_TWIPS, pts_to_twips(sec.page_width) as u32);
    w.write_prop_long(sec_pg_sz::H_TWIPS, pts_to_twips(sec.page_height) as u32);

    let orient = match sec.orientation {
        PageOrientation::Portrait => 0,
        PageOrientation::Landscape => 1,
        _ => 0,
    };
    w.write_prop_byte(sec_pg_sz::ORIENTATION, orient);
}

fn write_page_margins(w: &mut DocyWriter, sec: &SectionProperties) {
    w.write_prop_long(sec_pg_mar::LEFT_TWIPS, pts_to_twips(sec.margin_left) as u32);
    w.write_prop_long(sec_pg_mar::TOP_TWIPS, pts_to_twips(sec.margin_top) as u32);
    w.write_prop_long(sec_pg_mar::RIGHT_TWIPS, pts_to_twips(sec.margin_right) as u32);
    w.write_prop_long(sec_pg_mar::BOTTOM_TWIPS, pts_to_twips(sec.margin_bottom) as u32);
    w.write_prop_long(sec_pg_mar::HEADER_TWIPS, pts_to_twips(sec.header_distance) as u32);
    w.write_prop_long(sec_pg_mar::FOOTER_TWIPS, pts_to_twips(sec.footer_distance) as u32);
    w.write_prop_long(sec_pg_mar::GUTTER_TWIPS, 0);
}

fn write_settings(w: &mut DocyWriter, sec: &SectionProperties) {
    if sec.title_page {
        w.write_prop_bool(sec_settings::TITLE_PG, true);
    }
    if sec.even_and_odd_headers {
        w.write_prop_bool(sec_settings::EVEN_AND_ODD_HEADERS, true);
    }
    if let Some(section_type) = section_type_byte(sec.break_type) {
        w.write_prop_byte(sec_settings::SECTION_TYPE, section_type);
    }
}

fn write_columns(w: &mut DocyWriter, sec: &SectionProperties) {
    w.write_item(sec_columns::EQUAL_WIDTH, |w| w.write_bool(sec.equal_width));
    w.write_item(sec_columns::NUM, |w| w.write_long(sec.columns));
    w.write_item(sec_columns::SPACE, |w| {
        w.write_long(pts_to_twips(sec.column_spacing) as u32);
    });
}

fn section_type_byte(break_type: Option<SectionBreakType>) -> Option<u8> {
    match break_type {
        Some(SectionBreakType::Continuous) => Some(0),
        Some(SectionBreakType::EvenPage) => Some(1),
        Some(SectionBreakType::NextPage) => Some(3),
        Some(SectionBreakType::OddPage) => Some(4),
        Some(_) | None => None,
    }
}
