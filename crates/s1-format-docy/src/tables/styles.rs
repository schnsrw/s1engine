use crate::constants::*;
use crate::writer::DocyWriter;
use crate::props;
use s1_model::DocumentModel;

pub fn write(w: &mut DocyWriter, model: &DocumentModel) {
    let len_pos = w.begin_length_block();

    // Default paragraph properties
    w.write_item(style_table::DEF_PPR, |w| {
        let defaults = model.doc_defaults();
        props::para_props::write_defaults(w, defaults);
    });

    // Default run properties
    w.write_item(style_table::DEF_RPR, |w| {
        let defaults = model.doc_defaults();
        props::run_props::write_defaults(w, defaults);
    });

    // All styles
    w.write_item(style_table::STYLES, |w| {
        for s in model.styles() {
            write_style(w, s);
        }
    });

    w.end_length_block(len_pos);
}

fn write_style(w: &mut DocyWriter, s: &s1_model::Style) {
    w.write_item(style::STYLE, |w| {
        // Style ID
        w.write_string_item(style::STYLE_ID, &s.id);

        // Style name
        w.write_string_item(style::STYLE_NAME, &s.name);

        // Type (1=Char, 2=Num, 3=Para, 4=Tbl)
        let type_byte = match s.style_type {
            s1_model::StyleType::Character => 1,
            s1_model::StyleType::List => 2,
            s1_model::StyleType::Paragraph => 3,
            s1_model::StyleType::Table => 4, _ => 3,
        };
        w.write_prop_byte(style::STYLE_TYPE, type_byte);

        // Based on
        if let Some(ref parent) = s.parent_id {
            w.write_string_item(style::STYLE_BASED_ON, parent);
        }

        // Next style
        if let Some(ref next) = s.next_style_id {
            w.write_string_item(style::STYLE_NEXT, next);
        }

        // Default flag
        if s.is_default {
            w.write_prop_bool(style::STYLE_DEFAULT, true);
        }

        // Paragraph properties from style attributes
        w.write_item(style::STYLE_PARA_PR, |w| {
            props::para_props::write(w, &s.attributes);
        });

        // Run/text properties from style attributes
        w.write_item(style::STYLE_TEXT_PR, |w| {
            props::run_props::write(w, &s.attributes);
        });
    });
}
