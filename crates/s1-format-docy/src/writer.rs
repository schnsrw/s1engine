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

    /// Write string in DOCY format: 4-byte byte length + UTF-16LE characters.
    /// This matches sdkjs `CMemory.WriteString2`.
    pub fn write_string(&mut self, s: &str) {
        let utf16: Vec<u16> = s.encode_utf16().collect();
        self.write_long((utf16.len() * 2) as u32);
        for ch in &utf16 {
            self.buf.extend_from_slice(&ch.to_le_bytes());
        }
    }

    /// Write raw UTF-8 string without any encoding (for internal use only).
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

    /// Write a string item for Read1-style blocks: [type:1][utf16_byte_len:4][utf16le:N]
    pub fn write_string_item(&mut self, prop_type: u8, value: &str) {
        self.buf.push(prop_type);
        self.write_string(value);
    }

    /// Write a string property for Read2-style property blocks:
    /// [type:1][lenType:1=Variable][byte_len:4][utf16le:N]
    /// Note: This does NOT use write_string() because PROP_LEN_VARIABLE
    /// expects the BYTE length of the payload, whereas WriteString2 (Read1)
    /// expects the CHARACTER length.
    pub fn write_prop_string2(&mut self, prop_type: u8, value: &str) {
        self.write_prop_item(prop_type, |w| {
            let utf16: Vec<u16> = value.encode_utf16().collect();
            for ch in &utf16 {
                w.buf.extend_from_slice(&ch.to_le_bytes());
            }
        });
    }

    /// Write a variable-length property for Read2-style property blocks:
    /// [type:1][lenType:1=Variable][length:4][content:N]
    pub fn write_prop_item<F: FnOnce(&mut DocyWriter)>(&mut self, prop_type: u8, f: F) {
        self.buf.push(prop_type);
        self.buf.push(PROP_LEN_VARIABLE);
        let len_pos = self.buf.len();
        self.write_long(0); // placeholder for variable payload length
        f(self);
        let content_len = (self.buf.len() - len_pos - 4) as u32;
        let bytes = content_len.to_le_bytes();
        self.buf[len_pos] = bytes[0];
        self.buf[len_pos + 1] = bytes[1];
        self.buf[len_pos + 2] = bytes[2];
        self.buf[len_pos + 3] = bytes[3];
    }

    /// Begin a complex item: [type:1][length:4][...content...]
    /// NO lenType byte — matches sdkjs WriteItem pattern.
    /// Returns the position where length should be patched.
    pub fn begin_item(&mut self, prop_type: u8) -> usize {
        self.buf.push(prop_type);
        let len_pos = self.buf.len();
        self.write_long(0); // placeholder for length
        len_pos
    }

    /// End an item by patching the length at the given position.
    pub fn end_item(&mut self, len_pos: usize) {
        let content_len = (self.buf.len() - len_pos - 4) as u32;
        let bytes = content_len.to_le_bytes();
        self.buf[len_pos] = bytes[0];
        self.buf[len_pos + 1] = bytes[1];
        self.buf[len_pos + 2] = bytes[2];
        self.buf[len_pos + 3] = bytes[3];
    }

    /// Write a complex item with content from a closure.
    /// Pattern: [type:1][length:4][content:N]
    /// This matches sdkjs WriteItem (NOT property writes which have lenType).
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

    /// Write the main table header.
    /// Pattern: [table_count:u8][type:u8 + offset:u32 × count]
    pub fn begin_main_table(&mut self, table_count: u8) -> MainTableState {
        let count_pos = self.buf.len();
        self.write_byte(table_count);
        let items_start = self.buf.len();
        // Reserve space for table items (5 bytes each)
        for _ in 0..(table_count as usize) * 5 {
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

    /// Finalize the main table — verify count matches.
    pub fn end_main_table(&mut self, state: &MainTableState) {
        assert_eq!(
            self.buf[state.count_pos], state.count,
            "Table count mismatch: expected {}, registered {}",
            self.buf[state.count_pos], state.count
        );
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
    fn write_string_utf16le() {
        let mut w = DocyWriter::new();
        w.write_string("Hi");
        // 2 chars (UTF-16) = 4 bytes + UTF-16LE bytes: H=0x0048, i=0x0069
        assert_eq!(w.as_bytes(), &[4, 0, 0, 0, 0x48, 0x00, 0x69, 0x00]);
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
        // [type:5][length:2,0,0,0][0xFF][0xAA] — NO lenType byte
        assert_eq!(w.as_bytes(), &[5, 2, 0, 0, 0, 0xFF, 0xAA]);
    }

    #[test]
    fn main_table_state() {
        let mut w = DocyWriter::new();
        let mut state = w.begin_main_table(2);
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
