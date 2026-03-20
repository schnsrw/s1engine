# ZIP Entry Preservation — Detailed Tracker

> DOCX/ODT files are ZIP archives. Many entries contain data that s1engine doesn't semantically model but MUST preserve on round-trip to avoid data loss.

## Architecture

```
DocumentModel.preserved_parts: HashMap<String, Vec<u8>>
├── Reader: extracts matching ZIP entries → add_preserved_part(path, bytes)
└── Writer: iterates preserved_parts → writes each back to ZIP
```

## Preserved Prefixes & Files

| ZIP Path | Content | Read | Write | Status |
|----------|---------|------|-------|--------|
| `_xmlsignatures/*.xml` | Digital signatures (XMLDSIG) | DONE | DONE | Parsed for signer info + round-tripped |
| `_xmlsignatures/*.rels` | Signature relationships | DONE | DONE | Round-tripped |
| `word/vbaProject.bin` | VBA macro binary | DONE | DONE | Detected + round-tripped |
| `word/vbaData.xml` | VBA data | DONE | DONE | Round-tripped |
| `customXml/*` | Custom XML parts | DONE | DONE | Round-tripped |
| `word/diagrams/*` | SmartArt diagram data | DONE | DONE | Round-tripped |
| `word/charts/*` | Chart definitions | DONE | DONE | Round-tripped |
| `word/embeddings/*` | OLE embedded objects | DONE | DONE | Round-tripped |

## API

```rust
// Store a ZIP entry
doc.add_preserved_part("customXml/item1.xml", xml_bytes);

// Check existence
doc.has_preserved_part("word/vbaProject.bin");

// Get all preserved entries
for (path, data) in doc.preserved_parts() { ... }
```

## Metadata Flags

| Key | Value | Source |
|-----|-------|--------|
| `hasDigitalSignature` | `"true"` | `_xmlsignatures/` entries found |
| `hasMacros` | `"true"` | `vbaProject.bin` found |
| `signatureSubject` | CN/DN string | Parsed from X509SubjectName |
| `signatureDate` | ISO 8601 | Parsed from signing time |
| `signatureCount` | number | Count of signature XML files |
| `signatureValid` | `"unverified"` | Crypto validation not implemented |

## Impact on Phase 5 Items

| Item | Before | After |
|------|--------|-------|
| Q6 SmartArt | Dropped on import | `word/diagrams/` preserved — visible as placeholder, survives round-trip |
| Q7 Charts | DrawingML XML preserved | `word/charts/` ZIP entries also preserved — full round-trip |
| Q8 OLE objects | Dropped | `word/embeddings/` preserved — round-trip works |
| Q9 Custom XML | Dropped | `customXml/` preserved — round-trip works |
| P4 VBA macros | In ZIP but not accessible | `vbaProject.bin` preserved + `hasMacros` metadata flag |
| P5 Signatures | Not detected | Parsed, metadata extracted, round-tripped |

## What's NOT Preserved (Limitations)

- ZIP entries not matching the prefix list are dropped (e.g., `docProps/app.xml`, `docProps/thumbnail.jpeg`)
- `[Content_Types].xml` is regenerated, not preserved (may lose custom content type mappings)
- Signature entries are re-written from preserved bytes — modifying document content invalidates signatures but we don't re-sign
