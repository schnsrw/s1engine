// Find & Replace
import { state, $ } from './state.js';
import { renderDocument, syncAllText } from './render.js';
import { updateUndoRedo, recordUndoAction } from './toolbar.js';
import { broadcastOp } from './collab.js';

let _findRefreshTimer = null;
let _matchCase = false;
let _wholeWord = false;
let _useRegex = false;       // FS-08: Regex toggle
let _findInSelection = false; // FS-14: Find in selection toggle
let _selectionRange = null;   // FS-14: Saved selection range for find-in-selection
let _findInitialized = false;

/** FS-41: Update the visible count display AND the aria-live region for screen readers. */
function setFindCountText(text) {
  $('findCount').textContent = text;
  const live = $('findCountLive');
  if (live) live.textContent = text;
}

export function initFind() {
  // ED2-08: Prevent duplicate listener registration on reinit
  if (_findInitialized) return;
  _findInitialized = true;

  // E1.5: Register callback so render.js can trigger find refresh without circular import
  state._onTextChanged = refreshFindIfOpen;

  $('btnFind').addEventListener('click', () => {
    $('findBar').classList.add('show');
    $('findInput').focus();
  });

  $('findClose').addEventListener('click', () => {
    closeFindBar();
  });

  $('findInput').addEventListener('input', () => doFind());
  $('findNext').addEventListener('click', () => navigateMatch(1));
  $('findPrev').addEventListener('click', () => navigateMatch(-1));

  $('replaceBtn').addEventListener('click', () => doReplace());
  $('replaceAllBtn').addEventListener('click', () => doReplaceAll());

  // Match case toggle
  $('findMatchCase').addEventListener('click', () => {
    _matchCase = !_matchCase;
    $('findMatchCase').classList.toggle('active', _matchCase);
    doFind();
  });

  // Whole word toggle
  $('findWholeWord').addEventListener('click', () => {
    _wholeWord = !_wholeWord;
    $('findWholeWord').classList.toggle('active', _wholeWord);
    doFind();
  });

  // FS-08: Regex toggle
  if ($('findRegex')) {
    $('findRegex').addEventListener('click', () => {
      _useRegex = !_useRegex;
      $('findRegex').classList.toggle('active', _useRegex);
      doFind();
    });
  }

  // FS-14: Find in selection toggle
  if ($('findInSelection')) {
    $('findInSelection').addEventListener('click', () => {
      _findInSelection = !_findInSelection;
      $('findInSelection').classList.toggle('active', _findInSelection);
      if (_findInSelection) {
        // Capture the current selection range
        _selectionRange = captureSelectionRange();
      } else {
        _selectionRange = null;
      }
      doFind();
    });
  }

  // FS-14: Replace preview — highlight replace target on replaceInput focus/input
  if ($('replaceInput')) {
    $('replaceInput').addEventListener('input', () => showReplacePreview());
    $('replaceInput').addEventListener('focus', () => showReplacePreview());
    $('replaceInput').addEventListener('blur', () => clearReplacePreview());
  }

  // Escape to close, Tab to cycle within find bar
  const findBarKeydown = e => {
    if (e.key === 'Escape') { closeFindBar(); }
    if (e.key === 'Tab') {
      e.preventDefault();
      const focusable = $('findBar').querySelectorAll('input, button');
      const idx = Array.from(focusable).indexOf(document.activeElement);
      const next = e.shiftKey ? (idx - 1 + focusable.length) % focusable.length : (idx + 1) % focusable.length;
      focusable[next].focus();
    }
  };
  $('findInput').addEventListener('keydown', e => {
    findBarKeydown(e);
    if (e.key === 'Enter') navigateMatch(e.shiftKey ? -1 : 1);
    // Alt+C = toggle match case, Alt+W = toggle whole word, Alt+R = toggle regex, Alt+S = toggle find in selection
    if (e.altKey && e.key === 'c') { e.preventDefault(); $('findMatchCase').click(); }
    if (e.altKey && e.key === 'w') { e.preventDefault(); $('findWholeWord').click(); }
    if (e.altKey && e.key === 'r') { e.preventDefault(); if ($('findRegex')) $('findRegex').click(); }
    if (e.altKey && e.key === 's') { e.preventDefault(); if ($('findInSelection')) $('findInSelection').click(); }
  });
  $('replaceInput').addEventListener('keydown', e => {
    findBarKeydown(e);
    if (e.key === 'Enter') doReplace();
  });
}

/**
 * FS-14: Capture the current document selection as node range for find-in-selection.
 */
function captureSelectionRange() {
  const sel = window.getSelection();
  if (!sel || sel.isCollapsed || sel.rangeCount === 0) return null;
  const range = sel.getRangeAt(0);
  let startEl = range.startContainer;
  while (startEl && !startEl.dataset?.nodeId) startEl = startEl.parentElement;
  let endEl = range.endContainer;
  while (endEl && !endEl.dataset?.nodeId) endEl = endEl.parentElement;
  if (!startEl?.dataset?.nodeId || !endEl?.dataset?.nodeId) return null;
  return {
    startNodeId: startEl.dataset.nodeId,
    endNodeId: endEl.dataset.nodeId,
  };
}

/**
 * FS-08: Build a RegExp from the query, with flags from current options.
 * Returns null if the query is not a valid regex.
 */
function buildRegex(query) {
  const flags = _matchCase ? 'g' : 'gi';
  try {
    return new RegExp(query, flags);
  } catch (e) {
    return null;
  }
}

/**
 * FS-08: Show or hide regex error feedback.
 */
function setRegexError(msg) {
  const el = $('findRegexError');
  if (!el) return;
  if (msg) {
    el.textContent = msg;
    el.title = msg;
    el.style.display = 'inline';
  } else {
    el.textContent = '';
    el.title = '';
    el.style.display = 'none';
  }
}

function doFind() {
  clearHighlights();
  clearReplacePreview();
  setRegexError(null);
  let query = $('findInput').value;
  if (!query || !state.doc) { setFindCountText(''); return; }

  syncAllText();

  // FS-08: Regex mode — validate pattern
  if (_useRegex) {
    const flags = _matchCase ? 'g' : 'gi';
    try {
      new RegExp(query, flags);
    } catch (e) {
      setRegexError(e.message);
      setFindCountText('Invalid regex');
      state.findMatches = [];
      state.findIndex = -1;
      return;
    }
  }

  const caseSensitive = _matchCase;

  // FS-14: Determine which node IDs are in selection (if find-in-selection is active)
  // Re-capture the selection range on each doFind() call so it stays current
  let selectionNodeIds = null;
  if (_findInSelection) {
    const freshRange = captureSelectionRange();
    if (freshRange) _selectionRange = freshRange;
    if (_selectionRange) {
      selectionNodeIds = getNodeIdsBetween(_selectionRange.startNodeId, _selectionRange.endNodeId);
    }
    // If no valid selection range, search entire document instead of using stale range
    if (!selectionNodeIds || selectionNodeIds.length === 0) {
      selectionNodeIds = null;
    }
  }

  // FS-08: If regex mode, use DOM-based regex search instead of WASM find_text
  if (_useRegex) {
    regexFind(query, selectionNodeIds);
    showReplacePreview();
    return;
  }

  try {
    const results = JSON.parse(state.doc.find_text(query, caseSensitive));

    // Filter for whole word matches if enabled
    let filtered = results;
    if (_wholeWord) {
      filtered = results.filter(m => {
        const page = $('pageContainer');
        const el = page.querySelector(`[data-node-id="${m.nodeId}"]`);
        if (!el) return true;
        const text = el.textContent || '';
        const chars = Array.from(text);
        const before = m.offset > 0 ? chars[m.offset - 1] : ' ';
        const after = m.offset + m.length < chars.length ? chars[m.offset + m.length] : ' ';
        return /\W/.test(before) && /\W/.test(after);
      });
    }

    // FS-14: Filter by selection range
    if (selectionNodeIds) {
      filtered = filtered.filter(m => selectionNodeIds.has(m.nodeId));
    }

    state.findMatches = filtered;
    state.findIndex = filtered.length > 0 ? 0 : -1;
    setFindCountText(filtered.length + ' match' + (filtered.length !== 1 ? 'es' : ''));

    // Highlight matches in DOM
    filtered.forEach((m, i) => {
      highlightMatch(m, i === state.findIndex);
    });
  } catch (_) {
    // find_text may not be available — fall back to DOM search
    domFind(query);
  }

  showReplacePreview();
}

/**
 * FS-08: Regex-based find across document paragraphs.
 * Searches the text content of each paragraph element using a regex pattern.
 */
function regexFind(query, selectionNodeIds) {
  const flags = _matchCase ? 'g' : 'gi';
  let regex;
  try {
    regex = new RegExp(query, flags);
  } catch (e) {
    setRegexError(e.message);
    state.findMatches = [];
    state.findIndex = -1;
    setFindCountText('Invalid regex');
    return;
  }

  const page = $('pageContainer');
  if (!page) return;

  const results = [];
  const paragraphs = page.querySelectorAll('[data-node-id]');
  for (const para of paragraphs) {
    const nodeId = para.dataset.nodeId;
    if (selectionNodeIds && !selectionNodeIds.has(nodeId)) continue;
    const text = para.textContent || '';
    const chars = Array.from(text);
    // Build a map from string index to codepoint index
    const strToCp = new Array(text.length + 1);
    let cpIdx = 0;
    let strIdx = 0;
    for (const ch of chars) {
      strToCp[strIdx] = cpIdx;
      strIdx += ch.length;
      cpIdx++;
    }
    strToCp[strIdx] = cpIdx;

    let match;
    regex.lastIndex = 0;
    while ((match = regex.exec(text)) !== null) {
      const cpOffset = strToCp[match.index] !== undefined ? strToCp[match.index] : 0;
      const matchEndStr = match.index + match[0].length;
      const cpEnd = strToCp[matchEndStr] !== undefined ? strToCp[matchEndStr] : cpOffset;
      const cpLen = cpEnd - cpOffset;

      if (_wholeWord) {
        const before = cpOffset > 0 ? chars[cpOffset - 1] : ' ';
        const after = cpEnd < chars.length ? chars[cpEnd] : ' ';
        if (!/\W/.test(before) || !/\W/.test(after)) continue;
      }

      results.push({ nodeId, offset: cpOffset, length: cpLen });

      // Prevent infinite loop on zero-length matches
      if (match[0].length === 0) regex.lastIndex++;
    }
  }

  state.findMatches = results;
  state.findIndex = results.length > 0 ? 0 : -1;
  setFindCountText(results.length + ' match' + (results.length !== 1 ? 'es' : ''));

  results.forEach((m, i) => {
    highlightMatch(m, i === state.findIndex);
  });
}

/**
 * FS-14: Get the set of node IDs between two nodes (inclusive), in document order.
 */
function getNodeIdsBetween(startNodeId, endNodeId) {
  const page = $('pageContainer');
  if (!page) return new Set();
  const all = page.querySelectorAll('[data-node-id]');
  const ids = new Set();
  let inside = false;
  for (const el of all) {
    const nid = el.dataset.nodeId;
    if (nid === startNodeId || nid === endNodeId) {
      ids.add(nid);
      if (inside || startNodeId === endNodeId) { inside = false; break; }
      inside = true;
      continue;
    }
    if (inside) ids.add(nid);
  }
  return ids;
}

function domFind(query) {
  const page = $('pageContainer');
  const text = page.textContent || '';
  const searchIn = _matchCase ? text : text.toLowerCase();
  const q = _matchCase ? query : query.toLowerCase();
  let count = 0, idx = 0;
  while ((idx = searchIn.indexOf(q, idx)) !== -1) {
    if (_wholeWord) {
      const before = idx > 0 ? searchIn[idx - 1] : ' ';
      const after = idx + q.length < searchIn.length ? searchIn[idx + q.length] : ' ';
      if (/\w/.test(before) || /\w/.test(after)) { idx += q.length; continue; }
    }
    count++;
    idx += q.length;
  }
  state.findMatches = [];
  state.findIndex = -1;
  setFindCountText(count + ' match' + (count !== 1 ? 'es' : ''));
}

function highlightMatch(match, active, cssClass) {
  const page = $('pageContainer');
  const el = page.querySelector(`[data-node-id="${match.nodeId}"]`);
  if (!el) return;

  const cls = cssClass || (active ? 'find-highlight active' : 'find-highlight');

  // Walk text nodes to find the match offset
  const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT, null);
  let counted = 0, node;
  while ((node = walker.nextNode())) {
    const chars = Array.from(node.textContent);
    const nodeStart = counted;
    const nodeEnd = counted + chars.length;
    if (match.offset >= nodeStart && match.offset < nodeEnd) {
      const localOffset = match.offset - nodeStart;
      // Convert char offset to string offset (for surrogate pairs)
      let strOff = 0;
      for (let i = 0; i < localOffset && i < chars.length; i++) strOff += chars[i].length;
      let endStrOff = strOff;
      for (let i = localOffset; i < localOffset + match.length && i < chars.length; i++) endStrOff += chars[i].length;

      try {
        const range = document.createRange();
        range.setStart(node, strOff);
        range.setEnd(node, Math.min(endStrOff, node.textContent.length));
        const span = document.createElement('mark');
        span.className = cls;
        // Use extractContents instead of surroundContents to handle cross-element ranges safely
        span.appendChild(range.extractContents());
        range.insertNode(span);
        if (active) span.scrollIntoView({ block: 'center', behavior: 'smooth' });
      } catch (_) {}
      return;
    }
    counted = nodeEnd;
  }
}

function clearHighlights() {
  const container = $('pageContainer');
  if (!container) return;
  container.querySelectorAll('mark.find-highlight, mark.replace-preview').forEach(m => {
    const parent = m.parentNode;
    while (m.firstChild) parent.insertBefore(m.firstChild, m);
    m.remove();
    parent.normalize();
  });
}

/**
 * FS-14: Show replace preview — highlight the current match with a different color
 * to indicate what will be replaced.
 */
function showReplacePreview() {
  clearReplacePreview();
  if (state.findIndex < 0 || !state.findMatches.length) return;
  const replacement = $('replaceInput')?.value;
  if (replacement === undefined || replacement === null) return;
  // Only show preview when the replace input is focused or has content
  if (!$('replaceInput').value && document.activeElement !== $('replaceInput')) return;

  const match = state.findMatches[state.findIndex];
  // The active match is already highlighted — restyle it as replace-preview
  const container = $('pageContainer');
  if (!container) return;
  const activeMark = container.querySelector('mark.find-highlight.active');
  if (activeMark) {
    activeMark.classList.add('replace-preview');
  }
}

function clearReplacePreview() {
  const container = $('pageContainer');
  if (!container) return;
  container.querySelectorAll('mark.replace-preview').forEach(m => {
    m.classList.remove('replace-preview');
  });
}

function navigateMatch(dir) {
  if (state.findMatches.length === 0) return;
  clearHighlights();
  state.findIndex = (state.findIndex + dir + state.findMatches.length) % state.findMatches.length;
  state.findMatches.forEach((m, i) => highlightMatch(m, i === state.findIndex));
  setFindCountText((state.findIndex + 1) + '/' + state.findMatches.length);
  showReplacePreview();
}

function doReplace() {
  if (!state.doc || state.findIndex < 0) return;
  const match = state.findMatches[state.findIndex];
  const replacement = $('replaceInput').value;
  syncAllText();
  try {
    state.doc.replace_text(match.nodeId, match.offset, match.length, replacement);
    broadcastOp({ action: 'replaceText', nodeId: match.nodeId, offset: match.offset, length: match.length, replacement });
    renderDocument();
    updateUndoRedo();
    doFind(); // re-search
  } catch (e) { console.error('replace:', e); }
}

function doReplaceAll() {
  if (!state.doc) return;
  const query = $('findInput').value;
  const replacement = $('replaceInput').value;
  if (!query) return;
  syncAllText();
  try {
    recordUndoAction('Replace all');
    const count = state.doc.replace_all(query, replacement, _matchCase);
    broadcastOp({ action: 'replaceAll', query, replacement, caseSensitive: _matchCase });
    renderDocument();
    updateUndoRedo();
    setFindCountText(count + ' replaced');
    state.findMatches = [];
    state.findIndex = -1;
  } catch (e) { console.error('replace all:', e); }
}

/**
 * ED2-22/ED2-29: Close the find bar and clean up debounce timer.
 * Exported so other modules (file.js, input.js) can close the bar properly.
 */
export function closeFindBar() {
  clearTimeout(_findRefreshTimer);
  _findRefreshTimer = null;
  const bar = $('findBar');
  if (bar) bar.classList.remove('show');
  clearHighlights();
  // FS-14: Reset find-in-selection when closing
  _findInSelection = false;
  _selectionRange = null;
  if ($('findInSelection')) $('findInSelection').classList.remove('active');
  setRegexError(null);
}

/**
 * E1.5: Re-run find if the find bar is open.
 * Debounced to 300ms so rapid typing doesn't cause perf issues.
 */
export function refreshFindIfOpen() {
  if (!$('findBar').classList.contains('show')) return;
  if (!$('findInput').value) return;
  clearTimeout(_findRefreshTimer);
  _findRefreshTimer = setTimeout(() => {
    // Remember the previous match position to stay near it
    const prevMatch = state.findIndex >= 0 && state.findMatches[state.findIndex]
      ? state.findMatches[state.findIndex] : null;
    doFind();
    // Try to restore closest match index
    if (prevMatch && state.findMatches.length > 0) {
      let best = 0, bestDist = Infinity;
      state.findMatches.forEach((m, i) => {
        const dist = m.nodeId === prevMatch.nodeId
          ? Math.abs(m.offset - prevMatch.offset)
          : Infinity;
        if (dist < bestDist) { bestDist = dist; best = i; }
      });
      if (best !== state.findIndex) {
        clearHighlights();
        state.findIndex = best;
        state.findMatches.forEach((m, i) => highlightMatch(m, i === state.findIndex));
        setFindCountText((state.findIndex + 1) + '/' + state.findMatches.length);
      }
    }
  }, 300);
}
