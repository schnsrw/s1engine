//! Piece table parsing for DOC binary format.
//!
//! The piece table (PlcPcd) maps character positions to byte offsets in the
//! WordDocument stream. It is stored inside the Clx structure in the Table
//! stream, as located by `fcClx`/`lcbClx` from the FIB.
//!
//! The Clx consists of:
//! - Zero or more Prc entries (type byte = 0x01), each with a 2-byte length
//! - One Pcdt entry (type byte = 0x02) containing the PlcPcd
//!
//! The PlcPcd is an array of CPs (character positions, u32 LE) followed by
//! Pcd structures (8 bytes each). For n pieces, there are (n+1) CPs and n Pcds.
//!
//! Reference: MS-DOC specification, Sections 2.8.35 (Clx), 2.8.36 (PlcPcd)

use crate::error::ConvertError;

/// A parsed piece table from the Clx structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PieceTable {
    /// The individual pieces mapping character positions to byte offsets.
    pub pieces: Vec<Piece>,
}

/// A single piece in the piece table.
///
/// Maps a range of character positions (`cp_start..cp_end`) to a byte offset
/// in the WordDocument stream. The `is_ansi` flag indicates the text encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    /// Character position start (inclusive).
    pub cp_start: u32,
    /// Character position end (exclusive).
    pub cp_end: u32,
    /// Raw fc value from the Pcd (includes encoding bit).
    pub fc: u32,
    /// If true, the text is ANSI (CP1252/Windows-1252). Otherwise UTF-16LE.
    pub is_ansi: bool,
}

/// Size of a Pcd structure in bytes.
const PCD_SIZE: usize = 8;

/// Bit 30 of fc: when set, text is ANSI-encoded.
const FC_ANSI_BIT: u32 = 1 << 30;

/// Mask to extract the actual file offset from the fc field.
const FC_OFFSET_MASK: u32 = 0x3FFF_FFFF;

impl PieceTable {
    /// Parse the Clx data to extract the piece table.
    ///
    /// The Clx data is read from the Table stream at the offset specified
    /// by `fcClx` in the FIB, with length `lcbClx`.
    ///
    /// # Errors
    ///
    /// Returns `ConvertError::InvalidDoc` if:
    /// - The Clx data is empty or malformed
    /// - No Pcdt (type 0x02) is found
    /// - The PlcPcd structure is invalid
    pub fn parse(clx_data: &[u8]) -> Result<Self, ConvertError> {
        if clx_data.is_empty() {
            return Err(ConvertError::InvalidDoc("Clx data is empty".into()));
        }

        let mut offset = 0;

        // Skip Prc entries (type 0x01)
        while offset < clx_data.len() {
            let type_byte = clx_data[offset];

            if type_byte == 0x01 {
                // Prc: type byte (1) + cbGrpprl (2 bytes LE) + grpprl data
                if offset + 3 > clx_data.len() {
                    return Err(ConvertError::InvalidDoc(
                        "Prc entry truncated: missing cbGrpprl".into(),
                    ));
                }
                let cb = u16::from_le_bytes([clx_data[offset + 1], clx_data[offset + 2]]) as usize;
                offset += 3 + cb; // Skip type + cbGrpprl + grpprl data
            } else if type_byte == 0x02 {
                // Pcdt found — parse it
                break;
            } else {
                return Err(ConvertError::InvalidDoc(format!(
                    "unexpected Clx entry type: 0x{type_byte:02X} at offset {offset}"
                )));
            }
        }

        if offset >= clx_data.len() || clx_data[offset] != 0x02 {
            return Err(ConvertError::InvalidDoc(
                "no Pcdt (type 0x02) found in Clx".into(),
            ));
        }

        // Pcdt: type byte (1) + lcb (4 bytes LE) + PlcPcd data
        offset += 1; // Skip type byte

        if offset + 4 > clx_data.len() {
            return Err(ConvertError::InvalidDoc(
                "Pcdt truncated: missing lcb".into(),
            ));
        }

        let lcb = u32::from_le_bytes([
            clx_data[offset],
            clx_data[offset + 1],
            clx_data[offset + 2],
            clx_data[offset + 3],
        ]) as usize;
        offset += 4;

        if offset + lcb > clx_data.len() {
            return Err(ConvertError::InvalidDoc(format!(
                "Pcdt PlcPcd data truncated: need {} bytes at offset {}, have {}",
                lcb,
                offset,
                clx_data.len() - offset
            )));
        }

        let plc_data = &clx_data[offset..offset + lcb];
        Self::parse_plc_pcd(plc_data)
    }

    /// Parse a PlcPcd structure.
    ///
    /// PlcPcd = (n+1) CPs (u32 LE each) followed by n Pcds (8 bytes each).
    /// Total size = (n+1)*4 + n*8 = 4 + n*12.
    fn parse_plc_pcd(data: &[u8]) -> Result<Self, ConvertError> {
        // PlcPcd must have at least 2 CPs (4 bytes each) + 1 Pcd (8 bytes) = 16 bytes
        if data.len() < 16 {
            return Err(ConvertError::InvalidDoc(format!(
                "PlcPcd too short: {} bytes (need at least 16)",
                data.len()
            )));
        }

        // Calculate number of pieces: total = (n+1)*4 + n*8 = 4 + n*12
        // So n = (total - 4) / 12
        let remaining = data.len() - 4;
        if !remaining.is_multiple_of(12) {
            return Err(ConvertError::InvalidDoc(format!(
                "PlcPcd size {} not valid: (size - 4) must be divisible by 12",
                data.len()
            )));
        }

        let num_pieces = remaining / 12;
        let cp_array_end = (num_pieces + 1) * 4;

        let mut pieces = Vec::with_capacity(num_pieces);

        for i in 0..num_pieces {
            let cp_offset = i * 4;
            let cp_start = u32::from_le_bytes([
                data[cp_offset],
                data[cp_offset + 1],
                data[cp_offset + 2],
                data[cp_offset + 3],
            ]);

            let cp_next_offset = (i + 1) * 4;
            let cp_end = u32::from_le_bytes([
                data[cp_next_offset],
                data[cp_next_offset + 1],
                data[cp_next_offset + 2],
                data[cp_next_offset + 3],
            ]);

            // Pcd is at cp_array_end + i * PCD_SIZE
            let pcd_offset = cp_array_end + i * PCD_SIZE;
            if pcd_offset + PCD_SIZE > data.len() {
                return Err(ConvertError::InvalidDoc(format!(
                    "Pcd {} truncated at offset {}",
                    i, pcd_offset
                )));
            }

            // Pcd structure (8 bytes):
            //   bytes 0-1: reserved (always 0)
            //   bytes 2-5: fc (with encoding bit 30)
            //   bytes 6-7: prm (property modifier, ignored for text extraction)
            let fc = u32::from_le_bytes([
                data[pcd_offset + 2],
                data[pcd_offset + 3],
                data[pcd_offset + 4],
                data[pcd_offset + 5],
            ]);

            let is_ansi = fc & FC_ANSI_BIT != 0;

            pieces.push(Piece {
                cp_start,
                cp_end,
                fc,
                is_ansi,
            });
        }

        Ok(PieceTable { pieces })
    }
}

impl Piece {
    /// Compute the actual byte offset into the WordDocument stream.
    ///
    /// For ANSI text, the raw offset (with bit 30 cleared) is divided by 2.
    /// For Unicode text, the raw offset is used directly.
    pub fn byte_offset(&self) -> usize {
        let raw = (self.fc & FC_OFFSET_MASK) as usize;
        if self.is_ansi {
            raw / 2
        } else {
            raw
        }
    }

    /// Extract text from the WordDocument stream for this piece.
    ///
    /// # Errors
    ///
    /// Returns `ConvertError::InvalidDoc` if the byte range is out of bounds
    /// or if UTF-16LE decoding fails for Unicode pieces.
    pub fn text_from_stream(&self, word_doc: &[u8]) -> Result<String, ConvertError> {
        let char_count = (self.cp_end - self.cp_start) as usize;
        let start = self.byte_offset();

        if self.is_ansi {
            // ANSI text: 1 byte per character (Windows-1252)
            let end = start + char_count;
            if end > word_doc.len() {
                return Err(ConvertError::InvalidDoc(format!(
                    "ANSI piece byte range {}..{} exceeds WordDocument size {}",
                    start,
                    end,
                    word_doc.len()
                )));
            }

            let slice = &word_doc[start..end];
            Ok(decode_cp1252(slice))
        } else {
            // Unicode text: 2 bytes per character (UTF-16LE)
            let byte_count = char_count * 2;
            let end = start + byte_count;
            if end > word_doc.len() {
                return Err(ConvertError::InvalidDoc(format!(
                    "Unicode piece byte range {}..{} exceeds WordDocument size {}",
                    start,
                    end,
                    word_doc.len()
                )));
            }

            let slice = &word_doc[start..end];
            decode_utf16le(slice)
        }
    }
}

/// Decode a Windows-1252 (CP1252) byte slice to a Rust String.
///
/// CP1252 is a superset of ISO 8859-1 (Latin-1), with printable characters
/// in the 0x80-0x9F range that Latin-1 maps to control characters.
fn decode_cp1252(data: &[u8]) -> String {
    // CP1252 special mappings for 0x80..0x9F
    const CP1252_MAP: [char; 32] = [
        '\u{20AC}', '\u{FFFD}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}',
        '\u{2021}', '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{FFFD}',
        '\u{017D}', '\u{FFFD}', '\u{FFFD}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}',
        '\u{2022}', '\u{2013}', '\u{2014}', '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}',
        '\u{0153}', '\u{FFFD}', '\u{017E}', '\u{0178}',
    ];

    let mut result = String::with_capacity(data.len());
    for &byte in data {
        match byte {
            0x80..=0x9F => result.push(CP1252_MAP[(byte - 0x80) as usize]),
            _ => result.push(byte as char), // Latin-1 compatible range
        }
    }
    result
}

/// Decode a UTF-16LE byte slice to a Rust String.
fn decode_utf16le(data: &[u8]) -> Result<String, ConvertError> {
    if !data.len().is_multiple_of(2) {
        return Err(ConvertError::InvalidDoc(
            "UTF-16LE data has odd byte count".into(),
        ));
    }

    let code_units: Vec<u16> = data
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    String::from_utf16(&code_units)
        .map_err(|e| ConvertError::InvalidDoc(format!("invalid UTF-16LE text: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal Clx buffer containing a Pcdt with the given CPs and Pcds.
    ///
    /// Each Pcd is constructed from an fc value (with encoding bit already set).
    fn make_clx(cps: &[u32], fcs: &[u32]) -> Vec<u8> {
        assert_eq!(cps.len(), fcs.len() + 1, "need n+1 CPs for n pieces");

        let num_pieces = fcs.len();
        let plc_size = (num_pieces + 1) * 4 + num_pieces * PCD_SIZE;

        // Pcdt header: type (1 byte) + lcb (4 bytes)
        let mut buf = Vec::new();
        buf.push(0x02); // Pcdt type
        buf.extend_from_slice(&(plc_size as u32).to_le_bytes());

        // CPs
        for &cp in cps {
            buf.extend_from_slice(&cp.to_le_bytes());
        }

        // Pcds (8 bytes each)
        for &fc in fcs {
            buf.extend_from_slice(&[0x00, 0x00]); // reserved
            buf.extend_from_slice(&fc.to_le_bytes()); // fc with encoding bit
            buf.extend_from_slice(&[0x00, 0x00]); // prm
        }

        buf
    }

    #[test]
    fn piece_table_parse_single_piece() {
        let clx = make_clx(&[0, 10], &[0x4000_0000 | 200]); // ANSI, offset 200
        let pt = PieceTable::parse(&clx).unwrap();
        assert_eq!(pt.pieces.len(), 1);
        assert_eq!(pt.pieces[0].cp_start, 0);
        assert_eq!(pt.pieces[0].cp_end, 10);
        assert!(pt.pieces[0].is_ansi);
    }

    #[test]
    fn piece_table_parse_multiple_pieces() {
        let clx = make_clx(
            &[0, 5, 15],
            &[0x4000_0000 | 100, 200], // first ANSI, second Unicode
        );
        let pt = PieceTable::parse(&clx).unwrap();
        assert_eq!(pt.pieces.len(), 2);
        assert_eq!(pt.pieces[0].cp_start, 0);
        assert_eq!(pt.pieces[0].cp_end, 5);
        assert!(pt.pieces[0].is_ansi);
        assert_eq!(pt.pieces[1].cp_start, 5);
        assert_eq!(pt.pieces[1].cp_end, 15);
        assert!(!pt.pieces[1].is_ansi);
    }

    #[test]
    fn piece_table_parse_empty_clx() {
        let result = PieceTable::parse(&[]);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("empty"), "got: {err}");
    }

    #[test]
    fn piece_table_parse_no_pcdt() {
        // Just garbage bytes, no 0x02 type marker
        let result = PieceTable::parse(&[0xFF, 0x00, 0x00]);
        assert!(result.is_err());
    }

    #[test]
    fn piece_table_ansi_bit() {
        let fc = 0x4000_0064_u32; // bit 30 set + offset 100
        let clx = make_clx(&[0, 10], &[fc]);
        let pt = PieceTable::parse(&clx).unwrap();
        assert!(pt.pieces[0].is_ansi);
    }

    #[test]
    fn piece_table_unicode_bit() {
        let fc = 0x0000_0064_u32; // bit 30 clear, offset 100
        let clx = make_clx(&[0, 10], &[fc]);
        let pt = PieceTable::parse(&clx).unwrap();
        assert!(!pt.pieces[0].is_ansi);
    }

    #[test]
    fn piece_byte_offset_ansi() {
        let piece = Piece {
            cp_start: 0,
            cp_end: 10,
            fc: 0x4000_00C8, // bit 30 set, raw offset = 200
            is_ansi: true,
        };
        // ANSI: (fc & mask) / 2 = 200 / 2 = 100
        assert_eq!(piece.byte_offset(), 100);
    }

    #[test]
    fn piece_byte_offset_unicode() {
        let piece = Piece {
            cp_start: 0,
            cp_end: 10,
            fc: 0x0000_00C8, // bit 30 clear, raw offset = 200
            is_ansi: false,
        };
        // Unicode: fc & mask = 200
        assert_eq!(piece.byte_offset(), 200);
    }

    #[test]
    fn piece_text_from_stream_ansi() {
        let piece = Piece {
            cp_start: 0,
            cp_end: 5,
            fc: 0x4000_0014, // ANSI, raw offset = 20 → byte offset = 20/2 = 10
            is_ansi: true,
        };

        let mut word_doc = vec![0u8; 10];
        // Place "Hello" at byte offset 10
        word_doc.extend_from_slice(b"Hello");

        let text = piece.text_from_stream(&word_doc).unwrap();
        assert_eq!(text, "Hello");
    }

    #[test]
    fn piece_text_from_stream_unicode() {
        let piece = Piece {
            cp_start: 0,
            cp_end: 5,
            fc: 0x0000_000A, // Unicode, byte offset = 10
            is_ansi: false,
        };

        let mut word_doc = vec![0u8; 10];
        // Place "Hello" as UTF-16LE at byte offset 10
        for &ch in &[b'H', b'e', b'l', b'l', b'o'] {
            word_doc.push(ch);
            word_doc.push(0x00); // High byte for ASCII in UTF-16LE
        }

        let text = piece.text_from_stream(&word_doc).unwrap();
        assert_eq!(text, "Hello");
    }

    #[test]
    fn piece_text_from_stream_out_of_bounds() {
        let piece = Piece {
            cp_start: 0,
            cp_end: 100,
            fc: 0x4000_0000, // ANSI, byte offset = 0
            is_ansi: true,
        };

        let word_doc = vec![0u8; 10]; // Too short for 100 chars
        let result = piece.text_from_stream(&word_doc);
        assert!(result.is_err());
    }

    #[test]
    fn piece_text_cp1252_special() {
        let piece = Piece {
            cp_start: 0,
            cp_end: 3,
            fc: 0x4000_0000, // ANSI, byte offset = 0
            is_ansi: true,
        };

        // \x80 = Euro sign, \x93 = left double quotation, \x94 = right double quotation
        let word_doc = vec![0x80, 0x93, 0x94];
        let text = piece.text_from_stream(&word_doc).unwrap();
        assert_eq!(text, "\u{20AC}\u{201C}\u{201D}");
    }

    #[test]
    fn piece_table_with_prc_prefix() {
        // Clx with a Prc entry before the Pcdt
        let mut buf = Vec::new();

        // Prc: type 0x01, cbGrpprl = 2, grpprl = [0xAA, 0xBB]
        buf.push(0x01);
        buf.extend_from_slice(&2u16.to_le_bytes());
        buf.extend_from_slice(&[0xAA, 0xBB]);

        // Then append a Pcdt
        let clx_part = make_clx(&[0, 10], &[0x4000_0000 | 100]);
        buf.extend_from_slice(&clx_part);

        let pt = PieceTable::parse(&buf).unwrap();
        assert_eq!(pt.pieces.len(), 1);
        assert_eq!(pt.pieces[0].cp_start, 0);
        assert_eq!(pt.pieces[0].cp_end, 10);
    }
}
