// Rudra Office — AI Inline Suggestion Engine
// Inline prompt at cursor, streaming diff rendering, accept/reject via document ops.

import { state, $ } from './state.js';
import { aiComplete, abortAI } from './ai.js';
import { detectContext, buildContextPrompt } from './ai-panel.js';

let _promptEl, _promptInput, _suggestionEl, _diffEl;
let _lastAction = null;
let _lastPrompt = '';
let _diffRAF = null;
let _lastRequestTime = 0; // rate limiting cooldown (Enhancement 39)

// ── Word-level diff ─────────────────────────────────

function computeWordDiff(original, suggested) {
  const origWords = original.split(/(\s+)/);
  const sugWords = suggested.split(/(\s+)/);

  // Guard: fall back to simple before/after for very large texts
  if (origWords.length * sugWords.length > 100000) {
    return [
      { type: 'del', text: original },
      { type: 'ins', text: suggested }
    ];
  }

  // Simple LCS-based diff on words
  const m = origWords.length, n = sugWords.length;
  const dp = Array.from({ length: m + 1 }, () => new Uint32Array(n + 1));

  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      if (origWords[i - 1] === sugWords[j - 1]) {
        dp[i][j] = dp[i - 1][j - 1] + 1;
      } else {
        dp[i][j] = Math.max(dp[i - 1][j], dp[i][j - 1]);
      }
    }
  }

  // Backtrack to build diff
  let i = m, j = n;
  const ops = [];
  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && origWords[i - 1] === sugWords[j - 1]) {
      ops.push({ type: 'keep', text: origWords[i - 1] });
      i--; j--;
    } else if (j > 0 && (i === 0 || dp[i][j - 1] >= dp[i - 1][j])) {
      ops.push({ type: 'ins', text: sugWords[j - 1] });
      j--;
    } else {
      ops.push({ type: 'del', text: origWords[i - 1] });
      i--;
    }
  }
  ops.reverse();
  return ops;
}

function renderDiffHTML(original, suggested) {
  if (!original && suggested) {
    return `<span class="ai-diff-ins">${escapeHTML(suggested)}</span>`;
  }
  const ops = computeWordDiff(original, suggested);
  return ops.map(op => {
    const text = escapeHTML(op.text);
    if (op.type === 'del') return `<span class="ai-diff-del">${text}</span>`;
    if (op.type === 'ins') return `<span class="ai-diff-ins">${text}</span>`;
    return text;
  }).join('');
}

function escapeHTML(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

// ── Inline AI Prompt (at cursor) ────────────────────

export function showInlineAIPrompt() {
  if (!state.aiAvailable) return;
  if (state.readOnlyMode) return;

  _promptEl = $('aiInlinePrompt');
  _promptInput = $('aiInlinePromptInput');
  if (!_promptEl || !_promptInput) return;

  // Position at cursor using fixed positioning with viewport coords (Bug 3 fix)
  const sel = window.getSelection();
  if (sel && sel.rangeCount > 0) {
    const range = sel.getRangeAt(0);
    const rect = range.getBoundingClientRect();
    let top = rect.bottom + 4;
    // Clamp to viewport — if prompt would go below visible area, show above cursor
    if (top + 80 > window.innerHeight) top = Math.max(8, rect.top - 80);
    _promptEl.style.top = top + 'px';
    _promptEl.style.left = Math.max(8, Math.min(rect.left, window.innerWidth - 300)) + 'px';
  }

  _promptEl.style.display = 'block';
  _promptInput.value = '';
  _promptInput.focus();
}

export function hideInlinePrompt() {
  if (_promptEl) _promptEl.style.display = 'none';
}

// ── Inline Suggestion (diff view) ───────────────────

function positionSuggestionNearNode(nodeId) {
  _suggestionEl = $('aiInlineSuggestion');
  if (!_suggestionEl) return;

  const canvas = $('editorCanvas');
  if (!canvas) return;

  // Position near the target node using fixed positioning with viewport coords (Bug 3 fix)
  const nodeEl = canvas.querySelector(`[data-node-id="${nodeId}"]`);
  if (nodeEl) {
    const rect = nodeEl.getBoundingClientRect();
    let top = rect.bottom + 4;
    if (top + 200 > window.innerHeight) top = Math.max(8, rect.top - 200);
    _suggestionEl.style.top = top + 'px';
    _suggestionEl.style.left = Math.max(8, Math.min(rect.left, window.innerWidth - 340)) + 'px';
  }
}

export function showInlineSuggestion(nodeId, original, suggested, action, startOffset, multiNodeInfo) {
  _suggestionEl = $('aiInlineSuggestion');
  _diffEl = $('aiInlineDiff');
  if (!_suggestionEl || !_diffEl) return;

  state.aiInlineSuggestion = { nodeId, original, suggested, action, startOffset: startOffset ?? 0, multiNodeInfo: multiNodeInfo || null };

  // Show action label (Bug 59) + character count delta (Enhancement 90)
  const actionLabels = { improve: 'Improve', shorter: 'Shorten', longer: 'Expand', grammar: 'Grammar', translate: 'Translate', custom: 'Custom' };
  const label = actionLabels[action] || action;
  const delta = suggested.length - original.length;
  const deltaStr = delta > 0 ? `+${delta}` : String(delta);
  const deltaColor = delta > 0 ? 'var(--success, #188038)' : delta < 0 ? 'var(--danger, #d93025)' : 'var(--text-muted)';
  _diffEl.innerHTML = `<div style="font-size:11px;color:var(--text-muted);margin-bottom:4px;font-weight:500;">AI: ${escapeHTML(label)} <span style="color:${deltaColor};font-weight:400">(${deltaStr} chars)</span></div>` + renderDiffHTML(original, suggested);

  positionSuggestionNearNode(nodeId);
  _suggestionEl.style.display = 'block';
  // Ensure suggestion is visible in viewport
  requestAnimationFrame(() => {
    _suggestionEl.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
  });
}

export function hideInlineSuggestion() {
  if (_suggestionEl) _suggestionEl.style.display = 'none';
  state.aiInlineSuggestion = null;
}

// ── Table Detection & Insert (Enhancement 71) ──────

function tryInsertAsTable(nodeId, aiText) {
  // Detect markdown table format: lines with | separators
  const lines = aiText.trim().split('\n').filter(l => l.includes('|'));
  if (lines.length < 2) return false; // Not a table

  // Skip separator lines (--- | ---)
  const dataLines = lines.filter(l => !l.match(/^\s*\|?\s*[-:]+\s*(\|\s*[-:]+\s*)*\|?\s*$/));
  if (dataLines.length < 1) return false;

  // Parse cells
  const rows = dataLines.map(l =>
    l.split('|').map(c => c.trim()).filter(c => c !== '')
  );
  if (rows.length === 0 || rows[0].length === 0) return false;

  const numCols = Math.max(...rows.map(r => r.length));
  const numRows = rows.length;

  const doc = state.doc;
  if (!doc) return false;

  try {
    // Insert table via WASM
    doc.insert_table(nodeId, numRows, numCols);

    // Trigger re-render so the table appears in DOM
    const editorCanvas = $('editorCanvas');
    if (editorCanvas) editorCanvas.dispatchEvent(new Event('input', { bubbles: true }));

    state.dirty = true;
    state.undoHistory = state.undoHistory || [];
    state.undoHistory.unshift({ label: 'AI table inserted', timestamp: Date.now() });
    if (state.undoHistory.length > 50) state.undoHistory.length = 50;

    return true;
  } catch (e) {
    console.warn('[ai-inline] Table insert failed:', e);
    return false;
  }
}

// ── Core AI Action Trigger ──────────────────────────

export async function triggerAIAction(action, customPrompt) {
  if (state.aiGenerating) return;
  if (!state.aiAvailable) return;
  // Rate limiting: 1 second cooldown between requests (Enhancement 39)
  if (Date.now() - _lastRequestTime < 1000) return;

  const sel = window.getSelection();
  let selectedText = sel ? sel.toString().trim() : '';
  let targetNodeId = null;

  // Find the target paragraph node
  if (sel && sel.rangeCount > 0) {
    let node = sel.anchorNode;
    while (node && node !== $('editorCanvas')) {
      if (node.nodeType === 1 && node.dataset?.nodeId) {
        targetNodeId = node.dataset.nodeId;
        break;
      }
      node = node.parentNode;
    }
  }

  // Detect multi-paragraph selection (Bug #35)
  let multiNodeInfo = null;
  if (sel && sel.rangeCount > 0 && targetNodeId) {
    const range = sel.getRangeAt(0);
    const canvas = $('editorCanvas');
    if (canvas) {
      const allParas = canvas.querySelectorAll('[data-node-id]');
      const selectedParas = [];
      for (const para of allParas) {
        if (range.intersectsNode(para)) {
          selectedParas.push({
            nodeId: para.dataset.nodeId,
            text: para.textContent || ''
          });
        }
      }
      if (selectedParas.length > 1) {
        multiNodeInfo = selectedParas;
      }
    }
  }

  // If no selection, use the entire paragraph text
  if (!selectedText && targetNodeId) {
    const el = $('editorCanvas')?.querySelector(`[data-node-id="${targetNodeId}"]`);
    if (el) selectedText = el.textContent?.trim() || '';
  }

  if (!selectedText) return;

  // Capture the offset of the selected text within the paragraph.
  // Uses the browser Selection API to compute the actual character offset,
  // avoiding indexOf failures with duplicate text (Bug 44 fix).
  let selStartOffset = 0;
  if (sel && sel.rangeCount > 0 && targetNodeId) {
    try {
      const range = sel.getRangeAt(0);
      const paraEl = $('editorCanvas')?.querySelector(`[data-node-id="${targetNodeId}"]`);
      if (paraEl) {
        const preRange = document.createRange();
        preRange.setStart(paraEl, 0);
        preRange.setEnd(range.startContainer, range.startOffset);
        selStartOffset = preRange.toString().length;
      }
    } catch (_) {
      // Fallback to indexOf
      const el = $('editorCanvas')?.querySelector(`[data-node-id="${targetNodeId}"]`);
      const paraText = el?.textContent || '';
      const idx = paraText.indexOf(selectedText);
      selStartOffset = idx >= 0 ? idx : 0;
    }
  }

  const context = detectContext();
  const actionPrompts = {
    improve: `Improve this text for clarity, flow, and professionalism. Return only the improved text:\n\n${selectedText}`,
    shorter: `Make this more concise while keeping the key points. Return only the shortened text:\n\n${selectedText}`,
    longer: `Expand on this with more detail and supporting points. Return only the expanded text:\n\n${selectedText}`,
    grammar: `Fix any grammar, spelling, and punctuation issues. Return only the corrected text:\n\n${selectedText}`,
    translate: `Detect the language of this text. If it's English, translate to Spanish. If it's any other language, translate to English. Return only the translated text, nothing else:\n\n${selectedText}`,
    custom: customPrompt ? `${customPrompt}\n\nText:\n${selectedText}` : selectedText,
  };

  const prompt = actionPrompts[action] || actionPrompts.custom;
  const mode = action === 'grammar' ? 'grammar' : 'writer';
  const systemPrompt = buildContextPrompt(mode, context);

  _lastAction = action;
  _lastPrompt = customPrompt || '';

  state.aiGenerating = true;

  // Track AI action usage (Enhancement 88)
  import('./analytics.js').then(m => m.trackEvent('ai_action', action)).catch(() => {});

  // Show loading indicator
  _suggestionEl = $('aiInlineSuggestion');
  _diffEl = $('aiInlineDiff');
  if (_suggestionEl && _diffEl) {
    _diffEl.innerHTML = '<span class="ai-inline-loading">AI is thinking...</span>';
    // Position near the target node
    if (targetNodeId) positionSuggestionNearNode(targetNodeId);
    _suggestionEl.style.display = 'block';
  }

  try {
    let fullText = '';

    const result = await aiComplete(mode, prompt, {
      systemPrompt,
      context: state.aiConversation.slice(-12),
      noAutoAbort: true,
      onChunk(chunk) {
        fullText += chunk;
        if (targetNodeId && _diffEl && !_diffRAF) {
          _diffRAF = requestAnimationFrame(() => {
            _diffEl.innerHTML = renderDiffHTML(selectedText, fullText.trim()) + '<span class="ai-stream-cursor"></span>';
            if (_suggestionEl) _suggestionEl.style.display = 'block';
            _diffRAF = null;
          });
        }
      }
    });

    fullText = (fullText || result).trim();

    if (fullText && targetNodeId) {
      showInlineSuggestion(targetNodeId, selectedText, fullText, action, selStartOffset, multiNodeInfo);
    }

    // Add to conversation context
    state.aiConversation.push(
      { role: 'user', content: `[${action}] ${selectedText.slice(0, 150)}` },
      { role: 'assistant', content: fullText.slice(0, 200) }
    );
    if (state.aiConversation.length > 12) {
      state.aiConversation = state.aiConversation.slice(-12);
    }

  } catch (err) {
    if (err.name === 'AbortError') {
      // Check if it was a timeout
      const isTimeout = err.message?.includes('timed out') || (err.cause && String(err.cause).includes('timed out'));
      if (isTimeout) {
        // Show timeout-specific feedback in the suggestion area
        if (_diffEl) _diffEl.innerHTML = '<span class="ai-inline-loading" style="color:var(--danger)">Request timed out. Try a shorter selection.</span>';
        if (_suggestionEl) _suggestionEl.style.display = 'block';
        // Auto-hide after 3 seconds
        setTimeout(() => hideInlineSuggestion(), 3000);
      } else {
        hideInlineSuggestion();
      }
    } else {
      console.error('[ai-inline] Error:', err);
      // Show error in suggestion area
      if (_diffEl) _diffEl.innerHTML = '<span class="ai-inline-loading" style="color:var(--danger)">AI error: ' + escapeHTML(err.message || 'Unknown error') + '</span>';
      if (_suggestionEl) _suggestionEl.style.display = 'block';
      setTimeout(() => hideInlineSuggestion(), 4000);
    }
  } finally {
    state.aiGenerating = false;
    _lastRequestTime = Date.now(); // rate limiting cooldown (Enhancement 39)
  }
}

// ── Accept / Reject ─────────────────────────────────

export function acceptSuggestion() {
  const suggestion = state.aiInlineSuggestion;
  if (!suggestion) return;

  const { nodeId, original, suggested, startOffset, multiNodeInfo } = suggestion;
  const doc = state.doc;
  if (!doc) { hideInlineSuggestion(); return; }

  // Check if AI generated a table — insert as real WASM table (Enhancement 71)
  if (suggestion.action === 'custom' && suggested.includes('|') && suggested.split('\n').filter(l => l.includes('|')).length >= 2) {
    if (tryInsertAsTable(nodeId, suggested)) {
      hideInlineSuggestion();
      return;
    }
  }

  try {
    if (multiNodeInfo && multiNodeInfo.length > 1) {
      // ── Multi-paragraph accept (Bug #35) ──────────────────────
      // Verify paragraphs haven't been modified by collaborators since trigger time (Bug 54)
      let staleDetected = false;
      for (const para of multiNodeInfo) {
        const el = $('editorCanvas')?.querySelector(`[data-node-id="${para.nodeId}"]`);
        const currentText = el?.textContent || '';
        if (currentText !== para.text) {
          staleDetected = true;
          break;
        }
      }
      if (staleDetected) {
        // Show warning in the suggestion area instead of silently applying
        if (_diffEl) {
          _diffEl.innerHTML = '<span class="ai-inline-loading" style="color:var(--danger)">Document was modified while AI was generating. Please retry.</span>';
        }
        if (_suggestionEl) _suggestionEl.style.display = 'block';
        setTimeout(() => hideInlineSuggestion(), 4000);
        return;
      }

      // Split the AI-suggested text into paragraph-sized chunks by newlines.
      // Apply each chunk to its corresponding paragraph via set_paragraph_text.
      const suggestedParas = suggested.split('\n').filter(s => s.trim());

      // Warn if paragraph count doesn't match (Bug 74 fix)
      if (suggestedParas.length !== multiNodeInfo.length) {
        // If AI returned fewer paragraphs, pad with empty strings
        while (suggestedParas.length < multiNodeInfo.length) {
          suggestedParas.push('');
        }
        // If AI returned more, join the extras into the last paragraph
        if (suggestedParas.length > multiNodeInfo.length) {
          const extra = suggestedParas.splice(multiNodeInfo.length);
          suggestedParas[multiNodeInfo.length - 1] += '\n' + extra.join('\n');
        }
      }

      for (let i = 0; i < multiNodeInfo.length; i++) {
        const para = multiNodeInfo[i];
        const newText = i < suggestedParas.length ? suggestedParas[i] : '';
        try {
          // Use replace_text instead of set_paragraph_text to preserve
          // formatting on runs not touched by the replacement (Bug 43 fix).
          const cpLen = Array.from(para.text).length;
          doc.replace_text(para.nodeId, 0, cpLen, newText);
          // DOM will be updated by re-render below
        } catch (e) {
          console.warn('[ai-inline] Multi-para replace error for node', para.nodeId, e);
        }
      }

      // Trigger re-render from WASM model to preserve inline elements
      try {
        const editorCanvas = $('editorCanvas');
        if (editorCanvas) editorCanvas.dispatchEvent(new Event('input', { bubbles: true }));
      } catch (_) {}

      // Mark dirty and record undo
      state.dirty = true;
      state.undoHistory = state.undoHistory || [];
      state.undoHistory.unshift({ label: 'AI suggestion accepted (multi-paragraph)', timestamp: Date.now() });
      if (state.undoHistory.length > 50) state.undoHistory.length = 50;

    } else {
      // ── Single paragraph accept (existing logic) ──────────────
      // Find the DOM element
      const el = $('editorCanvas')?.querySelector(`[data-node-id="${nodeId}"]`);
      if (!el) { hideInlineSuggestion(); return; }

      // Use stored startOffset (captured at action trigger time) to avoid
      // indexOf failures with duplicate text (Bug 2 fix).
      // startOffset is a JS string index (from Selection API / indexOf).
      const jsOffset = startOffset ?? 0;

      // Verify paragraph hasn't been modified since suggestion was generated (I4 collab guard)
      const currentText = el.textContent || '';
      const expectedText = currentText.slice(jsOffset, jsOffset + original.length);
      if (expectedText !== original) {
        if (_diffEl) _diffEl.innerHTML = '<span class="ai-inline-loading" style="color:var(--danger)">Paragraph was modified. Please retry.</span>';
        if (_suggestionEl) _suggestionEl.style.display = 'block';
        setTimeout(() => hideInlineSuggestion(), 3000);
        return;
      }

      // WASM replace_text expects Unicode codepoint offsets, not JS UTF-16 code unit
      // indices. Convert using Array.from which splits by codepoint (Bug 42 fix).
      const fullText = currentText;
      const cpOffset = Array.from(fullText.slice(0, jsOffset)).length;
      const cpLength = Array.from(original).length;

      // Replace via WASM replace_text which preserves formatting of untouched runs (Bug 1 fix).
      // Signature: replace_text(node_id, codepoint_offset, codepoint_length, replacement)
      try {
        doc.replace_text(nodeId, cpOffset, cpLength, suggested);
      } catch (wasmErr) {
        console.warn('[ai-inline] WASM replace_text failed:', wasmErr);
      }

      // Trigger re-render from WASM model to preserve inline elements (images, links, equations)
      try {
        const editorCanvas = $('editorCanvas');
        if (editorCanvas) editorCanvas.dispatchEvent(new Event('input', { bubbles: true }));
      } catch (_) {
        // Fallback: direct textContent update (loses inline formatting elements)
        const before = fullText.slice(0, jsOffset);
        const after = fullText.slice(jsOffset + original.length);
        el.textContent = before + suggested + after;
      }

      // Mark dirty and record undo (Enhancement 34)
      state.dirty = true;
      state.undoHistory = state.undoHistory || [];
      state.undoHistory.unshift({ label: 'AI suggestion accepted', timestamp: Date.now() });
      if (state.undoHistory.length > 50) state.undoHistory.length = 50;

      // Restore cursor to end of replaced text (Enhancement 37)
      try {
        const sel = window.getSelection();
        const range = document.createRange();
        const textNode = el.firstChild;
        if (textNode) {
          const endPos = jsOffset + suggested.length;
          const safePos = Math.min(endPos, textNode.textContent?.length || 0);
          range.setStart(textNode, safePos);
          range.collapse(true);
          sel.removeAllRanges();
          sel.addRange(range);
        }
      } catch (_) { /* cursor restore is non-critical */ }

      // Collaboration: WASM replace_text integrates with the CRDT model,
      // so collab peers will receive the change via the next sync cycle.
    }

  } catch (err) {
    console.error('[ai-inline] Accept error:', err);
  }

  hideInlineSuggestion();
}

export function rejectSuggestion() {
  hideInlineSuggestion();
}

export function retrySuggestion() {
  const suggestion = state.aiInlineSuggestion;
  if (!suggestion) return;

  hideInlineSuggestion();

  // Open inline prompt pre-filled with last prompt for editing (Enhancement 89)
  showInlineAIPrompt();
  if (_promptInput) {
    const actionLabels = { improve: 'Improve this text', shorter: 'Make more concise', longer: 'Expand with detail', grammar: 'Fix grammar', translate: 'Translate', custom: '' };
    const prefill = _lastPrompt || actionLabels[_lastAction] || '';
    _promptInput.value = prefill;
    _promptInput.setSelectionRange(0, prefill.length);
  }
}

// ── Initialization ──────────────────────────────────

export function initAIInline() {
  const cfg = window.S1_CONFIG || {};
  if (!cfg.enableAI) return;

  _promptEl = $('aiInlinePrompt');
  _promptInput = $('aiInlinePromptInput');
  _suggestionEl = $('aiInlineSuggestion');
  _diffEl = $('aiInlineDiff');

  if (!_promptEl) return;

  // Inline prompt submit
  _promptInput?.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      const text = _promptInput.value.trim();
      if (text) {
        hideInlinePrompt();
        triggerAIAction('custom', text);
      }
    }
    if (e.key === 'Escape') {
      e.preventDefault();
      hideInlinePrompt();
    }
  });

  // Suggestion accept/reject buttons
  $('aiInlineAccept')?.addEventListener('click', acceptSuggestion);
  $('aiInlineReject')?.addEventListener('click', rejectSuggestion);
  $('aiInlineRetry')?.addEventListener('click', retrySuggestion);

  // Keyboard: Escape=reject, Ctrl/Cmd+Enter=accept when suggestion is visible
  document.addEventListener('keydown', (e) => {
    if (!state.aiInlineSuggestion) return;
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.isContentEditable) return;

    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
      e.preventDefault();
      e.stopPropagation();
      acceptSuggestion();
    }
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      rejectSuggestion();
    }
  });

  // Click outside inline prompt or suggestion — single combined handler
  document.addEventListener('mousedown', (e) => {
    // Close inline prompt if clicking outside it
    if (_promptEl?.style.display === 'block' && !_promptEl.contains(e.target)) {
      hideInlinePrompt();
    }
    // Reject suggestion if clicking outside it
    if (state.aiInlineSuggestion && _suggestionEl && !_suggestionEl.contains(e.target)) {
      rejectSuggestion();
    }
  });
}
