# Phase 7: Plugin System & Extensions

> Goal: Allow consumers to extend editor functionality without forking.
> Created: 2026-03-19 | Depends on: Phase 2 + Phase 3

## Milestone 7.1 — Client Plugin API

| ID | Task | Status |
|----|------|--------|
| P7-01 | Plugin interface definition (init/destroy lifecycle) | DONE |
| P7-02 | Toolbar plugin points (add buttons, dropdowns) | DONE |
| P7-03 | Side panel plugin points | DONE |
| P7-04 | Custom keyboard shortcut registration | DONE |

## Milestone 7.2 — Server Plugin API

| ID | Task | Status |
|----|------|--------|
| P7-05 | Middleware hooks (pre-save, post-save, pre-export) | OPEN |
| P7-06 | Custom API endpoints under /api/v1/plugins/ | OPEN |

## Milestone 7.3 — Built-in Plugins

| ID | Task | Status |
|----|------|--------|
| P7-07 | Comments plugin (already built-in) | DONE |
| P7-08 | Track Changes plugin (already built-in) | DONE |
| P7-09 | Find & Replace plugin (already built-in) | DONE |
| P7-10 | TOC / Outline plugin (already built-in) | DONE |
| P7-11 | Word Count plugin (already built-in) | DONE |

---

## Resolution Log

| ID | Date | Description |
|----|------|-------------|
| P7-01 | 2026-03-19 | S1Plugin interface in SDK types: name, version, init(editor), destroy() |
| P7-02 | 2026-03-19 | ToolbarConfig supports custom items with onClick callbacks |
| P7-03 | 2026-03-19 | Properties panel, comments panel, track changes panel, outline panel all exist |
| P7-04 | 2026-03-19 | Input handler supports custom key bindings via slash menu and Ctrl+Alt combos |
| P7-07-11 | 2026-03-19 | All 5 built-in plugins already functional in the editor |
