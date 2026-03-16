//! Core document model for s1engine.
//!
//! This crate defines the fundamental data structures for representing documents:
//! nodes, attributes, styles, metadata, and the document tree. It has **zero external
//! dependencies** — only pure Rust data structures.
//!
//! # Architecture
//!
//! A document is a tree of [`Node`]s, each identified by a globally unique [`NodeId`].
//! The tree is stored in a flat [`DocumentModel`] container with O(1) node lookup.
//!
//! ```text
//! Document
//! └── Body
//!     ├── Paragraph
//!     │   ├── Run { bold: true }
//!     │   │   └── Text "Hello "
//!     │   └── Run { italic: true }
//!     │       └── Text "world"
//!     └── Table
//!         └── ...
//! ```
//!
//! # CRDT-Ready Design
//!
//! Every node has a [`NodeId`] composed of `(replica_id, counter)`. For single-user
//! mode, `replica_id` is `0`. For collaborative editing, each user gets a unique
//! replica ID, ensuring IDs never collide across users.

pub mod attributes;
pub mod id;
pub mod media;
pub mod metadata;
pub mod node;
pub mod numbering;
pub mod section;
pub mod styles;
pub mod tree;

// Re-export primary types at crate root for convenience.
pub use attributes::{
    Alignment, AttributeKey, AttributeMap, AttributeValue, BorderSide, BorderStyle, Borders, Color,
    FieldType, LineSpacing, ListFormat, ListInfo, MediaId, PageOrientation, TabAlignment,
    TabLeader, TabStop, TableWidth, UnderlineStyle, VerticalAlignment,
};
pub use id::{IdGenerator, NodeId};
pub use media::{MediaItem, MediaStore};
pub use metadata::DocumentMetadata;
pub use node::{Node, NodeType};
pub use numbering::{
    AbstractNumbering, LevelOverride, NumberingDefinitions, NumberingInstance, NumberingLevel,
};
pub use section::{HeaderFooterRef, HeaderFooterType, SectionBreakType, SectionProperties};
pub use styles::{Style, StyleType};
pub use tree::{DocumentDefaults, DocumentModel, ModelError};
