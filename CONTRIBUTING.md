# Contributing to s1engine

Thank you for your interest in contributing to s1engine. This document provides guidelines and information for contributors.

## Getting Started

### Prerequisites

- **Rust 1.88+** — Install via [rustup](https://rustup.rs/)
- **wasm-pack** — For WASM builds: `cargo install wasm-pack`
- **Node.js 18+** — For the editor and relay server
- **Git** — For version control

### Setup

```bash
# Clone the repository
git clone https://github.com/schnsrw/s1engine.git
cd s1engine

# Build and test
cargo build --workspace
cargo test --workspace

# (Optional) Build WASM and start the editor
make wasm
cd editor && npm install && npm run dev
```

### Verify Your Setup

```bash
make check  # Runs formatting, clippy, and all tests
```

## Development Workflow

### 1. Find or Create an Issue

- Check existing issues before starting work
- For new features, open an issue first to discuss the approach
- Bug fixes can go directly to a PR

### 2. Create a Branch

```bash
git checkout -b feature/my-feature  # or fix/my-bugfix
```

### 3. Make Changes

Follow the coding conventions below. Key rules:

- All mutations go through the `Operation` type (never modify the document tree directly)
- `s1-model` has **zero external dependencies** — do not add any
- Format crates only depend on `s1-model`, never on each other
- No `.unwrap()` or `.expect()` in library code (tests are fine)
- Every public function needs at least one test

### 4. Test Your Changes

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p s1-format-docx

# Lint
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --check
```

### 5. Submit a Pull Request

- Write a clear PR description explaining what and why
- Reference any related issues
- Make sure CI passes (tests + clippy + fmt)
- Keep PRs focused — one feature or fix per PR

## Architecture Rules

These are non-negotiable and will be enforced in code review:

1. **Document model is sacred** — `s1-model` has zero external dependencies. Every node must have a globally unique `NodeId(replica_id, counter)`.

2. **All mutations via operations** — Never modify the document tree directly. All changes go through `Operation` applied via `s1-ops`. Every operation must implement `invert()` for undo.

3. **Format isolation** — Each format crate (`s1-format-docx`, `s1-format-odt`, etc.) only depends on `s1-model`. Format crates never depend on each other or on `s1-ops`/`s1-layout`.

4. **No panics in library code** — All public functions return `Result<T, Error>`. Use `thiserror` for error types. Errors must include context (file position, node ID, format element).

5. **No unsafe** — Unless absolutely necessary, and documented with a `// SAFETY:` comment explaining why.

## Coding Conventions

### Rust

- Follow standard Rust style (`cargo fmt`, `cargo clippy`)
- `snake_case` for functions/modules, `PascalCase` for types, `SCREAMING_SNAKE` for constants
- `&str` over `String` in function parameters
- `impl Into<String>` for builder methods that take ownership
- Derive `Debug, Clone, PartialEq` on all public types
- `#[non_exhaustive]` on public enums
- `///` doc comments on all public items with `# Errors` and `# Panics` sections

### Testing

- Every public function needs at least one test
- Format crates need round-trip tests (read -> write -> read -> compare)
- Test fixtures go in `tests/fixtures/`
- Use `proptest` for property-based testing on `s1-model` and `s1-ops`

### Editor (JavaScript)

- ES modules, no bundler plugins beyond Vite
- Clean, professional UI — no bright colors, no emojis in UI
- Every toolbar button must have a `title` tooltip
- Match the look of Google Docs / Word Online

## Crate Structure

```
crates/
  s1-model/          Core document model (ZERO external deps)
  s1-ops/            Operations, transactions, undo/redo
  s1-format-docx/    DOCX reader/writer
  s1-format-odt/     ODT reader/writer
  s1-format-pdf/     PDF export + editing
  s1-format-txt/     Plain text reader/writer
  s1-format-md/      Markdown reader/writer
  s1-convert/        Format conversion pipelines
  s1-layout/         Page layout engine
  s1-text/           Text processing (pure Rust)
  s1-crdt/           CRDT for collaboration
  s1engine/          Facade crate (public API)
ffi/
  wasm/              WASM bindings
  c/                 C FFI bindings
editor/              S1 editor web editor
scripts/             Build and utility scripts
docs/                Technical documentation
```

## Reporting Bugs

- Use GitHub Issues
- Include: steps to reproduce, expected behavior, actual behavior
- For document format bugs: attach a minimal test file if possible
- For editor bugs: include browser and OS version

## License

By contributing, you agree that your contributions will be licensed under the same AGPL-3.0-or-later license as the project.
