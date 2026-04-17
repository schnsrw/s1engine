use crate::constants::num;
use crate::writer::DocyWriter;
use s1_model::DocumentModel;

pub fn has_content(model: &DocumentModel) -> bool {
    !model.numbering().abstract_nums.is_empty()
}

pub fn write(w: &mut DocyWriter, model: &DocumentModel) {
    let len_pos = w.begin_length_block();
    let numbering = model.numbering();

    // Abstract numbering definitions
    w.write_item(num::ABSTRACT_NUMS, |w| {
        for abs_num in &numbering.abstract_nums {
            w.write_item(num::ABSTRACT_NUM, |w| {
                w.write_prop_long(num::ABSTRACT_NUM_ID, abs_num.abstract_num_id);

                // Levels
                w.write_item(num::ABSTRACT_NUM_LVLS, |w| {
                    for level in &abs_num.levels {
                        w.write_item(num::LVL, |w| {
                            // Number format
                            let fmt = match level.num_format {
                                s1_model::ListFormat::Bullet => 23,   // bullet
                                s1_model::ListFormat::Decimal => 0,   // decimal
                                s1_model::ListFormat::LowerAlpha => 4,
                                s1_model::ListFormat::UpperAlpha => 3,
                                s1_model::ListFormat::LowerRoman => 2,
                                s1_model::ListFormat::UpperRoman => 1,
                                _ => 0,
                            };
                            w.write_prop_long(num::LVL_FORMAT, fmt);
                            w.write_prop_long(num::LVL_START, level.start);

                            // Level text
                            w.write_item(num::LVL_TEXT, |w| {
                                w.write_item(num::LVL_TEXT_ITEM, |w| {
                                    w.write_string_item(num::LVL_TEXT_ITEM_TEXT, &level.level_text);
                                });
                            });

                            // Level paragraph properties (indentation)
                            if let Some(indent) = level.indent_left {
                                w.write_item(num::LVL_PARA_PR, |w| {
                                    let twips = crate::constants::pts_to_twips(indent);
                                    w.write_prop_long_signed(
                                        crate::constants::ppr::IND_LEFT_TWIPS,
                                        twips,
                                    );
                                    if let Some(hanging) = level.indent_hanging {
                                        let h_twips = crate::constants::pts_to_twips(hanging);
                                        // Hanging indent as negative first-line
                                        w.write_prop_long_signed(
                                            crate::constants::ppr::IND_FIRST_LINE_TWIPS,
                                            -h_twips,
                                        );
                                    }
                                });
                            }
                        });
                    }
                });
            });
        }
    });

    // Numbering instances
    w.write_item(num::NUMS, |w| {
        for inst in &numbering.instances {
            w.write_item(num::NUM, |w| {
                w.write_prop_long(num::NUM_ANUM_ID, inst.abstract_num_id);
                w.write_prop_long(num::NUM_NUM_ID, inst.num_id);
            });
        }
    });

    w.end_length_block(len_pos);
}
