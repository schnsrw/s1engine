# Format Crates

Each format has its own crate that only depends on `s1-model`.

## DOCX (s1-format-docx)

Reads and writes Office Open XML (ECMA-376):
- Paragraph/run formatting, styles, lists
- Tables (including nested), images, hyperlinks
- Headers/footers, comments, footnotes/endnotes
- Track changes, bookmarks, section properties
- Media deduplication by content hash

## ODT (s1-format-odt)

Reads and writes Open Document Format (ODF 1.2):
- Full formatting parity with DOCX
- Automatic styles → node attributes mapping
- Table column parsing with repeat support
- TOC source attributes preserved

## PDF (s1-format-pdf)

Export only (via pdf-writer):
- Font embedding with subsetting
- Proper ToUnicode CMap for text extraction
- JPEG passthrough, PNG decode
- Page layout from s1-layout

## TXT & Markdown (s1-format-txt, s1-format-md)

- Plain text: paragraph-per-line
- Markdown: GFM tables, headings, lists, images, code blocks
