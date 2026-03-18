//! Storage backend trait and implementations.
//!
//! Documents are stored as raw bytes with JSON metadata sidecars.
//! The [`StorageBackend`] trait abstracts over local filesystem, S3, etc.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Metadata stored alongside each document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMeta {
    pub id: String,
    pub filename: String,
    pub format: String,
    pub size: usize,
    pub title: Option<String>,
    pub author: Option<String>,
    pub word_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

/// Result type for storage operations.
pub type StorageResult<T> = Result<T, StorageError>;

/// Errors from storage operations.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Document not found: {0}")]
    NotFound(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Storage full")]
    #[allow(dead_code)]
    StorageFull,
}

/// Abstract storage backend for documents.
pub trait StorageBackend: Send + Sync {
    /// Store a document with metadata.
    fn put(&self, id: &str, data: &[u8], meta: &DocumentMeta) -> StorageResult<()>;

    /// Get document bytes by ID.
    fn get(&self, id: &str) -> StorageResult<Vec<u8>>;

    /// Get document metadata by ID.
    fn get_meta(&self, id: &str) -> StorageResult<DocumentMeta>;

    /// Delete a document by ID.
    fn delete(&self, id: &str) -> StorageResult<()>;

    /// List all document metadata, sorted by updated_at descending.
    fn list(&self, offset: usize, limit: usize) -> StorageResult<(Vec<DocumentMeta>, usize)>;

    /// Check if a document exists.
    #[allow(dead_code)]
    fn exists(&self, id: &str) -> bool;
}

// ─── Local Filesystem Storage ────────────────────────

/// Stores documents as files on the local filesystem.
///
/// Layout:
/// ```text
/// {data_dir}/
///   {doc_id}.bin       — document bytes
///   {doc_id}.meta.json — metadata sidecar
/// ```
pub struct LocalStorage {
    data_dir: PathBuf,
}

impl LocalStorage {
    /// Create a new local storage backend. Creates the directory if it doesn't exist.
    pub fn new(data_dir: impl Into<PathBuf>) -> StorageResult<Self> {
        let dir = data_dir.into();
        std::fs::create_dir_all(&dir)?;
        Ok(Self { data_dir: dir })
    }

    fn doc_path(&self, id: &str) -> PathBuf {
        self.data_dir.join(format!("{id}.bin"))
    }

    fn meta_path(&self, id: &str) -> PathBuf {
        self.data_dir.join(format!("{id}.meta.json"))
    }
}

impl StorageBackend for LocalStorage {
    fn put(&self, id: &str, data: &[u8], meta: &DocumentMeta) -> StorageResult<()> {
        std::fs::write(self.doc_path(id), data)?;
        let meta_json = serde_json::to_string_pretty(meta)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        std::fs::write(self.meta_path(id), meta_json)?;
        Ok(())
    }

    fn get(&self, id: &str) -> StorageResult<Vec<u8>> {
        let path = self.doc_path(id);
        if !path.exists() {
            return Err(StorageError::NotFound(id.to_string()));
        }
        Ok(std::fs::read(path)?)
    }

    fn get_meta(&self, id: &str) -> StorageResult<DocumentMeta> {
        let path = self.meta_path(id);
        if !path.exists() {
            return Err(StorageError::NotFound(id.to_string()));
        }
        let json = std::fs::read_to_string(path)?;
        serde_json::from_str(&json).map_err(|e| StorageError::Serialization(e.to_string()))
    }

    fn delete(&self, id: &str) -> StorageResult<()> {
        let doc_path = self.doc_path(id);
        let meta_path = self.meta_path(id);
        if !doc_path.exists() {
            return Err(StorageError::NotFound(id.to_string()));
        }
        std::fs::remove_file(doc_path)?;
        let _ = std::fs::remove_file(meta_path); // Meta might not exist
        Ok(())
    }

    fn list(&self, offset: usize, limit: usize) -> StorageResult<(Vec<DocumentMeta>, usize)> {
        let mut metas = Vec::new();

        for entry in std::fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json")
                && path.to_string_lossy().ends_with(".meta.json")
            {
                if let Ok(json) = std::fs::read_to_string(&path) {
                    if let Ok(meta) = serde_json::from_str::<DocumentMeta>(&json) {
                        metas.push(meta);
                    }
                }
            }
        }

        // Sort by updated_at descending
        metas.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        let total = metas.len();
        let page = metas.into_iter().skip(offset).take(limit).collect();
        Ok((page, total))
    }

    fn exists(&self, id: &str) -> bool {
        self.doc_path(id).exists()
    }
}

// ─── In-Memory Storage (for testing) ─────────────────

/// In-memory storage backend for testing.
pub struct MemoryStorage {
    docs: std::sync::Mutex<std::collections::HashMap<String, (Vec<u8>, DocumentMeta)>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            docs: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl StorageBackend for MemoryStorage {
    fn put(&self, id: &str, data: &[u8], meta: &DocumentMeta) -> StorageResult<()> {
        let mut docs = self.docs.lock().unwrap();
        docs.insert(id.to_string(), (data.to_vec(), meta.clone()));
        Ok(())
    }

    fn get(&self, id: &str) -> StorageResult<Vec<u8>> {
        let docs = self.docs.lock().unwrap();
        docs.get(id)
            .map(|(data, _)| data.clone())
            .ok_or_else(|| StorageError::NotFound(id.to_string()))
    }

    fn get_meta(&self, id: &str) -> StorageResult<DocumentMeta> {
        let docs = self.docs.lock().unwrap();
        docs.get(id)
            .map(|(_, meta)| meta.clone())
            .ok_or_else(|| StorageError::NotFound(id.to_string()))
    }

    fn delete(&self, id: &str) -> StorageResult<()> {
        let mut docs = self.docs.lock().unwrap();
        docs.remove(id)
            .ok_or_else(|| StorageError::NotFound(id.to_string()))?;
        Ok(())
    }

    fn list(&self, offset: usize, limit: usize) -> StorageResult<(Vec<DocumentMeta>, usize)> {
        let docs = self.docs.lock().unwrap();
        let mut metas: Vec<_> = docs.values().map(|(_, m)| m.clone()).collect();
        metas.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        let total = metas.len();
        let page = metas.into_iter().skip(offset).take(limit).collect();
        Ok((page, total))
    }

    fn exists(&self, id: &str) -> bool {
        self.docs.lock().unwrap().contains_key(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_meta(id: &str) -> DocumentMeta {
        DocumentMeta {
            id: id.to_string(),
            filename: format!("{id}.docx"),
            format: "docx".to_string(),
            size: 100,
            title: Some("Test".to_string()),
            author: None,
            word_count: 10,
            created_at: "2026-03-18T00:00:00Z".to_string(),
            updated_at: "2026-03-18T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn memory_storage_crud() {
        let storage = MemoryStorage::new();
        let meta = make_meta("doc1");

        // Put
        storage.put("doc1", b"hello", &meta).unwrap();
        assert!(storage.exists("doc1"));

        // Get
        let data = storage.get("doc1").unwrap();
        assert_eq!(data, b"hello");

        // Get meta
        let m = storage.get_meta("doc1").unwrap();
        assert_eq!(m.id, "doc1");

        // List
        let (list, total) = storage.list(0, 10).unwrap();
        assert_eq!(total, 1);
        assert_eq!(list[0].id, "doc1");

        // Delete
        storage.delete("doc1").unwrap();
        assert!(!storage.exists("doc1"));
        assert!(storage.get("doc1").is_err());
    }

    #[test]
    fn local_storage_crud() {
        let dir = std::env::temp_dir().join("s1_test_storage");
        let _ = std::fs::remove_dir_all(&dir);
        let storage = LocalStorage::new(&dir).unwrap();
        let meta = make_meta("doc2");

        storage.put("doc2", b"world", &meta).unwrap();
        assert!(storage.exists("doc2"));

        let data = storage.get("doc2").unwrap();
        assert_eq!(data, b"world");

        let m = storage.get_meta("doc2").unwrap();
        assert_eq!(m.filename, "doc2.docx");

        storage.delete("doc2").unwrap();
        assert!(!storage.exists("doc2"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
