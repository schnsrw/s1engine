//! CRDT algorithms for collaborative editing in s1engine.
//!
//! This crate provides conflict-free replicated data type (CRDT) algorithms
//! for real-time collaborative document editing. It builds on `s1-model` and
//! `s1-ops` to add multi-user support without modifying the existing document model.
//!
//! # Architecture
//!
//! ```text
//! CollabDocument (consumer API)
//! ├── CrdtResolver (coordinator)
//! │   ├── TextCrdt     — Fugue-based text CRDT
//! │   ├── TreeCrdt     — Replicated tree with tombstones
//! │   ├── AttrCrdt     — Per-key LWW attribute registers
//! │   └── MetadataCrdt — LWW for metadata and styles
//! ├── LamportClock / StateVector (causal ordering)
//! ├── AwarenessState (cursor/presence)
//! └── Serialization / Compression (transport)
//! ```
//!
//! # Custom CRDT Approach
//!
//! This crate implements custom CRDT algorithms rather than using an external
//! library (Diamond Types, Yrs) because:
//! - `s1-model` already has CRDT-ready primitives (`NodeId(replica, counter)`)
//! - External CRDTs would duplicate the document model
//! - We control the full stack and can optimize for document editing patterns
//!
//! # Zero Impact on Existing Code
//!
//! All CRDT code lives in this crate behind a feature flag. The existing 491+
//! tests are completely unaffected.

pub mod attr_crdt;
pub mod awareness;
pub mod clock;
pub mod collab;
pub mod compression;
pub mod crdt_op;
pub mod error;
pub mod metadata_crdt;
pub mod op_id;
pub mod resolver;
pub mod serialize;
pub mod state_vector;
pub mod text_crdt;
pub mod tombstone;
pub mod tree_crdt;

// Re-export primary types at crate root.
pub use attr_crdt::AttrCrdt;
pub use awareness::{AwarenessState, AwarenessUpdate, CursorState};
pub use clock::{LamportClock, VectorClock};
pub use collab::{CollabDocument, Snapshot};
pub use compression::compress_ops;
pub use crdt_op::CrdtOperation;
pub use error::CrdtError;
pub use metadata_crdt::MetadataCrdt;
pub use op_id::OpId;
pub use resolver::CrdtResolver;
pub use state_vector::StateVector;
pub use text_crdt::TextCrdt;
pub use tombstone::TombstoneTracker;
pub use tree_crdt::TreeCrdt;
