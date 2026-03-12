//! C FFI bindings for s1engine.
//!
//! Provides a C-compatible API using opaque handles and `extern "C"` functions.
//! All functions follow the naming convention `s1_<type>_<action>`.
//!
//! # Safety
//!
//! All functions that accept pointers check for null before dereferencing.
//! Callers must ensure that handles are not used after being freed.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

// --- Opaque handles ---

/// Opaque handle to an s1engine Engine.
pub struct S1Engine {
    inner: s1engine::Engine,
}

/// Opaque handle to an s1engine Document.
pub struct S1Document {
    inner: s1engine::Document,
}

/// Opaque handle to an error message.
pub struct S1Error {
    message: CString,
}

/// Opaque handle to a byte buffer returned from export.
pub struct S1Bytes {
    data: Vec<u8>,
}

/// Opaque handle to a string returned from the API.
pub struct S1String {
    value: CString,
}

// --- Engine ---

/// Create a new engine instance.
///
/// Returns a non-null pointer on success. The caller must free it with
/// `s1_engine_free`.
#[no_mangle]
pub extern "C" fn s1_engine_new() -> *mut S1Engine {
    Box::into_raw(Box::new(S1Engine {
        inner: s1engine::Engine::new(),
    }))
}

/// Free an engine instance.
///
/// # Safety
///
/// `engine` must be a valid pointer returned by `s1_engine_new`, or null.
/// After this call, the pointer must not be used again.
#[no_mangle]
pub unsafe extern "C" fn s1_engine_free(engine: *mut S1Engine) {
    if !engine.is_null() {
        drop(unsafe { Box::from_raw(engine) });
    }
}

/// Create a new empty document.
///
/// Returns a non-null pointer on success, or null if `engine` is null.
/// The caller must free the document with `s1_document_free`.
///
/// # Safety
///
/// `engine` must be a valid pointer returned by `s1_engine_new`, or null.
#[no_mangle]
pub unsafe extern "C" fn s1_engine_create(engine: *const S1Engine) -> *mut S1Document {
    if engine.is_null() {
        return ptr::null_mut();
    }
    let engine = unsafe { &*engine };
    Box::into_raw(Box::new(S1Document {
        inner: engine.inner.create(),
    }))
}

/// Open a document from bytes with auto-detected format.
///
/// Returns a non-null pointer on success. On failure, returns null and
/// sets `*error_out` to a non-null error (caller must free with `s1_error_free`).
///
/// # Safety
///
/// - `engine` must be a valid pointer or null.
/// - `data` must point to at least `len` bytes, or be null.
/// - `error_out` must be a valid pointer to a `*mut S1Error`, or null.
#[no_mangle]
pub unsafe extern "C" fn s1_engine_open(
    engine: *const S1Engine,
    data: *const u8,
    len: usize,
    error_out: *mut *mut S1Error,
) -> *mut S1Document {
    if engine.is_null() || data.is_null() {
        set_error(error_out, "null pointer");
        return ptr::null_mut();
    }
    let engine = unsafe { &*engine };
    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    match engine.inner.open(slice) {
        Ok(doc) => Box::into_raw(Box::new(S1Document { inner: doc })),
        Err(e) => {
            set_error(error_out, &e.to_string());
            ptr::null_mut()
        }
    }
}

// --- Document ---

/// Free a document.
///
/// # Safety
///
/// `doc` must be a valid pointer returned by `s1_engine_create` or
/// `s1_engine_open`, or null. After this call, the pointer must not be used.
#[no_mangle]
pub unsafe extern "C" fn s1_document_free(doc: *mut S1Document) {
    if !doc.is_null() {
        drop(unsafe { Box::from_raw(doc) });
    }
}

/// Get the document's plain text content.
///
/// Returns a non-null `S1String` on success (caller must free with
/// `s1_string_free`), or null if `doc` is null.
///
/// # Safety
///
/// `doc` must be a valid pointer or null.
#[no_mangle]
pub unsafe extern "C" fn s1_document_plain_text(doc: *const S1Document) -> *mut S1String {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let doc = unsafe { &*doc };
    let text = doc.inner.to_plain_text();
    match CString::new(text) {
        Ok(cstr) => Box::into_raw(Box::new(S1String { value: cstr })),
        Err(_) => ptr::null_mut(),
    }
}

/// Export the document to the specified format.
///
/// `format` should be one of: "docx", "odt", "txt", "pdf" (null-terminated C string).
///
/// Returns a non-null `S1Bytes` on success (caller must free with
/// `s1_bytes_free`). On failure, returns null and sets `*error_out`.
///
/// # Safety
///
/// - `doc` must be a valid pointer or null.
/// - `format` must be a valid null-terminated C string, or null.
/// - `error_out` must be a valid pointer or null.
#[no_mangle]
pub unsafe extern "C" fn s1_document_export(
    doc: *const S1Document,
    format: *const c_char,
    error_out: *mut *mut S1Error,
) -> *mut S1Bytes {
    if doc.is_null() || format.is_null() {
        set_error(error_out, "null pointer");
        return ptr::null_mut();
    }
    let doc = unsafe { &*doc };
    let fmt_str = match unsafe { CStr::from_ptr(format) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error(error_out, "invalid UTF-8 in format string");
            return ptr::null_mut();
        }
    };
    let fmt = match parse_format(fmt_str) {
        Some(f) => f,
        None => {
            set_error(error_out, &format!("unsupported format: {fmt_str}"));
            return ptr::null_mut();
        }
    };
    match doc.inner.export(fmt) {
        Ok(bytes) => Box::into_raw(Box::new(S1Bytes { data: bytes })),
        Err(e) => {
            set_error(error_out, &e.to_string());
            ptr::null_mut()
        }
    }
}

/// Get the document title from metadata.
///
/// Returns a non-null `S1String` if a title is set (caller must free with
/// `s1_string_free`), or null if no title or `doc` is null.
///
/// # Safety
///
/// `doc` must be a valid pointer or null.
#[no_mangle]
pub unsafe extern "C" fn s1_document_metadata_title(doc: *const S1Document) -> *mut S1String {
    if doc.is_null() {
        return ptr::null_mut();
    }
    let doc = unsafe { &*doc };
    match &doc.inner.metadata().title {
        Some(title) => match CString::new(title.as_str()) {
            Ok(cstr) => Box::into_raw(Box::new(S1String { value: cstr })),
            Err(_) => ptr::null_mut(),
        },
        None => ptr::null_mut(),
    }
}

/// Get the number of paragraphs in the document.
///
/// Returns 0 if `doc` is null.
///
/// # Safety
///
/// `doc` must be a valid pointer or null.
#[no_mangle]
pub unsafe extern "C" fn s1_document_paragraph_count(doc: *const S1Document) -> usize {
    if doc.is_null() {
        return 0;
    }
    let doc = unsafe { &*doc };
    doc.inner.paragraph_count()
}

// --- Error ---

/// Get the error message as a C string.
///
/// The returned pointer is valid until `s1_error_free` is called.
/// Returns null if `error` is null.
///
/// # Safety
///
/// `error` must be a valid pointer or null.
#[no_mangle]
pub unsafe extern "C" fn s1_error_message(error: *const S1Error) -> *const c_char {
    if error.is_null() {
        return ptr::null();
    }
    let error = unsafe { &*error };
    error.message.as_ptr()
}

/// Free an error.
///
/// # Safety
///
/// `error` must be a valid pointer returned by an API call, or null.
#[no_mangle]
pub unsafe extern "C" fn s1_error_free(error: *mut S1Error) {
    if !error.is_null() {
        drop(unsafe { Box::from_raw(error) });
    }
}

// --- String ---

/// Get the C string pointer from an `S1String`.
///
/// The returned pointer is valid until `s1_string_free` is called.
/// Returns null if `s` is null.
///
/// # Safety
///
/// `s` must be a valid pointer or null.
#[no_mangle]
pub unsafe extern "C" fn s1_string_ptr(s: *const S1String) -> *const c_char {
    if s.is_null() {
        return ptr::null();
    }
    let s = unsafe { &*s };
    s.value.as_ptr()
}

/// Free an `S1String`.
///
/// # Safety
///
/// `s` must be a valid pointer returned by an API call, or null.
#[no_mangle]
pub unsafe extern "C" fn s1_string_free(s: *mut S1String) {
    if !s.is_null() {
        drop(unsafe { Box::from_raw(s) });
    }
}

// --- Bytes ---

/// Get the data pointer from an `S1Bytes`.
///
/// The returned pointer is valid until `s1_bytes_free` is called.
/// Returns null if `b` is null.
///
/// # Safety
///
/// `b` must be a valid pointer or null.
#[no_mangle]
pub unsafe extern "C" fn s1_bytes_data(b: *const S1Bytes) -> *const u8 {
    if b.is_null() {
        return ptr::null();
    }
    let b = unsafe { &*b };
    b.data.as_ptr()
}

/// Get the length of an `S1Bytes`.
///
/// Returns 0 if `b` is null.
///
/// # Safety
///
/// `b` must be a valid pointer or null.
#[no_mangle]
pub unsafe extern "C" fn s1_bytes_len(b: *const S1Bytes) -> usize {
    if b.is_null() {
        return 0;
    }
    let b = unsafe { &*b };
    b.data.len()
}

/// Free an `S1Bytes`.
///
/// # Safety
///
/// `b` must be a valid pointer returned by an API call, or null.
#[no_mangle]
pub unsafe extern "C" fn s1_bytes_free(b: *mut S1Bytes) {
    if !b.is_null() {
        drop(unsafe { Box::from_raw(b) });
    }
}

// --- Helpers ---

fn parse_format(s: &str) -> Option<s1engine::Format> {
    match s.to_lowercase().as_str() {
        "docx" => Some(s1engine::Format::Docx),
        "odt" => Some(s1engine::Format::Odt),
        "pdf" => Some(s1engine::Format::Pdf),
        "txt" | "text" => Some(s1engine::Format::Txt),
        "doc" => Some(s1engine::Format::Doc),
        _ => None,
    }
}

/// Set error_out if it's non-null.
unsafe fn set_error(error_out: *mut *mut S1Error, msg: &str) {
    if !error_out.is_null() {
        let cstr = CString::new(msg).unwrap_or_else(|_| CString::new("unknown error").unwrap());
        unsafe {
            *error_out = Box::into_raw(Box::new(S1Error { message: cstr }));
        }
    }
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn c_engine_create_free() {
        unsafe {
            let engine = s1_engine_new();
            assert!(!engine.is_null());
            s1_engine_free(engine);
        }
    }

    #[test]
    fn c_document_create_free() {
        unsafe {
            let engine = s1_engine_new();
            let doc = s1_engine_create(engine);
            assert!(!doc.is_null());
            s1_document_free(doc);
            s1_engine_free(engine);
        }
    }

    #[test]
    fn c_document_plain_text() {
        unsafe {
            let engine = s1_engine_new();
            let doc = s1_engine_create(engine);
            let text = s1_document_plain_text(doc);
            assert!(!text.is_null());
            // Empty document should produce empty or whitespace-only text
            let ptr = s1_string_ptr(text);
            assert!(!ptr.is_null());
            s1_string_free(text);
            s1_document_free(doc);
            s1_engine_free(engine);
        }
    }

    #[test]
    fn c_document_open_docx() {
        unsafe {
            // Build a DOCX via the Rust API, then open via C FFI
            let builder = s1engine::DocumentBuilder::new();
            let doc_rust = builder.text("C FFI test").build();
            let docx_bytes = doc_rust
                .export(s1engine::Format::Docx)
                .expect("export failed");

            let engine = s1_engine_new();
            let mut error: *mut S1Error = ptr::null_mut();
            let doc = s1_engine_open(engine, docx_bytes.as_ptr(), docx_bytes.len(), &mut error);
            assert!(!doc.is_null(), "open failed");
            assert!(error.is_null());

            let text = s1_document_plain_text(doc);
            assert!(!text.is_null());
            let cstr = CStr::from_ptr(s1_string_ptr(text));
            let text_str = cstr.to_str().unwrap();
            assert!(text_str.contains("C FFI test"));

            s1_string_free(text);
            s1_document_free(doc);
            s1_engine_free(engine);
        }
    }

    #[test]
    fn c_document_export_docx() {
        unsafe {
            let engine = s1_engine_new();
            let doc = s1_engine_create(engine);
            let fmt = CString::new("docx").unwrap();
            let mut error: *mut S1Error = ptr::null_mut();
            let bytes = s1_document_export(doc, fmt.as_ptr(), &mut error);
            assert!(!bytes.is_null(), "export failed");
            assert!(error.is_null());

            let data = s1_bytes_data(bytes);
            let len = s1_bytes_len(bytes);
            assert!(len > 4);
            // Check ZIP magic bytes
            let slice = std::slice::from_raw_parts(data, len);
            assert_eq!(&slice[0..4], &[0x50, 0x4B, 0x03, 0x04]);

            s1_bytes_free(bytes);
            s1_document_free(doc);
            s1_engine_free(engine);
        }
    }

    #[test]
    fn c_document_metadata() {
        unsafe {
            // Build a doc with metadata via Rust, export, reopen via C FFI
            let builder = s1engine::DocumentBuilder::new();
            let doc_rust = builder.title("C Title").build();
            let docx_bytes = doc_rust
                .export(s1engine::Format::Docx)
                .expect("export failed");

            let engine = s1_engine_new();
            let mut error: *mut S1Error = ptr::null_mut();
            let doc = s1_engine_open(engine, docx_bytes.as_ptr(), docx_bytes.len(), &mut error);
            assert!(!doc.is_null());

            let title = s1_document_metadata_title(doc);
            assert!(!title.is_null());
            let cstr = CStr::from_ptr(s1_string_ptr(title));
            assert_eq!(cstr.to_str().unwrap(), "C Title");

            s1_string_free(title);
            s1_document_free(doc);
            s1_engine_free(engine);
        }
    }

    #[test]
    fn c_error_message() {
        unsafe {
            let engine = s1_engine_new();
            let mut error: *mut S1Error = ptr::null_mut();
            // Try to open invalid data
            let bad_data: [u8; 3] = [0xFF, 0xFE, 0xFD];
            let doc = s1_engine_open(engine, bad_data.as_ptr(), bad_data.len(), &mut error);
            // TXT reader is lenient, so this may succeed. Either way, no crash.
            if doc.is_null() {
                assert!(!error.is_null());
                let msg = s1_error_message(error);
                assert!(!msg.is_null());
                s1_error_free(error);
            } else {
                s1_document_free(doc);
            }
            s1_engine_free(engine);
        }
    }

    #[test]
    fn c_null_safety() {
        unsafe {
            // All functions should handle null gracefully
            s1_engine_free(ptr::null_mut());
            s1_document_free(ptr::null_mut());
            s1_error_free(ptr::null_mut());
            s1_string_free(ptr::null_mut());
            s1_bytes_free(ptr::null_mut());

            let doc = s1_engine_create(ptr::null());
            assert!(doc.is_null());

            let text = s1_document_plain_text(ptr::null());
            assert!(text.is_null());

            let count = s1_document_paragraph_count(ptr::null());
            assert_eq!(count, 0);

            assert!(s1_error_message(ptr::null()).is_null());
            assert!(s1_string_ptr(ptr::null()).is_null());
            assert!(s1_bytes_data(ptr::null()).is_null());
            assert_eq!(s1_bytes_len(ptr::null()), 0);
        }
    }

    #[test]
    fn c_double_free_safety() {
        // This test verifies the API doesn't crash on double-free.
        // NOTE: This is technically UB in Rust (double Box::from_raw), but
        // the null-check path is safe. We test that null-freeing is fine.
        unsafe {
            s1_engine_free(ptr::null_mut());
            s1_engine_free(ptr::null_mut());
            s1_document_free(ptr::null_mut());
            s1_document_free(ptr::null_mut());
        }
    }

    #[test]
    fn c_format_roundtrip() {
        unsafe {
            // Create doc via Rust builder → export DOCX via C FFI → reopen via C FFI
            let builder = s1engine::DocumentBuilder::new();
            let doc_rust = builder.text("Roundtrip content").build();
            let docx_bytes = doc_rust
                .export(s1engine::Format::Docx)
                .expect("export failed");

            let engine = s1_engine_new();
            let mut error: *mut S1Error = ptr::null_mut();

            // Open the DOCX
            let doc = s1_engine_open(engine, docx_bytes.as_ptr(), docx_bytes.len(), &mut error);
            assert!(!doc.is_null());

            // Export as TXT
            let fmt = CString::new("txt").unwrap();
            let bytes = s1_document_export(doc, fmt.as_ptr(), &mut error);
            assert!(!bytes.is_null());

            let data = s1_bytes_data(bytes);
            let len = s1_bytes_len(bytes);
            let txt = std::str::from_utf8(std::slice::from_raw_parts(data, len)).unwrap();
            assert!(txt.contains("Roundtrip content"));

            s1_bytes_free(bytes);
            s1_document_free(doc);
            s1_engine_free(engine);
        }
    }
}
