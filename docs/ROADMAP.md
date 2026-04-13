# Development Roadmap

## Engine (COMPLETE)

```
Phase 0: Planning           ████████████████████  COMPLETE
Phase 1: Foundation         ████████████████████  COMPLETE
Phase 2: Rich Documents     ████████████████████  COMPLETE
Phase 3: Layout & Export    ████████████████████  COMPLETE
Phase 4: Collaboration      ████████████████████  COMPLETE
Phase 5: Production Ready   ████████████████████  COMPLETE (WASM + C FFI)
Phase 6-12: Formats/Layout  ████████████████████  COMPLETE (1,589 tests passing)
```

The s1engine document engine is complete and production-ready.

## Web Editor (IN PROGRESS)

**Approach**: Integrate OnlyOffice Web as the editor shell and input/rendering runtime, with `s1engine-wasm` providing document import/export and future structural interoperability.

```
Phase 16: OnlyOffice Integration  ░░░░░░░░░░░░░░░░░░░░  IN PROGRESS
```

### Phase 16: OnlyOffice Web + s1engine WASM

**Current Goal**: Stand up a working OnlyOffice-based web editor path, then expand the bridge from text-only DOCX handling into structural fidelity.

Current status:
1. `web/` hosts the active web client
2. `web/adapter.js` provides a text-only DOCX open/save bridge
3. `web/pkg/` contains generated `s1engine-wasm` artifacts

Next required steps:
1. Serve `web/` by default from the Axum server
2. Replace text-only import with structural import
3. Replace text-only export with structural export
4. Define model ownership for editing and undo/redo
5. Rebuild fidelity validation around the current `web/` architecture
6. Integrate collaboration only after the model boundary is clear
