//! CHPx (Character Property eXceptions) parsing for DOC binary format.
//!
//! Reads the PlcfBteChpx bin table from the Table stream, then resolves each
//! entry through CHPX FKPs (Formatted disK Pages) in the WordDocument stream
//! to extract character formatting runs.
//!
//! Structure chain:
//! 1. PlcfBteChpx (Table stream) — PLC of FC/BteChpx pairs mapping CPs to FKP pages
//! 2. CHPX FKP (WordDocument stream, 512-byte pages) — contains character runs with
//!    FC ranges and offsets to CHPx data
//! 3. CHPx — byte-length-prefixed array of SPRM data
//! 4. SPRMs — parsed into character properties
//!
//! Reference: MS-DOC specification, Sections 2.8.10 (PlcfBteChpx), 2.9.40 (ChpxFkp)

use crate::error::ConvertError;
use crate::fib::Fib;
use crate::sprm::{
    parse_sprms, SPRM_CF_BOLD, SPRM_CF_CAPS, SPRM_CF_ITALIC, SPRM_CF_STRIKE, SPRM_C_HPS,
    SPRM_C_ICO, SPRM_C_KUL, SPRM_C_RG_FTC0, SPRM_C_SUPER_SUB,
};

/// Size of a CHPX FKP page in bytes.
const FKP_PAGE_SIZE: usize = 512;

/// A character formatting run, mapping a range of file character positions
/// to a set of character properties.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterRun {
    /// File character position start (inclusive).
    pub fc_start: u32,
    /// File character position end (exclusive).
    pub fc_end: u32,
    /// Resolved character properties from SPRM data.
    pub properties: CharProperties,
}

/// Character properties extracted from CHPx SPRM data.
///
/// All fields are `Option` — `None` means the property was not explicitly set
/// in this run (inherits from the default/style).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CharProperties {
    /// Bold formatting.
    pub bold: Option<bool>,
    /// Italic formatting.
    pub italic: Option<bool>,
    /// Font size in half-points (divide by 2 for points).
    pub font_size_half_pts: Option<u16>,
    /// Color index into the standard DOC color table (0-16).
    pub color_index: Option<u8>,
    /// Font index for ASCII text (index into the font table).
    pub font_index: Option<u16>,
    /// Underline type (0=none, 1=single, 2=words-only, etc.).
    pub underline: Option<u8>,
    /// Strikethrough formatting.
    pub strikethrough: Option<bool>,
    /// Superscript formatting.
    pub superscript: Option<bool>,
    /// Subscript formatting.
    pub subscript: Option<bool>,
    /// All caps formatting.
    pub all_caps: Option<bool>,
}

/// Apply a list of SPRMs to a `CharProperties`, setting fields as encountered.
pub fn apply_sprms_to_properties(sprm_data: &[u8]) -> CharProperties {
    let sprms = parse_sprms(sprm_data);
    let mut props = CharProperties::default();

    for sprm in &sprms {
        match sprm.opcode {
            SPRM_CF_BOLD => {
                if let Some(&val) = sprm.operand.first() {
                    // 0=off, 1=on, 128=toggle (treat as on for simplicity)
                    props.bold = Some(val != 0 && val != 128 || val == 1);
                    // More precise: 0 → false, 1 → true, 128 → toggle (context-dependent)
                    // Without parent context, treat 1 and 128 as true
                    props.bold = Some(val == 1 || val == 128);
                }
            }
            SPRM_CF_ITALIC => {
                if let Some(&val) = sprm.operand.first() {
                    props.italic = Some(val == 1 || val == 128);
                }
            }
            SPRM_CF_STRIKE => {
                if let Some(&val) = sprm.operand.first() {
                    props.strikethrough = Some(val == 1 || val == 128);
                }
            }
            SPRM_CF_CAPS => {
                if let Some(&val) = sprm.operand.first() {
                    props.all_caps = Some(val == 1 || val == 128);
                }
            }
            SPRM_C_KUL => {
                if let Some(&val) = sprm.operand.first() {
                    props.underline = Some(val);
                }
            }
            SPRM_C_ICO => {
                if let Some(&val) = sprm.operand.first() {
                    props.color_index = Some(val);
                }
            }
            SPRM_C_HPS => {
                if sprm.operand.len() >= 2 {
                    let size = u16::from_le_bytes([sprm.operand[0], sprm.operand[1]]);
                    props.font_size_half_pts = Some(size);
                }
            }
            SPRM_C_SUPER_SUB => {
                if let Some(&val) = sprm.operand.first() {
                    match val {
                        1 => {
                            props.superscript = Some(true);
                            props.subscript = Some(false);
                        }
                        2 => {
                            props.superscript = Some(false);
                            props.subscript = Some(true);
                        }
                        _ => {
                            props.superscript = Some(false);
                            props.subscript = Some(false);
                        }
                    }
                }
            }
            SPRM_C_RG_FTC0 => {
                if sprm.operand.len() >= 2 {
                    let idx = u16::from_le_bytes([sprm.operand[0], sprm.operand[1]]);
                    props.font_index = Some(idx);
                }
            }
            _ => {
                // Unknown SPRM — silently ignore
            }
        }
    }

    props
}

/// Parse the PlcfBteChpx bin table and resolve character formatting runs.
///
/// Reads the bin table from the Table stream (at the offset/length specified
/// by the FIB), then for each entry reads the corresponding CHPX FKP page
/// from the WordDocument stream and extracts character runs with formatting.
///
/// # Errors
///
/// Returns `ConvertError::InvalidDoc` if:
/// - The PlcfBteChpx offset/length in the FIB is invalid
/// - An FKP page number points outside the WordDocument stream
/// - The FKP structure is malformed
pub fn parse_chpx_bin_table(
    table_stream: &[u8],
    word_stream: &[u8],
    fib: &Fib,
) -> Result<Vec<CharacterRun>, ConvertError> {
    let fc_offset = fib.fc_plcf_bte_chpx as usize;
    let lcb = fib.lcb_plcf_bte_chpx as usize;

    if lcb == 0 {
        // No character formatting bin table — all text uses defaults
        return Ok(Vec::new());
    }

    if fc_offset + lcb > table_stream.len() {
        return Err(ConvertError::InvalidDoc(format!(
            "PlcfBteChpx range {}..{} exceeds table stream size {}",
            fc_offset,
            fc_offset + lcb,
            table_stream.len()
        )));
    }

    let plcf_data = &table_stream[fc_offset..fc_offset + lcb];

    // PlcfBteChpx is a PLC: (n+1) FCs (4 bytes each) + n BteChpx (4 bytes each)
    // Total = (n+1)*4 + n*4 = 4 + n*8
    // So n = (total - 4) / 8
    if plcf_data.len() < 12 {
        return Err(ConvertError::InvalidDoc(format!(
            "PlcfBteChpx too short: {} bytes (need at least 12)",
            plcf_data.len()
        )));
    }

    let remaining = plcf_data.len() - 4;
    if !remaining.is_multiple_of(8) {
        return Err(ConvertError::InvalidDoc(format!(
            "PlcfBteChpx size {} not valid: (size - 4) must be divisible by 8",
            plcf_data.len()
        )));
    }

    let num_entries = remaining / 8;
    let fc_array_end = (num_entries + 1) * 4;

    let mut all_runs = Vec::new();

    for i in 0..num_entries {
        // Read BteChpx (page number in WordDocument stream)
        let bte_offset = fc_array_end + i * 4;
        if bte_offset + 4 > plcf_data.len() {
            return Err(ConvertError::InvalidDoc(format!(
                "BteChpx {} truncated at offset {}",
                i, bte_offset
            )));
        }

        let page_number = u32::from_le_bytes([
            plcf_data[bte_offset],
            plcf_data[bte_offset + 1],
            plcf_data[bte_offset + 2],
            plcf_data[bte_offset + 3],
        ]);

        // Read the CHPX FKP page from WordDocument stream
        let page_offset = page_number as usize * FKP_PAGE_SIZE;
        if page_offset + FKP_PAGE_SIZE > word_stream.len() {
            // Skip this page if it's out of bounds rather than failing entirely
            continue;
        }

        let fkp = &word_stream[page_offset..page_offset + FKP_PAGE_SIZE];
        let mut runs = parse_chpx_fkp(fkp)?;
        all_runs.append(&mut runs);
    }

    Ok(all_runs)
}

/// Parse a single CHPX FKP (Formatted disK Page, 512 bytes).
///
/// FKP structure:
/// - Last byte: `crun` (number of character runs)
/// - First `(crun+1) * 4` bytes: FC array (file character positions, u32 LE)
/// - After FC array: `crun` bytes of offsets (in 2-byte words) into the FKP
///   for each CHPx
/// - Each CHPx: first byte = `cb` (size in bytes), then `cb` bytes of SPRM data
fn parse_chpx_fkp(fkp: &[u8]) -> Result<Vec<CharacterRun>, ConvertError> {
    if fkp.len() != FKP_PAGE_SIZE {
        return Err(ConvertError::InvalidDoc(format!(
            "CHPX FKP must be {} bytes, got {}",
            FKP_PAGE_SIZE,
            fkp.len()
        )));
    }

    let crun = fkp[FKP_PAGE_SIZE - 1] as usize;
    if crun == 0 {
        return Ok(Vec::new());
    }

    // Validate that we have enough room for the FC array and offset bytes
    let fc_array_size = (crun + 1) * 4;
    let offsets_start = fc_array_size;
    let offsets_end = offsets_start + crun;
    if offsets_end > FKP_PAGE_SIZE - 1 {
        return Err(ConvertError::InvalidDoc(format!(
            "CHPX FKP crun={crun} too large: FC array + offsets ({offsets_end}) exceeds page"
        )));
    }

    let mut runs = Vec::with_capacity(crun);

    for i in 0..crun {
        // Read FC pair for this run
        let fc_off = i * 4;
        let fc_start = u32::from_le_bytes([
            fkp[fc_off],
            fkp[fc_off + 1],
            fkp[fc_off + 2],
            fkp[fc_off + 3],
        ]);
        let fc_next_off = (i + 1) * 4;
        let fc_end = u32::from_le_bytes([
            fkp[fc_next_off],
            fkp[fc_next_off + 1],
            fkp[fc_next_off + 2],
            fkp[fc_next_off + 3],
        ]);

        // Read the CHPx offset byte
        let chpx_word_offset = fkp[offsets_start + i] as usize;

        let properties = if chpx_word_offset == 0 {
            // Offset 0 means no CHPx — use defaults
            CharProperties::default()
        } else {
            // Convert word offset to byte offset within the FKP
            let chpx_byte_offset = chpx_word_offset * 2;
            if chpx_byte_offset >= FKP_PAGE_SIZE {
                // Out of bounds — use defaults
                CharProperties::default()
            } else {
                // First byte of CHPx is cb (size)
                let cb = fkp[chpx_byte_offset] as usize;
                if cb == 0 || chpx_byte_offset + 1 + cb > FKP_PAGE_SIZE {
                    CharProperties::default()
                } else {
                    let sprm_data = &fkp[chpx_byte_offset + 1..chpx_byte_offset + 1 + cb];
                    apply_sprms_to_properties(sprm_data)
                }
            }
        };

        runs.push(CharacterRun {
            fc_start,
            fc_end,
            properties,
        });
    }

    Ok(runs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_char_properties_from_sprms() {
        // Bold SPRM only
        let mut data = Vec::new();
        data.extend_from_slice(&SPRM_CF_BOLD.to_le_bytes());
        data.push(0x01);

        let props = apply_sprms_to_properties(&data);
        assert_eq!(props.bold, Some(true));
        assert_eq!(props.italic, None);
        assert_eq!(props.font_size_half_pts, None);
    }

    #[test]
    fn char_properties_bold_italic() {
        let mut data = Vec::new();
        // Bold on
        data.extend_from_slice(&SPRM_CF_BOLD.to_le_bytes());
        data.push(0x01);
        // Italic on
        data.extend_from_slice(&SPRM_CF_ITALIC.to_le_bytes());
        data.push(0x01);

        let props = apply_sprms_to_properties(&data);
        assert_eq!(props.bold, Some(true));
        assert_eq!(props.italic, Some(true));
    }

    #[test]
    fn char_properties_font_size() {
        let mut data = Vec::new();
        // Font size = 28 half-points (14pt)
        data.extend_from_slice(&SPRM_C_HPS.to_le_bytes());
        data.extend_from_slice(&28u16.to_le_bytes());

        let props = apply_sprms_to_properties(&data);
        assert_eq!(props.font_size_half_pts, Some(28));
    }

    #[test]
    fn char_properties_color_index() {
        let mut data = Vec::new();
        // Color index = 6 (red in DOC color table)
        data.extend_from_slice(&SPRM_C_ICO.to_le_bytes());
        data.push(6);

        let props = apply_sprms_to_properties(&data);
        assert_eq!(props.color_index, Some(6));
    }

    #[test]
    fn char_properties_empty() {
        let props = apply_sprms_to_properties(&[]);
        assert_eq!(props, CharProperties::default());
        assert_eq!(props.bold, None);
        assert_eq!(props.italic, None);
        assert_eq!(props.font_size_half_pts, None);
        assert_eq!(props.color_index, None);
        assert_eq!(props.font_index, None);
        assert_eq!(props.underline, None);
        assert_eq!(props.strikethrough, None);
        assert_eq!(props.superscript, None);
        assert_eq!(props.subscript, None);
    }

    #[test]
    fn char_properties_superscript_subscript() {
        // Superscript
        let mut data = Vec::new();
        data.extend_from_slice(&SPRM_C_SUPER_SUB.to_le_bytes());
        data.push(1);
        let props = apply_sprms_to_properties(&data);
        assert_eq!(props.superscript, Some(true));
        assert_eq!(props.subscript, Some(false));

        // Subscript
        let mut data = Vec::new();
        data.extend_from_slice(&SPRM_C_SUPER_SUB.to_le_bytes());
        data.push(2);
        let props = apply_sprms_to_properties(&data);
        assert_eq!(props.superscript, Some(false));
        assert_eq!(props.subscript, Some(true));

        // Normal (neither)
        let mut data = Vec::new();
        data.extend_from_slice(&SPRM_C_SUPER_SUB.to_le_bytes());
        data.push(0);
        let props = apply_sprms_to_properties(&data);
        assert_eq!(props.superscript, Some(false));
        assert_eq!(props.subscript, Some(false));
    }
}
