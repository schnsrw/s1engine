use crate::constants::settings;
use crate::writer::DocyWriter;
use s1_model::DocumentModel;

pub fn write(w: &mut DocyWriter, model: &DocumentModel) {
    let len_pos = w.begin_length_block();

    // Default tab stop (720 twips = 36pt = 0.5 inch)
    w.write_prop_long(settings::DEFAULT_TAB_STOP_TWIPS, 720);

    // Track revisions
    if model.metadata().track_changes {
        w.write_prop_bool(settings::TRACK_REVISIONS, true);
    }

    w.end_length_block(len_pos);
}
