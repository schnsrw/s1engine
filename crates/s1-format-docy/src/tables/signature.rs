use crate::constants::{sig, DOCY_VERSION};
use crate::writer::DocyWriter;

pub fn write(w: &mut DocyWriter) {
    let len_pos = w.begin_length_block();
    w.write_prop_long(sig::VERSION, DOCY_VERSION);
    w.end_length_block(len_pos);
}
