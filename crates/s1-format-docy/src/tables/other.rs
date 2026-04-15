use crate::writer::DocyWriter;

pub fn write(w: &mut DocyWriter) {
    // Empty "Other" table — theme is optional
    let len_pos = w.begin_length_block();
    w.end_length_block(len_pos);
}
