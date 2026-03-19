# C FFI API

s1engine provides C-compatible bindings via opaque handles.

## Build

```bash
cd ffi/c
cargo build --release
# Output: target/release/libs1engine_c.{so,dylib,dll}
```

## API

```c
#include "s1engine.h"

// Create engine
S1Engine* engine = s1_engine_new();

// Open a document
S1Error* err = NULL;
S1Document* doc = s1_engine_open(engine, data, len, &err);

// Get text
S1String* text = s1_document_plain_text(doc);
printf("%s\n", s1_string_ptr(text));

// Export
S1Bytes* docx = s1_document_export(doc, "docx", &err);
// Use s1_bytes_data() and s1_bytes_len()

// Cleanup
s1_string_free(text);
s1_bytes_free(docx);
s1_document_free(doc);
s1_engine_free(engine);
```

## Handle Types

| Type | Description | Free Function |
|------|-------------|---------------|
| `S1Engine` | Engine instance | `s1_engine_free` |
| `S1Document` | Open document | `s1_document_free` |
| `S1Error` | Error message | `s1_error_free` |
| `S1String` | UTF-8 string | `s1_string_free` |
| `S1Bytes` | Byte buffer | `s1_bytes_free` |
