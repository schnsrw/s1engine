# Testing

## Run Tests

```bash
# All tests
cargo test --workspace

# Single crate
cargo test -p s1-format-docx

# With output
cargo test -p s1-model -- --nocapture
```

## Test Structure

- Unit tests: `#[cfg(test)] mod tests` in each source file
- Integration tests: `tests/` directory in each crate
- Property-based tests: `proptest` in s1-model and s1-ops
- Round-trip tests: write → read → compare in format crates
- CRDT tests: convergence and error path tests in s1-crdt

## Writing Tests

```rust
#[test]
fn my_feature_works() {
    let mut doc = DocumentModel::new();
    let body_id = doc.body_id().unwrap();
    // ... setup ...
    assert_eq!(result, expected);
}
```

## Format Round-Trip Tests

```rust
#[test]
fn roundtrip_feature() {
    let mut doc = make_doc_with_feature();
    let bytes = write(&doc).unwrap();
    let doc2 = read(&bytes).unwrap();
    // Verify feature survived round-trip
    assert_eq!(doc2.feature(), doc.feature());
}
```

## Before Submitting

```bash
cargo test --workspace     # All tests pass
cargo clippy --workspace -- -D warnings  # No warnings
cargo fmt --check          # Formatting correct
```
