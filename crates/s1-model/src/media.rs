//! Embedded media storage (images, etc.).

use crate::attributes::MediaId;
use std::collections::HashMap;

/// An embedded media item (image, etc.).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct MediaItem {
    pub id: MediaId,
    /// MIME type (e.g., "image/png", "image/jpeg").
    pub content_type: String,
    /// Raw bytes of the media content.
    pub data: Vec<u8>,
    /// Original filename, if known.
    pub filename: Option<String>,
}

/// Storage for embedded media, keyed by [`MediaId`].
/// Deduplicates content by hash.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct MediaStore {
    items: HashMap<MediaId, MediaItem>,
    /// Maps content hash to MediaId for deduplication.
    hash_to_id: HashMap<u64, MediaId>,
    next_id: u64,
}

impl MediaStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert media. If identical content already exists, returns the existing [`MediaId`].
    pub fn insert(
        &mut self,
        content_type: impl Into<String>,
        data: Vec<u8>,
        filename: Option<String>,
    ) -> MediaId {
        let hash = simple_hash(&data);

        // Dedup: return existing ID if content already stored
        if let Some(&existing_id) = self.hash_to_id.get(&hash) {
            return existing_id;
        }

        let id = MediaId(self.next_id);
        self.next_id += 1;

        let item = MediaItem {
            id,
            content_type: content_type.into(),
            data,
            filename,
        };

        self.hash_to_id.insert(hash, id);
        self.items.insert(id, item);
        id
    }

    /// Get media by ID.
    pub fn get(&self, id: MediaId) -> Option<&MediaItem> {
        self.items.get(&id)
    }

    /// Remove media by ID.
    pub fn remove(&mut self, id: MediaId) -> Option<MediaItem> {
        if let Some(item) = self.items.remove(&id) {
            let hash = simple_hash(&item.data);
            self.hash_to_id.remove(&hash);
            Some(item)
        } else {
            None
        }
    }

    /// Number of stored media items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if no media is stored.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Iterate over all media items.
    pub fn iter(&self) -> impl Iterator<Item = &MediaItem> {
        self.items.values()
    }
}

/// Simple hash for deduplication. Not cryptographic.
fn simple_hash(data: &[u8]) -> u64 {
    // FNV-1a hash
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get() {
        let mut store = MediaStore::new();
        let id = store.insert("image/png", vec![1, 2, 3, 4], Some("test.png".into()));
        assert_eq!(store.len(), 1);

        let item = store.get(id).unwrap();
        assert_eq!(item.content_type, "image/png");
        assert_eq!(item.data, vec![1, 2, 3, 4]);
        assert_eq!(item.filename.as_deref(), Some("test.png"));
    }

    #[test]
    fn deduplication() {
        let mut store = MediaStore::new();
        let data = vec![10, 20, 30];
        let id1 = store.insert("image/png", data.clone(), None);
        let id2 = store.insert("image/png", data, None);
        assert_eq!(id1, id2); // Same content → same ID
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn different_content_different_id() {
        let mut store = MediaStore::new();
        let id1 = store.insert("image/png", vec![1, 2, 3], None);
        let id2 = store.insert("image/jpeg", vec![4, 5, 6], None);
        assert_ne!(id1, id2);
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn remove_media() {
        let mut store = MediaStore::new();
        let id = store.insert("image/png", vec![1, 2, 3], None);
        assert_eq!(store.len(), 1);

        let removed = store.remove(id);
        assert!(removed.is_some());
        assert_eq!(store.len(), 0);
        assert!(store.get(id).is_none());
    }

    #[test]
    fn empty_store() {
        let store = MediaStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert!(store.get(MediaId(0)).is_none());
    }
}
