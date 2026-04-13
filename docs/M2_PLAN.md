# M2 Completion Plan — Serve web/ + Fix Open/Save

## Current State

- Open: uses `to_plain_text()` → split by \n → AddToParagraph per char
- Save: uses `GetSelectedText(false, ...)` with fallback manual extraction
- Server: `./web` added to static dir candidates (done)

## Known Bugs

### Bug 1: Save returns empty text
- `GetSelectedText(false, opts)` returns empty string
- Root cause hypothesis: when called with `false` as first param AND no active selection,
  GetSelectedText may not work as expected in this sdkjs build
- Evidence: adapter log showed "Text length: 0" even when document had 13K chars visible
- Fix plan: Use Paragraph.GetText() per-paragraph instead of document-level GetSelectedText
- Verification: Paragraph.GetText at Paragraph.js:16730 — confirmed to exist
- Code path: `logicDoc.Content[i].GetText({ParaSeparator: ''})` for each IsParagraph() element

### Bug 2: Document sometimes appears empty after loading
- 13,380 chars were inserted (confirmed by totalChars counter)
- Recalculate() is called after insertion
- Hypothesis: the editor IS rendering but user may not see it due to:
  a) Scroll position not at top after recalculate
  b) Canvas not repainting properly
- Evidence: user said "more than 1 min" — could be recalculate taking long on 13K chars
- Fix plan: Add `api.WordControl.m_oDrawingDocument.OnStartRecalculate(0)` before Recalculate

### Bug 3: Slow loading (32s for 72 paragraphs)
- Character-by-character AddToParagraph is O(n) where n = total chars
- Each AddToParagraph goes through Controller → finds paragraph → inserts → tracks
- Fix plan: Use TurnOff_Recalculate before loop, TurnOn after
- Risk: AddToParagraph may silently fail when recalc is off
- Mitigation: Test with a small doc first, verify char count matches
- Alternative if TurnOff fails: Build ParaRun directly, add to paragraph Content

## Execution Order

1. Fix save (Bug 1) — switch to per-paragraph GetText()
2. Test save works — verify non-empty DOCX output
3. Test open still works — verify text appears
4. Try TurnOff_Recalculate for speed (Bug 3)
5. If speed fix breaks rendering, revert it
6. Verify server serves web/ correctly
7. Commit

## Lines to Change

### adapter.js save function (line 67+):

BEFORE:
```javascript
var allText = logicDoc.GetSelectedText(false, { ParaSeparator: '\n', ... });
```

AFTER:
```javascript
var paragraphs = [];
for (var i = 0; i < logicDoc.Content.length; i++) {
  var el = logicDoc.Content[i];
  if (el && el.IsParagraph && el.IsParagraph()) {
    paragraphs.push(el.GetText({ ParaSeparator: '' }) || '');
  }
}
```

### adapter.js open function — speed optimization:

BEFORE (line 40-60):
```javascript
// No recalc control
var isFirst = true;
for (...)
```

AFTER:
```javascript
logicDoc.TurnOff_Recalculate();
var isFirst = true;
for (...)
// after loop:
logicDoc.TurnOn_Recalculate(false);
logicDoc.Recalculate();
```

NOTE: If TurnOff breaks AddToParagraph, revert to without it.
