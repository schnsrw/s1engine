//! Low-level binary writer for DOCY format.
//!
//! Implements the TLV (Tag-Length-Value) encoding pattern used throughout
//! the OnlyOffice binary format. All multi-byte integers are little-endian.

pub struct DocyWriter {
    buf: Vec<u8>,
}

impl DocyWriter {
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(64 * 1024),
        }
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn position(&self) -> usize {
        self.buf.len()
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    // ── Primitive writes ────────────────────────────────────────

    pub fn write_byte(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub fn write_bool(&mut self, v: bool) {
        self.buf.push(if v { 1 } else { 0 });
    }

    /// Write 4-byte little-endian unsigned integer.
    pub fn write_long(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// Write 4-byte little-endian signed integer.
    pub fn write_long_signed(&mut self, v: i32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// Write 8-byte little-endian double.
    pub fn write_double(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    /// Write UTF-8 string with 4-byte length prefix.
    pub fn write_string(&mut self, s: &str) {
        let bytes = s.as_bytes();
        self.write_long(bytes.len() as u32);
        self.buf.extend_from_slice(bytes);
    }

    /// Write UTF-8 string without length prefix (used for String2 pattern).
    pub fn write_string_raw(&mut self, s: &str) {
        self.buf.extend_from_slice(s.as_bytes());
    }

    /// Write 3-byte RGB color.
    pub fn write_color_rgb(&mut self, r: u8, g: u8, b: u8) {
        self.buf.push(r);
        self.buf.push(g);
        self.buf.push(b);
    }

    /// Write raw bytes.
    pub fn write_raw(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    // ── TLV Property writes ─────────────────────────────────────

    /// Write a byte property: [type:1][lenType:1=Byte][value:1]
    pub fn write_prop_byte(&mut self, prop_type: u8, value: u8) {
        self.buf.push(prop_type);
        self.buf.push(PROP_LEN_BYTE);
        self.buf.push(value);
    }

    /// Write a bool property: [type:1][lenType:1=Byte][value:1]
    pub fn write_prop_bool(&mut self, prop_type: u8, value: bool) {
        self.write_prop_byte(prop_type, if value { 1 } else { 0 });
    }

    /// Write a long property: [type:1][lenType:1=Long][value:4]
    pub fn write_prop_long(&mut self, prop_type: u8, value: u32) {
        self.buf.push(prop_type);
        self.buf.push(PROP_LEN_LONG);
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Write a signed long property.
    pub fn write_prop_long_signed(&mut self, prop_type: u8, value: i32) {
        self.buf.push(prop_type);
        self.buf.push(PROP_LEN_LONG);
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Write a double property: [type:1][lenType:1=Double][value:8]
    pub fn write_prop_double(&mut self, prop_type: u8, value: f64) {
        self.buf.push(prop_type);
        self.buf.push(PROP_LEN_DOUBLE);
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Write a string property (variable length): [type:1][data written by closure]
    /// This is the "String2" pattern — type byte then raw string.
    pub fn write_prop_string2(&mut self, prop_type: u8, value: &str) {
        self.buf.push(prop_type);
        self.write_string(value);
    }

    /// Begin a variable-length item: [type:1][lenType:Variable][length:4][...content...]
    /// Returns the position where length should be patched.
    pub fn begin_item(&mut self, prop_type: u8) -> usize {
        self.buf.push(prop_type);
        self.buf.push(PROP_LEN_VARIABLE);
        let len_pos = self.buf.len();
        self.write_long(0); // placeholder for length
        len_pos
    }

    /// End a variable-length item by patching the length at the given position.
    pub fn end_item(&mut self, len_pos: usize) {
        let content_len = (self.buf.len() - len_pos - 4) as u32;
        let bytes = content_len.to_le_bytes();
        self.buf[len_pos] = bytes[0];
        self.buf[len_pos + 1] = bytes[1];
        self.buf[len_pos + 2] = bytes[2];
        self.buf[len_pos + 3] = bytes[3];
    }

    /// Write a complete item with content from a closure.
    pub fn write_item<F: FnOnce(&mut DocyWriter)>(&mut self, prop_type: u8, f: F) {
        let len_pos = self.begin_item(prop_type);
        f(self);
        self.end_item(len_pos);
    }

    /// Begin a length-prefixed block (no type byte): [length:4][...content...]
    /// Returns the position where length should be patched.
    pub fn begin_length_block(&mut self) -> usize {
        let len_pos = self.buf.len();
        self.write_long(0); // placeholder
        len_pos
    }

    /// End a length-prefixed block.
    pub fn end_length_block(&mut self, len_pos: usize) {
        let content_len = (self.buf.len() - len_pos - 4) as u32;
        let bytes = content_len.to_le_bytes();
        self.buf[len_pos] = bytes[0];
        self.buf[len_pos + 1] = bytes[1];
        self.buf[len_pos + 2] = bytes[2];
        self.buf[len_pos + 3] = bytes[3];
    }

    // ── Main Table ──────────────────────────────────────────────

    /// Write the main table header. Returns (count_pos, items_start).
    /// After all tables are written, call `patch_table_count`.
    pub fn begin_main_table(&mut self) -> MainTableState {
        let count_pos = self.buf.len();
        self.write_byte(0); // placeholder for table count
        // Reserve space for max 128 table items (5 bytes each)
        let items_start = self.buf.len();
        for _ in 0..(MAX_TABLES as usize) * 5 {
            self.buf.push(0);
        }
        MainTableState {
            count_pos,
            items_start,
            count: 0,
        }
    }

    /// Register a table in the main table header.
    pub fn register_table(&mut self, state: &mut MainTableState, table_type: u8) {
        let offset = self.buf.len() as u32;
        let item_pos = state.items_start + (state.count as usize) * 5;
        self.buf[item_pos] = table_type;
        let offset_bytes = offset.to_le_bytes();
        self.buf[item_pos + 1] = offset_bytes[0];
        self.buf[item_pos + 2] = offset_bytes[1];
        self.buf[item_pos + 3] = offset_bytes[2];
        self.buf[item_pos + 4] = offset_bytes[3];
        state.count += 1;
    }

    /// Finalize the main table — patch count and trim unused item slots.
    pub fn end_main_table(&mut self, state: &MainTableState) {
        self.buf[state.count_pos] = state.count;
        // We can't easily trim the reserved space since data follows it.
        // The reserved unused slots contain zeros which the reader skips.
    }
}

pub struct MainTableState {
    pub count_pos: usize,
    pub items_start: usize,
    pub count: u8,
}

// PropLenType constants (from Serialize2.js c_oSerPropLenType)
// Null=0, Byte=1, Short=2, Three=3, Long=4, Double=5, Variable=6
const PROP_LEN_BYTE: u8 = 1;
const _PROP_LEN_SHORT: u8 = 2;
const PROP_LEN_LONG: u8 = 4;
const PROP_LEN_DOUBLE: u8 = 5;
const PROP_LEN_VARIABLE: u8 = 6;

const MAX_TABLES: u8 = 128;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_byte() {
        let mut w = DocyWriter::new();
        w.write_byte(42);
        assert_eq!(w.as_bytes(), &[42]);
    }

    #[test]
    fn write_long_le() {
        let mut w = DocyWriter::new();
        w.write_long(0x01020304);
        assert_eq!(w.as_bytes(), &[4, 3, 2, 1]); // little-endian
    }

    #[test]
    fn write_string_with_length() {
        let mut w = DocyWriter::new();
        w.write_string("Hi");
        assert_eq!(w.as_bytes(), &[2, 0, 0, 0, b'H', b'i']);
    }

    #[test]
    fn write_prop_bool() {
        let mut w = DocyWriter::new();
        w.write_prop_bool(7, true);
        assert_eq!(w.as_bytes(), &[7, 1, 1]); // type=7, lenType=Byte(1), value=1
    }

    #[test]
    fn write_item_with_content() {
        let mut w = DocyWriter::new();
        w.write_item(5, |w| {
            w.write_byte(0xFF);
            w.write_byte(0xAA);
        });
        // [type:5][lenType:Variable(6)][length:2,0,0,0][0xFF][0xAA]
        assert_eq!(w.as_bytes(), &[5, 6, 2, 0, 0, 0, 0xFF, 0xAA]);
    }

    #[test]
    fn main_table_state() {
        let mut w = DocyWriter::new();
        let mut state = w.begin_main_table();
        // Write a dummy table
        w.register_table(&mut state, 0); // Signature at current offset
        w.write_byte(42); // table data
        w.register_table(&mut state, 6); // Document at current offset
        w.write_byte(99); // table data
        w.end_main_table(&state);
        assert_eq!(state.count, 2);
        assert_eq!(w.as_bytes()[0], 2); // table count
    }
}
