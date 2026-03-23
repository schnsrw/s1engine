# Phase 3: Production Scale (Performance & Features)

## Goal
Optimize the system for large documents (100+ pages) and large teams (20+ concurrent editors). Implement professional-grade features.

## Key Objectives

### 1. Binary Sync (Efficiency)
Base64 encoding document snapshots is 33% slower and heavier than raw bytes.
- **Action:** Implement Binary WebSocket frames with a custom header [4 bytes length][JSON Header][Raw Bytes].
- **Action:** Enable automatic fallback to Base64 if the environment doesn't support binary frames.

### 2. Structural CRDTs
Move beyond character-level sync.
- **Action:** Implement `SplitNode` and `MergeNode` operations in the `s1-crdt` crate.
- **Action:** Eliminate the need for `fullSync` (full document re-upload) when a user presses Enter or Backspace.

### 3. Authoritative Snapshots
- **Action:** Server maintains a "Live State" by applying operations to an in-memory document model.
- **Action:** New joiners receive a guaranteed-fresh snapshot from the server without relying on other peers.

### 4. Pro Features
- **Track Changes:** Native CRDT support for "Proposed" vs "Accepted" states in the tree.
- **Comments:** Threaded comments anchored to specific NodeIDs and text ranges.
- **Advanced Tables:** Multi-page table support with repeating header rows.
