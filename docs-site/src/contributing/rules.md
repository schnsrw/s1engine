# Architecture Rules

These are enforced in code review:

## 1. Document Model is Sacred

`s1-model` has **zero external dependencies**. Every node must have a globally unique `NodeId(replica_id, counter)`. Never expose internal model representation in public API.

## 2. All Mutations Via Operations

Never modify the document tree directly. All changes go through `Operation` → applied via `s1-ops`. Every operation must implement `invert()` for undo.

## 3. Format Isolation

Each format crate only depends on `s1-model`. Format crates never depend on each other or on `s1-ops`/`s1-layout`.

## 4. No Panics in Library Code

All public functions return `Result<T, Error>`. No `.unwrap()` or `.expect()` in library code. Tests are fine.

## 5. No Unsafe

Unless absolutely necessary, with a documented `// SAFETY:` comment.

## 6. Editor UI Standards

- Clean, professional look (match Google Docs / Word Online)
- No emojis in UI
- Every button needs a `title` tooltip
- Use CSS variables, not hardcoded colors
