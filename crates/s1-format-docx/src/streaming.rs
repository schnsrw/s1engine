//! Streaming document I/O traits and implementations.
//!
//! These traits define the interface for streaming document reading and writing,
//! enabling processing of large documents without loading the entire file into memory.
//!
//! # Current Status
//!
//! The streaming traits are defined and the chunked writer is implemented.
//! The default reader/writer still use in-memory buffers but can be replaced
//! with streaming implementations for large file handling.

use std::io::{Read, Write};

/// Streaming document reader trait.
///
/// Implementations can read document content in chunks rather than loading
/// the entire ZIP archive into memory at once.
pub trait StreamingReader {
    /// The error type for read operations.
    type Error: std::fmt::Display;

    /// Read a document from a byte stream.
    ///
    /// The implementation should process the document incrementally,
    /// yielding document model nodes as they are parsed.
    fn read_stream<R: Read>(&self, reader: R) -> Result<s1_model::DocumentModel, Self::Error>;

    /// Get the maximum memory budget for buffering (in bytes).
    fn max_buffer_size(&self) -> usize {
        256 * 1024 * 1024 // 256 MB default
    }
}

/// Streaming document writer trait.
///
/// Implementations write document content in chunks to a byte stream
/// rather than building the entire output in memory.
pub trait StreamingWriter {
    /// The error type for write operations.
    type Error: std::fmt::Display;

    /// Write a document to a byte stream.
    fn write_stream<W: Write>(
        &self,
        doc: &s1_model::DocumentModel,
        writer: W,
    ) -> Result<(), Self::Error>;
}

/// A chunked writer that writes XML in configurable chunk sizes.
///
/// This reduces peak memory by flushing to the output stream
/// periodically rather than building the entire XML string first.
pub struct ChunkedXmlWriter<W: Write> {
    inner: W,
    buffer: String,
    chunk_size: usize,
    bytes_written: usize,
}

impl<W: Write> ChunkedXmlWriter<W> {
    /// Create a new chunked writer with the given chunk size.
    pub fn new(writer: W, chunk_size: usize) -> Self {
        Self {
            inner: writer,
            buffer: String::with_capacity(chunk_size),
            chunk_size,
            bytes_written: 0,
        }
    }

    /// Append XML content. Flushes automatically when buffer exceeds chunk size.
    pub fn push_str(&mut self, s: &str) -> std::io::Result<()> {
        self.buffer.push_str(s);
        if self.buffer.len() >= self.chunk_size {
            self.flush()?;
        }
        Ok(())
    }

    /// Flush the current buffer to the underlying writer.
    pub fn flush(&mut self) -> std::io::Result<()> {
        if !self.buffer.is_empty() {
            self.inner.write_all(self.buffer.as_bytes())?;
            self.bytes_written += self.buffer.len();
            self.buffer.clear();
        }
        Ok(())
    }

    /// Finish writing and return the total bytes written.
    pub fn finish(mut self) -> std::io::Result<usize> {
        self.flush()?;
        Ok(self.bytes_written)
    }

    /// Get total bytes written so far (including unflushed buffer).
    pub fn total_bytes(&self) -> usize {
        self.bytes_written + self.buffer.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunked_writer_basic() {
        let mut output = Vec::new();
        let mut writer = ChunkedXmlWriter::new(&mut output, 16);

        writer.push_str("<root>").unwrap();
        writer.push_str("<child>Hello</child>").unwrap(); // This exceeds 16 bytes → auto flush
        writer.push_str("</root>").unwrap();

        let total = writer.finish().unwrap();
        let result = String::from_utf8(output).unwrap();
        assert_eq!(result, "<root><child>Hello</child></root>");
        assert_eq!(total, result.len());
    }

    #[test]
    fn chunked_writer_empty() {
        let mut output = Vec::new();
        let writer = ChunkedXmlWriter::new(&mut output, 64);
        let total = writer.finish().unwrap();
        assert_eq!(total, 0);
        assert!(output.is_empty());
    }

    #[test]
    fn chunked_writer_large_chunks() {
        let mut output = Vec::new();
        let mut writer = ChunkedXmlWriter::new(&mut output, 4);

        for i in 0..100 {
            writer.push_str(&format!("<n>{i}</n>")).unwrap();
        }
        writer.finish().unwrap();

        let result = String::from_utf8(output).unwrap();
        assert!(result.contains("<n>0</n>"));
        assert!(result.contains("<n>99</n>"));
    }
}
