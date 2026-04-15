//! DOCY binary format constants.
//! Mapped from Serialize2.js c_oSer* constants.

// Table types (c_oSerTableTypes)
pub mod table_type {
    pub const SIGNATURE: u8 = 0;
    pub const MEDIA: u8 = 2;
    pub const NUMBERING: u8 = 3;
    pub const HDR_FTR: u8 = 4;
    pub const STYLE: u8 = 5;
    pub const DOCUMENT: u8 = 6;
    pub const OTHER: u8 = 7;
    pub const COMMENTS: u8 = 8;
    pub const SETTINGS: u8 = 9;
    pub const FOOTNOTES: u8 = 10;
    pub const ENDNOTES: u8 = 11;
}

// Signature types (c_oSerSigTypes)
pub mod sig {
    pub const VERSION: u8 = 0;
}

// Paragraph types (c_oSerParType)
pub mod par {
    pub const PAR: u8 = 0;
    pub const PPR: u8 = 1;
    pub const CONTENT: u8 = 2;
    pub const TABLE: u8 = 3;
    pub const SECT_PR: u8 = 4;
    pub const RUN: u8 = 5;
    pub const COMMENT_START: u8 = 6;
    pub const COMMENT_END: u8 = 7;
    pub const HYPERLINK: u8 = 10;
    pub const SDT: u8 = 15;
    pub const BOOKMARK_START: u8 = 23;
    pub const BOOKMARK_END: u8 = 24;
}

// Run types (c_oSerRunType)
pub mod run {
    pub const RUN: u8 = 0;
    pub const RPR: u8 = 1;
    pub const TAB: u8 = 2;
    pub const PAGENUM: u8 = 3;
    pub const PAGEBREAK: u8 = 4;
    pub const LINEBREAK: u8 = 5;
    pub const IMAGE: u8 = 6;
    pub const CONTENT: u8 = 8;
    pub const COLUMN_BREAK: u8 = 18;
    pub const FOOTNOTE_REF: u8 = 24;
    pub const ENDNOTE_REF: u8 = 25;
    pub const FOOTNOTE_REFERENCE: u8 = 26;
    pub const ENDNOTE_REFERENCE: u8 = 27;
    pub const FLD_CHAR: u8 = 29;
    pub const INSTR_TEXT: u8 = 30;
}

// Run properties (c_oSerProp_rPrType)
pub mod rpr {
    pub const BOLD: u8 = 0;
    pub const ITALIC: u8 = 1;
    pub const UNDERLINE: u8 = 2;
    pub const STRIKEOUT: u8 = 3;
    pub const FONT_ASCII: u8 = 4;
    pub const FONT_HANSI: u8 = 5;
    pub const FONT_AE: u8 = 6;
    pub const FONT_CS: u8 = 7;
    pub const FONT_SIZE: u8 = 8;
    pub const COLOR: u8 = 9;
    pub const VERT_ALIGN: u8 = 10;
    pub const HIGHLIGHT: u8 = 11;
    pub const RSTYLE: u8 = 13;
    pub const SPACING: u8 = 14;
    pub const DSTRIKEOUT: u8 = 42;
    pub const BOLD_CS: u8 = 43;
    pub const FONT_SIZE_CS: u8 = 44;
    pub const ITALIC_CS: u8 = 45;
    pub const CAPS: u8 = 49;
    pub const SMALL_CAPS: u8 = 50;
    pub const VANISH: u8 = 19;
    pub const LANG: u8 = 40;
}

// Paragraph properties (c_oSerProp_pPrType)
pub mod ppr {
    pub const CONTEXTUAL_SPACING: u8 = 1;
    pub const IND_LEFT: u8 = 3;
    pub const IND_RIGHT: u8 = 4;
    pub const JC: u8 = 5;
    pub const KEEP_LINES: u8 = 6;
    pub const KEEP_NEXT: u8 = 7;
    pub const PAGE_BREAK_BEFORE: u8 = 8;
    pub const SPACING: u8 = 9;
    pub const SHD: u8 = 14;
    pub const WIDOW_CONTROL: u8 = 15;
    pub const PARA_STYLE: u8 = 21;
    pub const PBDR: u8 = 32;
    pub const NUM_PR: u8 = 37;
    pub const TABS: u8 = 38;
    pub const IND_FIRST_LINE: u8 = 39;
    pub const OUTLINE_LVL: u8 = 40;
    pub const BIDI: u8 = 46;
}

// Spacing sub-properties (c_oSerProp_pPrType spacing sub-types)
pub mod spacing {
    pub const LINE: u8 = 0;
    pub const LINE_RULE: u8 = 1;
    pub const BEFORE: u8 = 2;
    pub const AFTER: u8 = 3;
    pub const BEFORE_AUTO: u8 = 5;
    pub const AFTER_AUTO: u8 = 6;
}

// Table properties (c_oSerProp_tblPrType)
pub mod tbl_pr {
    pub const JC: u8 = 0;
    pub const TABLE_BORDERS: u8 = 5;
    pub const TABLE_CELL_MAR: u8 = 6;
    pub const TABLE_W: u8 = 7;
    pub const TABLE_LAYOUT: u8 = 8;
    pub const TABLE_IND: u8 = 15;
    pub const ROWS: u8 = 1;
}

// Row properties (c_oSerProp_rowPrType)
pub mod row_pr {
    pub const HEIGHT: u8 = 0;
    pub const TABLE_HEADER: u8 = 5;
}

// Cell properties (c_oSerProp_cellPrType)
pub mod cell_pr {
    pub const GRID_SPAN: u8 = 0;
    pub const BORDERS: u8 = 1;
    pub const SHD: u8 = 2;
    pub const CELL_W: u8 = 3;
    pub const VMERGE: u8 = 4;
    pub const VALIGN: u8 = 5;
}

// Section properties (c_oSerProp_secPrType)
pub mod sec_pr {
    pub const PG_SZ_W: u8 = 0;
    pub const PG_SZ_H: u8 = 1;
    pub const PG_SZ_ORIENT: u8 = 12;
    pub const PG_MAR_TOP: u8 = 3;
    pub const PG_MAR_LEFT: u8 = 4;
    pub const PG_MAR_RIGHT: u8 = 5;
    pub const PG_MAR_BOTTOM: u8 = 6;
    pub const PG_MAR_HEADER: u8 = 7;
    pub const PG_MAR_FOOTER: u8 = 8;
    pub const COLS: u8 = 10;
    pub const TITLE_PG: u8 = 13;
}

// Style types (c_oSer_sts)
pub mod style {
    pub const STYLE: u8 = 0;
    pub const STYLE_ID: u8 = 1;
    pub const STYLE_NAME: u8 = 2;
    pub const STYLE_BASED_ON: u8 = 3;
    pub const STYLE_NEXT: u8 = 4;
    pub const STYLE_TEXT_PR: u8 = 5;
    pub const STYLE_PARA_PR: u8 = 6;
    pub const STYLE_TABLE_PR: u8 = 7;
    pub const STYLE_DEFAULT: u8 = 8;
    pub const STYLE_TYPE: u8 = 9;
    pub const STYLE_Q_FORMAT: u8 = 10;
    pub const STYLE_UI_PRIORITY: u8 = 11;
    pub const STYLE_LINK: u8 = 18;
}

// Style table top-level (c_oSer_st)
pub mod style_table {
    pub const DEF_PPR: u8 = 0;
    pub const DEF_RPR: u8 = 1;
    pub const STYLES: u8 = 2;
}

// Header/footer types (c_oSerHdrFtrTypes)
pub mod hdr_ftr {
    pub const HEADER: u8 = 0;
    pub const FOOTER: u8 = 1;
    pub const FIRST: u8 = 2;
    pub const EVEN: u8 = 3;
    pub const ODD: u8 = 4;
    pub const CONTENT: u8 = 5;
}

// Comment types (c_oSer_CommentsType)
pub mod comments {
    pub const COMMENT: u8 = 0;
    pub const ID: u8 = 1;
    pub const INITIALS: u8 = 2;
    pub const USER_NAME: u8 = 3;
    pub const USER_ID: u8 = 4;
    pub const DATE: u8 = 5;
    pub const TEXT: u8 = 6;
    pub const SOLVED: u8 = 8;
    pub const REPLIES: u8 = 9;
}

// Numbering types (c_oSerNumTypes)
pub mod num {
    pub const ABSTRACT_NUMS: u8 = 0;
    pub const ABSTRACT_NUM: u8 = 1;
    pub const ABSTRACT_NUM_ID: u8 = 2;
    pub const ABSTRACT_NUM_LVLS: u8 = 4;
    pub const LVL: u8 = 5;
    pub const LVL_FORMAT: u8 = 6;
    pub const LVL_TEXT: u8 = 8;
    pub const LVL_TEXT_ITEM: u8 = 9;
    pub const LVL_TEXT_ITEM_TEXT: u8 = 10;
    pub const LVL_TEXT_ITEM_NUM: u8 = 11;
    pub const LVL_RESTART: u8 = 12;
    pub const LVL_START: u8 = 13;
    pub const LVL_SUFF: u8 = 14;
    pub const LVL_PARA_PR: u8 = 15;
    pub const LVL_TEXT_PR: u8 = 16;
    pub const NUMS: u8 = 17;
    pub const NUM: u8 = 18;
    pub const NUM_ANUM_ID: u8 = 19;
    pub const NUM_NUM_ID: u8 = 20;
}

// Footnote/endnote types (c_oSerNotes)
pub mod notes {
    pub const NOTE: u8 = 0;
    pub const NOTE_TYPE: u8 = 1;
    pub const NOTE_ID: u8 = 2;
    pub const NOTE_CONTENT: u8 = 3;
}

// Settings types (c_oSer_SettingsType)
pub mod settings {
    pub const DEFAULT_TAB_STOP_TWIPS: u8 = 9;
    pub const TRACK_REVISIONS: u8 = 3;
    pub const COMPAT: u8 = 8;
}

// Color types
pub mod color {
    pub const AUTO: u8 = 0;
    pub const RGB: u8 = 1;
    pub const THEME: u8 = 2;
}

// Alignment values (sdkjs-specific: 0=Right, 1=Left, 2=Center, 3=Justify)
pub mod align {
    pub const RIGHT: u8 = 0;
    pub const LEFT: u8 = 1;
    pub const CENTER: u8 = 2;
    pub const JUSTIFY: u8 = 3;
}

/// DOCY format version
pub const DOCY_VERSION: u32 = 5;
pub const DOCY_SIGNATURE: &str = "DOCY";

/// Convert points to twips (1 pt = 20 twips)
pub fn pts_to_twips(pts: f64) -> i32 {
    (pts * 20.0).round() as i32
}

/// Convert points to half-points (1 pt = 2 half-pts)
pub fn pts_to_half_pts(pts: f64) -> i32 {
    (pts * 2.0).round() as i32
}

/// Convert points to EMU (1 pt = 12700 EMU)
pub fn pts_to_emu(pts: f64) -> i32 {
    (pts * 12700.0).round() as i32
}
