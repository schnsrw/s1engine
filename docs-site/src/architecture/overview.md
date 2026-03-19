# Architecture Overview

```
Consumer Applications
        |  Rust API / C FFI / WASM
+-------v--------------------------------------------+
|                s1engine (facade)                    |
|----------------------------------------------------|
|  s1-ops       s1-layout       s1-convert           |
|  Operations   Page Layout     Format Conversion    |
|  Undo/Redo    Pagination      DOC → DOCX           |
|----------------------------------------------------|
|  s1-crdt                s1-model                   |
|  Collaborative          Core Document Model        |
|  Editing (Fugue)        (zero external deps)       |
|----------------------------------------------------|
|  format-docx  format-odt  format-pdf  format-txt   |
|  format-md                                         |
|----------------------------------------------------|
|                s1-text (Pure Rust)                  |
|        rustybuzz  ttf-parser  fontdb               |
+----------------------------------------------------+
```

## Design Principles

1. **s1-model has zero external dependencies** — pure Rust data structures
2. **All mutations via Operations** — never modify the tree directly
3. **Format isolation** — each format crate only depends on s1-model
4. **No panics in library code** — all public functions return `Result`
5. **No unsafe** — unless documented with a SAFETY comment
